//! Trusted LOCAL read-only unit-profile preparation. No AWS/wallet calls,
//! native registration, funds key, balance or financial readiness initialization.
use cinder_kernel::{identity::*, ledger::Location};
use cinder_pacifica::{execution as p, profile as pp};
use cinder_service::{
    Error,
    boot::{Manifest, Role},
    units::{deployment as d, worker},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::io::{Read, Write};
use zeroize::{Zeroize, Zeroizing};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    original: Vec<u8>,
    ids: [[u8; 32]; 6],
    bulk_account: [u8; 32],
    bulk_trading: Vec<u8>,
    privacy_nonce: [u8; 32],
    key_arns: Vec<String>,
    bulk_root: Vec<u8>,
    native_reads: bool,
}
impl Drop for Input {
    fn drop(&mut self) {
        self.original.zeroize();
        self.bulk_trading.zeroize();
        self.privacy_nonce.zeroize();
    }
}
fn run() -> Result<(), Error> {
    if std::env::args().len() != 1 {
        return Err(Error);
    }
    let mut bytes = Zeroizing::new(Vec::new());
    std::io::stdin().take(131073).read_to_end(&mut bytes)?;
    if bytes.len() > 131072 {
        return Err(Error);
    }
    let input: Input = serde_json::from_slice(&bytes).map_err(|_| Error)?;
    if input.original.len() > 65536 || input.key_arns.len() != 7 || input.bulk_trading.len() != 32 {
        return Err(Error);
    }
    let mut value: Value = serde_cbor::from_slice(&input.original).map_err(|_| Error)?;
    let mut manifest: Manifest =
        serde_json::from_value(value["manifest"].take()).map_err(|_| Error)?;
    if manifest.version != 1 {
        return Err(Error);
    }
    let old: cinder_service::runtime::Configuration =
        serde_json::from_value(value["configuration"].take()).map_err(|_| Error)?;
    if old.owners.len() != 1 || old.source != 0 || old.execution.origin != p::Origin::Testnet {
        return Err(Error);
    }
    let owner = &old.owners[0];
    let mut ledger = cinder_journal::wire::decode_config(&old.ledger).map_err(|_| Error)?;
    ledger.customers.clear();
    if ledger.sources.len() != 3 || ledger.sources[0].location != Location::Venue {
        return Err(Error);
    }
    let mut bulk_ledger = ledger.clone();
    bulk_ledger.venue = VenueId::new(input.ids[2]).map_err(|_| Error)?;
    bulk_ledger.venue_account = VenueAccountId::new(input.ids[3]).map_err(|_| Error)?;
    for source in &mut bulk_ledger.sources {
        source.scope.venue = bulk_ledger.venue;
        source.scope.account = bulk_ledger.venue_account;
    }
    let unit = |ledger: &cinder_kernel::ledger::Config,
                account: [u8; 32],
                id: [u8; 32]|
     -> Result<d::Unit, Error> {
        Ok(d::Unit {
            ledger: cinder_journal::wire::encode_config(ledger).map_err(|_| Error)?,
            native_account: account,
            funding_wallet: account,
            custody_config: id,
            payout_vault: id,
            key_policy: id,
        })
    };
    let evidence =
        "P40 empty-account read-only transport qualification; no financial interpretation";
    let pacifica = d::Pacifica {
        unit: unit(&ledger, old.route.broker, input.ids[0])?,
        source: 0,
        revision: 1,
        evidence: evidence.into(),
        precision: pp::Level::Qualified,
        markets: old.markets,
        quote_places: old.quote_places,
        perp_tag: old.perp_tag,
        execution: p::Policy {
            revision: 1,
            evidence: evidence.into(),
            execution: pp::Level::Unknown,
            origin: p::Origin::Testnet,
            expiry_ms: 10000,
            credits: 1000,
            cleanup_reserve: 120,
            read_cost: 120,
        },
        trading_epoch: 1,
        fills: Some(pp::Level::Unknown),
    };
    let bulk = d::Bulk {
        unit: unit(&bulk_ledger, input.bulk_account, input.ids[1])?,
        source: 0,
        revision: 1,
        evidence: input.ids[4],
        quote_places: 0,
        markets: vec![d::BulkMarket {
            symbol: "BTC-USD".into(),
            market: 0,
            size_step: "1".into(),
            price_step: "1".into(),
        }],
        expiry_ms: 10000,
        credits: 20,
        cleanup_reserve: 2,
        trading_epoch: 1,
        qualification: Some(cinder_bulk::execution::Level::Unknown),
    };
    manifest.version = 8;
    manifest.application = [1; 32];
    manifest.gates.trading = false;
    manifest.gates.funding = false;
    manifest.gates.native_reads = input.native_reads;
    manifest.chain = None;
    manifest.native_capture = None;
    manifest.demo_deposit = None;
    manifest.history = Some(cinder_service::boot::HistoryPolicy {
        record_bytes: 65536,
        history_bytes: 8388608,
        records: 256,
        put_requests: 1024,
        put_bytes: 134217728,
    });
    manifest.units = Some(d::Policy {
        registry: [1; 32],
        pacifica: input.ids[0],
        bulk: input.ids[1],
        bulk_port: 9010,
        bulk_root: input.bulk_root.clone(),
        bulk_root_hash: openssl::sha::sha256(&input.bulk_root),
        worker: Some(worker::Policy {
            poll_ms: 1000,
            pacifica_reads: u16::from(input.native_reads),
            bulk_reads: u16::from(input.native_reads),
            pacifica_dispatches: 0,
            bulk_dispatches: 0,
        }),
    });
    let endpoint = manifest.slots.first().ok_or(Error)?.endpoint.clone();
    manifest.slots = [
        Role::Configuration,
        Role::Storage,
        Role::Trading,
        Role::BulkTrading,
        Role::PacificaConfiguration,
        Role::BulkConfiguration,
        Role::Witness,
    ]
    .into_iter()
    .zip(input.key_arns.iter())
    .map(|(role, arn)| {
        let mut endpoint = endpoint.clone();
        endpoint.resource = arn.clone();
        cinder_service::boot::Slot {
            role,
            endpoint,
            plaintext_hash: [1; 32],
        }
    })
    .collect();
    manifest.validate()?;
    let configuration = d::Configuration {
        privacy_nonce: input.privacy_nonce,
        owners: vec![d::Owner {
            wallet: owner.wallet,
            pacifica_tokens: owner.tokens,
            bulk_tokens: input.ids[5],
        }],
        auth_ms: 30000,
        grant_ms: 600000,
    };
    let output = json!({"manifest":manifest,"configuration":configuration,"pacifica_configuration":pacifica,"bulk_configuration":bulk,
        "storage":value["storage"],"trading":value["trading"],"bulk_trading":input.bulk_trading,"witness":value["witness"]});
    let output = Zeroizing::new(serde_cbor::to_vec(&output).map_err(|_| Error)?);
    if output.len() > 65536 {
        return Err(Error);
    }
    std::io::stdout().write_all(&output)?;
    Ok(())
}
fn main() {
    if run().is_err() {
        eprintln!("unit qualification preparation refused");
        std::process::exit(1);
    }
}
