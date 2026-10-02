//! Trusted LOCAL disposable hardware-profile preparation. No AWS/wallet calls.
//! No funds, real market qualification or native authority is introduced. The
//! synthetic adapter fields initialize inactive controllers; all gates are false.
use cinder_kernel::{amounts::*, identity::*, ledger::*, position::Market};
use cinder_pacifica::{execution::*, funding::*, profile::*};
use cinder_service::{
    Error,
    boot::*,
    cloud::*,
    runtime::{Configuration, Owner},
};
use serde_json::{Value, json};
use std::io::{Read, Write};

fn bytes(v: &Value) -> Result<[u8; 32], Error> {
    serde_json::from_value(v.clone()).map_err(|_| Error)
}
fn run() -> Result<(), Error> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() == 2 && args[0] == "renew-witness" {
        let root = std::path::Path::new(&args[1]);
        let manifest_bytes = std::fs::read(root.join("manifest.cbor"))?;
        if manifest_bytes.len() > 131072 {
            return Err(Error);
        }
        let manifest: Manifest = serde_cbor::from_slice(&manifest_bytes).map_err(|_| Error)?;
        manifest.validate()?;
        let body = |role: Role| -> Result<zeroize::Zeroizing<Vec<u8>>, Error> {
            let wrapped = zeroize::Zeroizing::new(std::fs::read(
                root.join(format!("{}.plain", role.name())),
            )?);
            if wrapped.len() > 4096 {
                return Err(Error);
            }
            let prefix = manifest.wrap(role, &[])?;
            let body = wrapped.strip_prefix(prefix.as_slice()).ok_or(Error)?;
            let slot = manifest
                .slots
                .iter()
                .find(|s| s.role == role)
                .ok_or(Error)?;
            if body.is_empty() || openssl::sha::sha256(body) != slot.plaintext_hash {
                return Err(Error);
            }
            Ok(zeroize::Zeroizing::new(body.to_vec()))
        };
        let configuration: Configuration =
            serde_cbor::from_slice(&body(Role::Configuration)?).map_err(|_| Error)?;
        let mut incoming = zeroize::Zeroizing::new(Vec::new());
        std::io::stdin().take(16385).read_to_end(&mut incoming)?;
        if incoming.len() > 16384 {
            return Err(Error);
        }
        let witness: Credential = serde_json::from_slice(&incoming).map_err(|_| Error)?;
        let output = zeroize::Zeroizing::new(
            serde_cbor::to_vec(&json!({
                "manifest":manifest,"configuration":configuration,
                "storage":body(Role::Storage)?.as_slice(),
                "trading":body(Role::Trading)?.as_slice(),
                "broker":body(Role::Broker)?.as_slice(),"witness":witness
            }))
            .map_err(|_| Error)?,
        );
        if output.len() > 65536 {
            return Err(Error);
        }
        std::io::stdout().write_all(&output)?;
        return Ok(());
    }
    if args.as_slice() == ["bootstrap"] {
        let mut bytes = zeroize::Zeroizing::new(Vec::new());
        std::io::stdin().take(65537).read_to_end(&mut bytes)?;
        if bytes.len() > 65536 {
            return Err(Error);
        }
        let boot: Bootstrap = serde_json::from_slice(&bytes).map_err(|_| Error)?;
        if boot.capsules.len() != 5
            || boot
                .capsules
                .iter()
                .any(|c| c.ciphertext.is_empty() || c.ciphertext.len() > 6144)
        {
            return Err(Error);
        }
        let out = zeroize::Zeroizing::new(serde_cbor::to_vec(&boot).map_err(|_| Error)?);
        if out.len() > 65536 {
            return Err(Error);
        }
        std::io::stdout().write_all(&out)?;
        return Ok(());
    }
    if args.len() == 2 && args[0] == "inspect" {
        let b = std::fs::read(&args[1])?;
        if b.len() > 131072 {
            return Err(Error);
        }
        let m: Manifest = serde_cbor::from_slice(&b).map_err(|_| Error)?;
        m.validate()?;
        let out = json!({"manifest":m,"digest":m.digest()?,"streamKey":m.domain.iter().chain(m.stream.iter()).map(|b|format!("{b:02x}")).collect::<String>()});
        std::io::stdout().write_all(&serde_json::to_vec(&out).map_err(|_| Error)?)?;
        return Ok(());
    }
    if !args.is_empty() {
        return Err(Error);
    }
    let mut input = zeroize::Zeroizing::new(Vec::new());
    std::io::stdin().take(65537).read_to_end(&mut input)?;
    if input.len() > 65536 {
        return Err(Error);
    }
    let v: Value = serde_json::from_slice(&input).map_err(|_| Error)?;
    let ids = v["ids"].as_array().ok_or(Error)?;
    if ids.len() != 24 {
        return Err(Error);
    }
    let id = |n: usize| bytes(&ids[n]);
    let domain = Domain {
        network: NetworkId::new(id(0)?).map_err(|_| Error)?,
        deployment: DeploymentId::new(id(1)?).map_err(|_| Error)?,
    };
    let quote = AssetUnit {
        asset: AssetId::new(id(2)?).map_err(|_| Error)?,
        precision: PrecisionVersion::new(1).map_err(|_| Error)?,
    };
    let venue = VenueId::new(id(3)?).map_err(|_| Error)?;
    let venue_account = VenueAccountId::new(id(4)?).map_err(|_| Error)?;
    let customer = AccountId::new(id(7)?).map_err(|_| Error)?;
    let config = Config {
        domain,
        quote,
        venue,
        venue_account,
        policy: PolicyVersion::new(1).map_err(|_| Error)?,
        sources: [
            (5, Location::Venue),
            (8, Location::Vault),
            (9, Location::Broker),
        ]
        .into_iter()
        .map(|(n, location)| {
            Ok(Source {
                scope: EventScope {
                    domain,
                    venue,
                    account: venue_account,
                    namespace: NamespaceId::new(id(n)?).map_err(|_| Error)?,
                },
                location,
            })
        })
        .collect::<Result<_, Error>>()?,
        markets: vec![
            Market::new(
                MarketUnit {
                    market: MarketId::new(id(6)?).map_err(|_| Error)?,
                    precision: quote.precision,
                    quote,
                },
                1,
                1,
            )
            .map_err(|_| Error)?,
        ],
        customers: vec![customer],
    };
    let evidence = "P20 disposable synthetic inactive adapter profile; NOT venue qualification";
    let owner = Owner {
        account: customer.bytes(),
        wallet: bytes(&v["ownerPublic"])?,
        tokens: id(10)?,
    };
    let route = Route {
        domain: domain.deployment.bytes(),
        pool: id(11)?,
        funds: bytes(&v["fundsPublic"])?,
        beneficiaries: vec![Beneficiary {
            account: owner.account,
            wallet: owner.wallet,
            tokens: owner.tokens,
        }],
        program: id(12)?,
        config: id(13)?,
        vault: id(14)?,
        mint: id(15)?,
        broker: bytes(&v["brokerPublic"])?,
        broker_tokens: id(16)?,
        venue_program: id(17)?,
        venue_vault: id(18)?,
        epoch: 1,
        decimals: 0,
        withdrawal: Level::Qualified,
        chain: Level::Qualified,
        settings: Level::Qualified,
        withdrawal_cost: 10,
        maximum_movement: 1000,
        maximum_fee: 5,
        setup_max_age: 10000,
    };
    let configuration = Configuration {
        ledger: cinder_journal::wire::encode_config(&config).map_err(|_| Error)?,
        owners: vec![owner],
        auth_ms: 30000,
        grant_ms: 600000,
        source: 0,
        account: v["brokerAddress"].as_str().ok_or(Error)?.into(),
        environment: Origin::Testnet.url().into(),
        revision: 1,
        evidence: evidence.into(),
        precision: Level::Qualified,
        fills: Level::Qualified,
        markets: vec![Mapping {
            symbol: "BTC".into(),
            market: 0,
            size: Grid { places: 0, step: 1 },
            price: Grid { places: 0, step: 1 },
        }],
        quote_places: 0,
        perp_tag: 0,
        execution: Policy {
            revision: 1,
            evidence: evidence.into(),
            execution: Level::Qualified,
            origin: Origin::Testnet,
            expiry_ms: 3000,
            credits: 100,
            cleanup_reserve: 20,
            read_cost: 10,
        },
        route,
        trading_epoch: 1,
    };
    let aws_root: Vec<u8> = serde_json::from_value(v["awsRoot"].clone()).map_err(|_| Error)?;
    let venue_root: Vec<u8> = serde_json::from_value(v["venueRoot"].clone()).map_err(|_| Error)?;
    let endpoint = |service: &str, resource: String, port| Endpoint {
        service: service.into(),
        region: "us-east-1".into(),
        resource,
        port,
        root_hash: openssl::sha::sha256(&aws_root),
        root: aws_root.clone(),
    };
    let resources = &v["resources"];
    let keys = resources["keyArns"].as_array().ok_or(Error)?;
    if keys.len() != 5 {
        return Err(Error);
    }
    let manifest = Manifest {
        version: 1,
        domain: [domain.network.bytes(), domain.deployment.bytes()].concat(),
        application: [1; 32],
        stream: id(19)?,
        generation: 1,
        epoch: 1,
        ingress: 9001,
        bootstrap: 9002,
        venue_port: 9007,
        venue_root_hash: openssl::sha::sha256(&venue_root),
        venue_root,
        first: endpoint(
            "s3",
            resources["buckets"][0].as_str().ok_or(Error)?.into(),
            9003,
        ),
        second: endpoint(
            "s3",
            resources["buckets"][1].as_str().ok_or(Error)?.into(),
            9004,
        ),
        witness: endpoint(
            "dynamodb",
            resources["table"].as_str().ok_or(Error)?.into(),
            9005,
        ),
        slots: [
            Role::Configuration,
            Role::Storage,
            Role::Trading,
            Role::Broker,
            Role::Witness,
        ]
        .into_iter()
        .enumerate()
        .map(|(i, role)| {
            Ok(Slot {
                role,
                endpoint: endpoint("kms", keys[i].as_str().ok_or(Error)?.into(), 9006),
                plaintext_hash: [1; 32],
            })
        })
        .collect::<Result<_, Error>>()?,
        gates: Gates {
            trading: false,
            funding: false,
            native_reads: false,
            maximum_boot_ms: 2400000,
        },
    };
    let prepare = json!({"manifest":manifest,"configuration":configuration,"storage":v["storage"],"trading":v["trading"],"broker":v["broker"],"witness":v["witness"]});
    let output = zeroize::Zeroizing::new(serde_cbor::to_vec(&prepare).map_err(|_| Error)?);
    if output.len() > 65536 {
        return Err(Error);
    }
    std::io::stdout().write_all(&output)?;
    Ok(())
}
fn main() {
    if run().is_err() {
        eprintln!("qualification preparation refused");
        std::process::exit(1);
    }
}
