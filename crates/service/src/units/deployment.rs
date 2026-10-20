//! Measured two-unit assembly, not the local fixture or native activation.
//! All three private configurations arrive as separate bounded KMS capsules;
//! no private ledger/owner inventory is put in the image or parent filesystem.
use crate::{
    Error,
    boot::{Manifest, Role},
    cloud::Credential,
    egress::{BulkEgress, Egress, Trust},
    runtime::Storage,
    transport::{Clock, Handler},
    vsock::Target,
};
use cinder_api::units::{Authorizer, service::Service};
use cinder_bulk::{execution as bulk, financial as bf};
use cinder_journal::{
    Backend,
    encrypted::RecordCipher,
    model::CommitId,
    read::{Binding, Witness},
    replicated::Stream,
    units,
};
use cinder_kernel::identity::*;
use cinder_pacifica::{execution as pacifica, profile as pp};
use openssl::sha::sha256;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use zeroize::Zeroizing;

/// Public route/registry contract. Fixed testnet origins live in the real egress
/// implementations; a host cannot supply an arbitrary URL or default unit.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    /// Canonical immutable private descriptor commitment.
    pub registry: [u8; 32],
    /// Exact Pacifica unit.
    pub pacifica: [u8; 32],
    /// Exact BULK unit, distinct from Pacifica.
    pub bulk: [u8; 32],
    /// Separate opaque parent tunnel for BULK testnet HTTPS.
    pub bulk_port: u32,
    /// Approved canonical DER trust anchor for BULK.
    pub bulk_root: Vec<u8>,
    /// Independently approved anchor digest.
    pub bulk_root_hash: [u8; 32],
}
impl Policy {
    /// Pure closed contract validation; no network I/O or activation.
    pub fn validate(&self) -> Result<(), Error> {
        if self.registry == [0; 32] || self.pacifica == self.bulk {
            return Err(Error);
        }
        UnitId::new(self.pacifica).map_err(|_| Error)?;
        UnitId::new(self.bulk).map_err(|_| Error)?;
        Target::new(3, self.bulk_port)?;
        Trust::from_der(&self.bulk_root, self.bulk_root_hash)?;
        Ok(())
    }
}
/// A private, explicit managed owner. No initial credit or native account.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Owner {
    /// Actual Solana owner verification key; API commands still need signatures.
    pub wallet: [u8; 32],
    /// Unit-local token recipient for Pacifica's quote asset.
    pub pacifica_tokens: [u8; 32],
    /// Unit-local token recipient for BULK's quote asset.
    pub bulk_tokens: [u8; 32],
}
/// Global private configuration, distinct from each unit's KMS plaintext.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    /// Fresh secret random blinding for the public configuration-body hash.
    /// Not a signing key; keep it private to prevent owner-list guessing.
    pub privacy_nonce: [u8; 32],
    /// Bounded managed inventory only. No public discovery or custody promise.
    pub owners: Vec<Owner>,
    /// Owner/agent authentication lifetime ceiling, milliseconds.
    pub auth_ms: u64,
    /// Strategy grant lifetime ceiling, milliseconds.
    pub grant_ms: u64,
}
/// Existing unit descriptor, using the canonical ledger codec unchanged.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unit {
    /// Canonical empty kernel configuration; no state/snapshot import.
    pub ledger: Vec<u8>,
    /// Exact pooled native owner/account bytes.
    pub native_account: [u8; 32],
    /// Declared inactive custody inventory, not an activated funds controller.
    pub funding_wallet: [u8; 32],
    /// Declared inactive custody configuration.
    pub custody_config: [u8; 32],
    /// Declared inactive payout vault.
    pub payout_vault: [u8; 32],
    /// Declared inactive custody policy; no deposit discovery is exposed.
    pub key_policy: [u8; 32],
}
impl Unit {
    fn descriptor(&self, id: [u8; 32], environment: [u8; 32]) -> Result<units::Descriptor, Error> {
        Ok(units::Descriptor {
            id: UnitId::new(id).map_err(|_| Error)?,
            environment,
            native_account: self.native_account,
            funding_wallet: self.funding_wallet,
            custody_config: self.custody_config,
            payout_vault: self.payout_vault,
            key_policy: self.key_policy,
            ledger: cinder_journal::wire::decode_config(&self.ledger).map_err(|_| Error)?,
        })
    }
}
/// Private Pacifica profile, released under its own configuration role.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pacifica {
    /// Empty unit descriptor.
    pub unit: Unit,
    /// Exact source index in the canonical ledger configuration.
    pub source: usize,
    /// Profile interpretation revision.
    pub revision: u64,
    /// Named interpretation evidence; not native activation by itself.
    pub evidence: String,
    /// Qualified native decimal grids.
    pub precision: pp::Level,
    /// Canonical market mappings.
    pub markets: Vec<pp::Mapping>,
    /// Native quote decimal scale.
    pub quote_places: u8,
    /// Native perpetual instrument tag.
    pub perp_tag: u64,
    /// Must be testnet/Unknown execution in version 7.
    pub execution: pacifica::Policy,
    /// Native agent epoch, independent of writer/storage generation.
    pub trading_epoch: u64,
}
/// A BULK grid maps to one exact kernel market, never inferred from responses.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BulkMarket {
    /// Native symbol.
    pub symbol: String,
    /// Kernel configuration market index.
    pub market: usize,
    /// Exact native size-step decimal.
    pub size_step: String,
    /// Exact native price-step decimal.
    pub price_step: String,
}
/// Private BULK profile, independent agent/key epoch and account scope.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bulk {
    /// Empty unit descriptor.
    pub unit: Unit,
    /// Exact source index.
    pub source: usize,
    /// Nonzero interpretation revision.
    pub revision: u64,
    /// Evidence commitment; no qualification boolean in this assembly.
    pub evidence: [u8; 32],
    /// Native quote decimal scale.
    pub quote_places: u8,
    /// Explicit mappings to configured markets.
    pub markets: Vec<BulkMarket>,
    /// Local one-shot egress lifetime, milliseconds.
    pub expiry_ms: u64,
    /// Governed allocated read/execution credits (not activated here).
    pub credits: u32,
    /// Reserved cancellation credits.
    pub cleanup_reserve: u32,
    /// Native agent epoch.
    pub trading_epoch: u64,
}
/// No risk-increasing financial decision can be admitted by this assembly.
pub struct ClosedAdmission;
impl cinder_api::Admission for ClosedAdmission {
    fn order_holds(
        &self,
        _: &cinder_journal::model::State,
        _: &cinder_journal::orders::Intent,
    ) -> Result<Vec<cinder_journal::model::Reservation>, cinder_api::Error> {
        Err(cinder_api::Error::Unavailable)
    }
}
type Api = Service<ClosedAdmission>;
type App<B> = super::Runtime<B, RecordCipher, ClosedAdmission>;
/// Actual loaded controllers/egress, retained privately, not exposed for callers
/// to bypass the inactive deployment gates.
struct Native {
    _pacifica: pacifica::Gateway,
    _bulk: bulk::Gateway,
    _pacifica_egress: Egress,
    _bulk_egress: BulkEgress,
}
/// Validated assembly before opening storage. No fixture constructors or ports.
pub struct Loaded {
    manifest: Manifest,
    config: units::Configuration,
    owners: Vec<Owner>,
    api: Api,
    native: Native,
    storage: Zeroizing<[u8; 32]>,
    witness: Credential,
    application: [u8; 32],
}
// Egress construction is pure. Its clock is replaced at open with the actual
// leased trusted clock; this sentinel can never authorize pre-open network I/O.
struct ClosedClock;
impl Clock for ClosedClock {
    fn now(&self) -> Result<u64, Error> {
        Err(Error)
    }
}
struct BootClock {
    clock: Arc<dyn Clock>,
    stop: Arc<AtomicBool>,
}
impl Clock for BootClock {
    fn now(&self) -> Result<u64, Error> {
        if self.stop.load(Ordering::SeqCst) {
            return Err(Error);
        }
        self.clock.now()
    }
}

fn take(
    keys: &mut BTreeMap<Role, Zeroizing<Vec<u8>>>,
    role: Role,
) -> Result<Zeroizing<[u8; 32]>, Error> {
    Ok(Zeroizing::new(
        keys.remove(&role)
            .ok_or(Error)?
            .as_slice()
            .try_into()
            .map_err(|_| Error)?,
    ))
}
fn capsule<T: serde::de::DeserializeOwned>(
    keys: &mut BTreeMap<Role, Zeroizing<Vec<u8>>>,
    role: Role,
) -> Result<T, Error> {
    serde_cbor::from_slice(&keys.remove(&role).ok_or(Error)?).map_err(|_| Error)
}
fn inventory(manifest: &Manifest, p: &Pacifica, b: &Bulk) -> Result<units::Configuration, Error> {
    let policy = manifest.units.as_ref().ok_or(Error)?;
    let p = p
        .unit
        .descriptor(policy.pacifica, pacifica::Origin::Testnet.commitment())?;
    let b = b
        .unit
        .descriptor(policy.bulk, cinder_bulk::Network::Testnet.commitment())?;
    units::Configuration::new(p.ledger.domain, vec![p, b]).map_err(|_| Error)
}
impl Configuration {
    /// Trusted preparation can derive the public registry from the actual two
    /// private configurations; never accept a host's unverified replacement.
    pub fn registry(manifest: &Manifest, p: &Pacifica, b: &Bulk) -> Result<[u8; 32], Error> {
        inventory(manifest, p, b)?.commitment().map_err(|_| Error)
    }
    /// Bind the actual loaded objects. No external calls and no journal writes.
    pub fn construct_for(
        self,
        manifest: &Manifest,
        mut keys: BTreeMap<Role, Zeroizing<Vec<u8>>>,
    ) -> Result<Loaded, Error> {
        manifest.validate()?;
        if manifest.version != 7 {
            return Err(Error);
        }
        crate::boot::validate_keys(&keys)?;
        let pbytes = keys.get(&Role::PacificaConfiguration).ok_or(Error)?.clone();
        let bbytes = keys.get(&Role::BulkConfiguration).ok_or(Error)?.clone();
        let p: Pacifica = capsule(&mut keys, Role::PacificaConfiguration)?;
        let b: Bulk = capsule(&mut keys, Role::BulkConfiguration)?;
        let policy = manifest.units.as_ref().ok_or(Error)?;
        let config = inventory(manifest, &p, &b)?;
        if config.commitment().map_err(|_| Error)? != policy.registry
            || [
                config.domain().network.bytes(),
                config.domain().deployment.bytes(),
            ]
            .concat()
                != manifest.domain
            || self.privacy_nonce == [0; 32]
            || self.owners.is_empty()
            || self.owners.len() > 16
            || self.owners.iter().enumerate().any(|(i, o)| {
                o.wallet == [0; 32]
                    || o.pacifica_tokens == [0; 32]
                    || o.bulk_tokens == [0; 32]
                    || self.owners[..i].iter().any(|old| old.wallet == o.wallet)
            })
        {
            return Err(Error);
        }
        let api = Service::new(
            Authorizer::new(self.auth_ms, self.grant_ms).map_err(|_| Error)?,
            ClosedAdmission,
        );
        let pd = config
            .units()
            .iter()
            .find(|u| u.id.bytes() == policy.pacifica)
            .ok_or(Error)?;
        let bd = config
            .units()
            .iter()
            .find(|u| u.id.bytes() == policy.bulk)
            .ok_or(Error)?;
        let pg = pacifica::Gateway::new_read_only(
            pp::Profile {
                config: pd.ledger.clone(),
                source: pd.ledger.sources.get(p.source).ok_or(Error)?.scope,
                account: cinder_pacifica::funding::chain::address(pd.native_account),
                environment: pacifica::Origin::Testnet.url().into(),
                revision: p.revision,
                evidence: p.evidence,
                precision: p.precision,
                fills: pp::Level::Unknown,
                markets: p.markets,
                quote_places: p.quote_places,
                perp_tag: p.perp_tag,
            },
            p.execution,
            take(&mut keys, Role::Trading)?,
            p.trading_epoch,
        )
        .map_err(|_| Error)?;
        pg.unit_binding(pd).map_err(|_| Error)?;
        let mappings = b
            .markets
            .iter()
            .map(|m| {
                Ok(bf::Mapping::new(
                    cinder_bulk::precision::Market::new(
                        &m.symbol,
                        &m.size_step,
                        &m.price_step,
                        b.quote_places,
                    )
                    .map_err(|_| Error)?,
                    bd.ledger.markets.get(m.market).ok_or(Error)?.unit(),
                ))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let bg = bulk::Gateway::new_read_only(
            bd.ledger.clone(),
            bf::Profile::new(
                cinder_bulk::Scope::new(
                    bd.ledger.sources.get(b.source).ok_or(Error)?.scope,
                    cinder_bulk::Network::Testnet,
                    bd.native_account,
                )
                .map_err(|_| Error)?,
                bd.ledger.quote,
                b.quote_places,
                bf::Target::Master,
                mappings,
            )
            .map_err(|_| Error)?,
            bulk::Policy {
                unit: bd.id,
                revision: b.revision,
                evidence: b.evidence,
                qualification: bulk::Level::Unknown,
                expiry_ms: b.expiry_ms,
                credits: b.credits,
                cleanup_reserve: b.cleanup_reserve,
            },
            take(&mut keys, Role::BulkTrading)?,
            b.trading_epoch,
        )
        .map_err(|_| Error)?;
        // Distinct secret bytes are necessary but not sufficient: neither agent
        // may equal either pooled owner/funds wallet or a managed user's owner.
        for key in [pg.agent_key(), bg.agent_key()] {
            if config
                .units()
                .iter()
                .any(|u| key == u.native_account || key == u.funding_wallet)
                || self.owners.iter().any(|o| o.wallet == key)
            {
                return Err(Error);
            }
        }
        if pg.agent_key() == bg.agent_key() {
            return Err(Error);
        }
        let (pe, be) = egress(manifest, Arc::new(ClosedClock))?;
        let application = sha256(
            &[
                b"CINDER-UNIT-LOADED-ASSEMBLY-1\0".as_slice(),
                &sha256(&serde_cbor::to_vec(&self).map_err(|_| Error)?),
                &sha256(&pbytes),
                &sha256(&bbytes),
                &api.commitment(&config).map_err(|_| Error)?,
                &pg.release_commitment(&pd.ledger).map_err(|_| Error)?,
                &bg.release_commitment(),
                &pe.commitment(),
                &be.commitment(),
                &serde_cbor::to_vec(manifest.history.as_ref().ok_or(Error)?).map_err(|_| Error)?,
            ]
            .concat(),
        );
        let storage = take(&mut keys, Role::Storage)?;
        let witness = capsule(&mut keys, Role::Witness)?;
        if !keys.is_empty() {
            return Err(Error);
        }
        Ok(Loaded {
            manifest: manifest.clone(),
            config,
            owners: self.owners,
            api,
            native: Native {
                _pacifica: pg,
                _bulk: bg,
                _pacifica_egress: pe,
                _bulk_egress: be,
            },
            storage,
            witness,
            application,
        })
    }
}
fn egress(manifest: &Manifest, clock: Arc<dyn Clock>) -> Result<(Egress, BulkEgress), Error> {
    let p = manifest.units.as_ref().ok_or(Error)?;
    Ok((
        Egress::new(
            pacifica::Origin::Testnet,
            Target::new(3, manifest.venue_port)?,
            Trust::from_der(&manifest.venue_root, manifest.venue_root_hash)?,
            clock.clone(),
        )?,
        BulkEgress::new(
            Target::new(3, p.bulk_port)?,
            Trust::from_der(&p.bulk_root, p.bulk_root_hash)?,
            clock,
        )?,
    ))
}
/// Real private handler plus retained inactive native controllers. No public
/// method exposes signing/funding or bypasses the measured inactive gates.
pub struct Instance<B: Backend> {
    handler: Arc<App<B>>,
    _native: Native,
    stop: Arc<AtomicBool>,
}
impl<B: Backend + Send + 'static> Instance<B> {
    /// Same TLS/Noise Handler, not a fixture pipe or host API implementation.
    pub fn handler(&self) -> Arc<dyn Handler> {
        self.handler.clone()
    }
    /// Finite inactive supervisor; no remote poll, signature or financial action.
    pub fn tick(&self) -> Result<(), Error> {
        if self.stop.load(Ordering::SeqCst) || self.handler.now().is_err() {
            self.fence();
            return Err(Error);
        }
        Ok(())
    }
    /// Sticky local fence and publication invalidation, shared with the listener.
    pub fn fence(&self) {
        self.stop.store(true, Ordering::SeqCst);
        self.handler.fence();
    }
}
impl Loaded {
    /// Derived commitment for trusted preparation, not a hardware qualification.
    pub fn commitment(&self) -> [u8; 32] {
        self.application
    }
    /// Only the real protected cloud backend is used by the shipping entrypoint.
    pub fn open(
        self,
        manifest: &Manifest,
        parent: Credential,
        clock: Arc<dyn Clock>,
        stop: Arc<AtomicBool>,
    ) -> Result<Instance<Storage>, Error> {
        self.matches(manifest)?;
        let stream = Stream {
            domain: self.config.domain(),
            id: manifest.stream,
        };
        let (backend, anchor, witness) = crate::runtime::open_storage(
            manifest,
            parent,
            Credential {
                access: self.witness.access.clone(),
                secret: self.witness.secret.clone(),
                token: self.witness.token.clone(),
                expires: self.witness.expires,
            },
            stream,
            clock.clone(),
        )?;
        self.attach(
            manifest,
            backend,
            anchor.head.is_none(),
            witness,
            clock,
            stop,
        )
    }
    fn matches(&self, manifest: &Manifest) -> Result<(), Error> {
        manifest.validate()?;
        if &self.manifest != manifest || manifest.application != self.application {
            return Err(Error);
        }
        Ok(())
    }
    fn attach<B: Backend + Send + 'static>(
        self,
        manifest: &Manifest,
        backend: B,
        fresh: bool,
        witness: Arc<dyn Witness>,
        clock: Arc<dyn Clock>,
        stop: Arc<AtomicBool>,
    ) -> Result<Instance<B>, Error> {
        self.matches(manifest)?;
        let now = clock.now()?;
        if stop.load(Ordering::SeqCst) {
            return Err(Error);
        }
        let cipher = RecordCipher::new(self.storage, manifest.generation, manifest.stream)
            .map_err(|_| Error)?;
        let mut journal = if fresh {
            units::Journal::create(backend, cipher, self.config)
        } else {
            units::Journal::open(backend, cipher, self.config)
        }
        .map_err(|_| Error)?;
        initialize_owners(
            &mut journal,
            &self.owners,
            manifest.units.as_ref().ok_or(Error)?,
            now,
        )?;
        let api = self
            .api
            .commitment(journal.configuration())
            .map_err(|_| Error)?;
        let binding = Binding {
            stream: Stream {
                domain: journal.configuration().domain(),
                id: manifest.stream,
            },
            epoch: manifest.epoch,
            api,
        };
        let private_clock: Arc<dyn Clock> = Arc::new(BootClock {
            clock,
            stop: stop.clone(),
        });
        let handler = super::Runtime::open(
            journal,
            self.api,
            binding,
            witness,
            private_clock,
            now.checked_add(manifest.gates.maximum_boot_ms)
                .ok_or(Error)?,
        )?;
        Ok(Instance {
            handler: Arc::new(handler),
            _native: self.native,
            stop,
        })
    }
}
fn initialize_owners<B: Backend>(
    j: &mut units::Journal<B, RecordCipher>,
    owners: &[Owner],
    p: &Policy,
    now: u64,
) -> Result<(), Error> {
    let state = j.verified_state().map_err(|_| Error)?;
    if state.logical_time() > now
        || state
            .customers()
            .iter()
            .any(|c| !owners.iter().any(|o| o.wallet == c.wallet))
    {
        return Err(Error);
    }
    for owner in owners {
        let recipients: Vec<_> = j
            .configuration()
            .units()
            .iter()
            .map(|u| units::Recipient {
                unit: u.id,
                tokens: if u.id.bytes() == p.pacifica {
                    owner.pacifica_tokens
                } else {
                    owner.bulk_tokens
                },
            })
            .collect();
        if let Some(customer) = j
            .state()
            .map_err(|_| Error)?
            .customers()
            .iter()
            .find(|c| c.wallet == owner.wallet)
        {
            for r in &recipients {
                if j.state()
                    .map_err(|_| Error)?
                    .unit(r.unit)
                    .ok_or(Error)?
                    .registrations()
                    .iter()
                    .find(|v| v.account == customer.account)
                    .is_none_or(|v| v.wallet != owner.wallet || v.tokens != r.tokens)
                {
                    return Err(Error);
                }
            }
            continue; // preserve existing global epoch, grants and operations.
        }
        let mut id = [0; 32];
        openssl::rand::rand_bytes(&mut id)?;
        let tx = units::Transaction {
            id: CommitId::new(id).map_err(|_| Error)?,
            expected: j.head(),
            at: now,
            change: units::Change::Enroll {
                wallet: owner.wallet,
                recipients,
            },
        };
        j.commit(tx).map_err(|_| Error)?;
    }
    Ok(())
}

#[cfg(all(test, feature = "local-fixture"))]
mod tests;
