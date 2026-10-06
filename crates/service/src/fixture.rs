//! Disposable offline attester, policy, clock and local witness. NONE is a Nitro
//! implementation. Never compiled into the default service/verifier/relay graph.
use crate::{
    Error,
    attestation::{Context, Policy, user_data},
    transport::{Attester, Clock, Handler, Session},
};
use aws_nitro_enclaves_cose::{CoseSign1, crypto::Openssl, header_map::HeaderMap};
use cinder_api::{Admission, Contract, OwnerBinding, Service};
use cinder_journal::{
    Head, Journal, collateral,
    encrypted::RecordCipher,
    model::*,
    orders,
    replicated::{Anchor, FileReplica, Replicated, Stream, Witness},
    risk,
};
use cinder_kernel::{
    amounts::*,
    identity::*,
    ledger::{evidence::*, *},
    position::Market,
};
use openssl::{
    asn1::Asn1Time,
    ec::{EcGroup, EcKey},
    hash::MessageDigest,
    nid::Nid,
    pkey::{PKey, Private},
    x509::{
        X509, X509NameBuilder,
        extension::{AuthorityKeyIdentifier, BasicConstraints, KeyUsage, SubjectKeyIdentifier},
    },
};
use serde_cbor::Value;
use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};
use zeroize::Zeroizing;

/// Local OS wall clock, not a qualified enclave clock.
pub struct FixtureClock;
impl cinder_web_channel::Entropy for FixtureClock {
    fn fill(&self, output: &mut [u8]) -> Result<(), cinder_web_channel::Error> {
        openssl::rand::rand_bytes(output).map_err(|_| cinder_web_channel::Error)
    }
}
impl Clock for FixtureClock {
    fn now(&self) -> Result<u64, Error> {
        u64::try_from(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| Error)?
                .as_millis(),
        )
        .map_err(|_| Error)
    }
}
/// Synthetic ECDSA CA/leaf, with actual signatures and X.509 constraints.
pub struct FixtureAttester {
    key: PKey<Private>,
    leaf: X509,
    root: X509,
}
impl crate::web::Attester for FixtureAttester {
    fn web_quote(
        &self,
        policy: &Policy,
        fields: &[u8; 128],
        expires: u64,
        _now: u64,
    ) -> Result<Vec<u8>, Error> {
        let context = cinder_web_channel::profile::context(&policy.encode(), fields, expires)
            .map_err(|_| Error)?;
        self.quote_fields(
            policy,
            FixtureClock.now()?,
            &fields[96..],
            &fields[..32],
            &cinder_web_channel::profile::user_data(&context),
        )
    }
}
impl FixtureAttester {
    /// Fresh disposable CA, not a static secret/root accepted by production.
    pub fn new() -> Result<Self, Error> {
        Self::build(true)
    }
    #[cfg(test)]
    pub(crate) fn without_leaf_identifier() -> Result<Self, Error> {
        Self::build(false)
    }
    fn build(leaf_identifier: bool) -> Result<Self, Error> {
        let group = EcGroup::from_curve_name(Nid::SECP384R1)?;
        let root_key = PKey::from_ec_key(EcKey::generate(&group)?)?;
        let root = certificate(&root_key, "fixture-root", None, true, true)?;
        let key = PKey::from_ec_key(EcKey::generate(&group)?)?;
        let leaf = certificate(
            &key,
            "fixture-leaf",
            Some((&root, &root_key)),
            false,
            leaf_identifier,
        )?;
        Ok(Self { key, leaf, root })
    }
    /// Public root for the EXPLICIT fixture verifier only.
    pub fn root(&self) -> &X509 {
        &self.root
    }
}
fn certificate(
    key: &PKey<Private>,
    name: &str,
    issuer: Option<(&X509, &PKey<Private>)>,
    ca: bool,
    identifier: bool,
) -> Result<X509, Error> {
    let mut n = X509NameBuilder::new()?;
    n.append_entry_by_text("CN", name)?;
    let n = n.build();
    let mut b = X509::builder()?;
    b.set_version(2)?;
    b.set_subject_name(&n)?;
    b.set_issuer_name(issuer.map(|(c, _)| c.subject_name()).unwrap_or(&n))?;
    b.set_pubkey(key)?;
    let mut serial = [0; 16];
    openssl::rand::rand_bytes(&mut serial)?;
    b.set_serial_number(
        openssl::bn::BigNum::from_slice(&serial)?
            .to_asn1_integer()?
            .as_ref(),
    )?;
    b.set_not_before(Asn1Time::from_unix(0)?.as_ref())?;
    b.set_not_after(Asn1Time::days_from_now(1)?.as_ref())?;
    let mut bc = BasicConstraints::new();
    bc.critical();
    if ca {
        bc.ca();
        bc.pathlen(0);
    }
    b.append_extension(bc.build()?)?;
    let mut ku = KeyUsage::new();
    ku.critical();
    if ca {
        ku.key_cert_sign().crl_sign();
    } else {
        ku.digital_signature();
    }
    b.append_extension(ku.build()?)?;
    let ski = SubjectKeyIdentifier::new()
        .build(&b.x509v3_context(issuer.map(|(c, _)| c.as_ref()), None))?;
    b.append_extension(ski)?;
    if identifier {
        let aki = AuthorityKeyIdentifier::new()
            .keyid(true)
            .build(&b.x509v3_context(issuer.map(|(c, _)| c.as_ref()), None))?;
        b.append_extension(aki)?;
    }
    b.sign(
        issuer.map(|(_, k)| k).unwrap_or(key),
        MessageDigest::sha384(),
    )?;
    Ok(b.build())
}
impl Attester for FixtureAttester {
    fn quote(&self, p: &Policy, c: &Context<'_>) -> Result<Vec<u8>, Error> {
        self.quote_fields(p, c.now, c.spki, &c.nonce, &user_data(p, c))
    }
}
impl FixtureAttester {
    pub(crate) fn quote_fields(
        &self,
        p: &Policy,
        at: u64,
        public_key: &[u8],
        nonce: &[u8],
        data: &[u8],
    ) -> Result<Vec<u8>, Error> {
        let mut map = BTreeMap::new();
        let mut put = |k: &str, v: Value| {
            map.insert(Value::Text(k.into()), v);
        };
        put("module_id", Value::Text("LOCAL-FIXTURE-NOT-NITRO".into()));
        put("timestamp", Value::Integer(at as i128));
        put("digest", Value::Text("SHA384".into()));
        put(
            "pcrs",
            Value::Map(
                p.pcrs
                    .iter()
                    .enumerate()
                    .map(|(n, b)| (Value::Integer(n as i128), Value::Bytes(b.to_vec())))
                    .collect(),
            ),
        );
        put("certificate", Value::Bytes(self.leaf.to_der()?));
        put(
            "cabundle",
            Value::Array(vec![Value::Bytes(self.root.to_der()?)]),
        );
        put("public_key", Value::Bytes(public_key.to_vec()));
        put("nonce", Value::Bytes(nonce.to_vec()));
        put("user_data", Value::Bytes(data.to_vec()));
        let mut payload = serde_cbor::to_vec(&Value::Map(map)).map_err(|_| Error)?;
        // Match the top-level map framing observed on actual NSM 1.0 hardware.
        // This remains a synthetic, separately rooted signed test document.
        if payload.first() != Some(&0xa9) {
            return Err(Error);
        }
        payload[0] = 0xbf;
        payload.push(0xff);
        CoseSign1::new::<Openssl>(&payload, &HeaderMap::new(), &self.key)
            .map_err(|_| Error)?
            .as_bytes(true)
            .map_err(|_| Error)
    }
}
/// Explicit synthetic release policy for offline tests only.
pub fn policy() -> Policy {
    Policy {
        domain: [[1; 32], [2; 32]].concat().try_into().unwrap(),
        manifest: [21; 32],
        pcrs: [[22; 48], [23; 48], [24; 48]],
    }
}

/// Full offline lifecycle controls; never a public endpoint or production port.
pub mod acceptance;
/// Joined SBF recovery fixture helpers, never available in the production build.
pub mod recovery;

// Public accepted-head file for the local crash test only. Same-host storage is
// NOT an authenticated independent witness and cannot qualify G03 freshness.
struct LocalWitness(PathBuf);
impl Witness for LocalWitness {
    fn read(&mut self, _: Stream) -> Result<Anchor, cinder_journal::Error> {
        let f = File::open(&self.0).map_err(|_| cinder_journal::Error::Storage)?;
        let mut b = Vec::new();
        f.take(49)
            .read_to_end(&mut b)
            .map_err(|_| cinder_journal::Error::Storage)?;
        if b == [0] {
            return Ok(Anchor {
                epoch: 1,
                head: None,
            });
        }
        let (epoch, at) = if b.len() == 40 {
            (1, 0)
        } else if b.len() == 48 {
            (u64::from_be_bytes(b[..8].try_into().unwrap()), 8)
        } else {
            return Err(cinder_journal::Error::Storage);
        };
        if epoch == 0 {
            return Err(cinder_journal::Error::Storage);
        }
        Ok(Anchor {
            epoch,
            head: Some(Head {
                sequence: u64::from_be_bytes(b[at..at + 8].try_into().unwrap()),
                hash: b[at + 8..].try_into().unwrap(),
            }),
        })
    }
    fn accept(
        &mut self,
        s: Stream,
        expected: Anchor,
        next: Head,
    ) -> Result<(), cinder_journal::Error> {
        if self.read(s)? != expected {
            return Err(cinder_journal::Error::Stale);
        }
        let tmp = self.0.with_extension("pending");
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
            .map_err(|_| cinder_journal::Error::Storage)?;
        let bytes = if expected.epoch == 1 {
            [&next.sequence.to_be_bytes()[..], &next.hash].concat()
        } else {
            [
                &expected.epoch.to_be_bytes()[..],
                &next.sequence.to_be_bytes(),
                &next.hash,
            ]
            .concat()
        };
        f.write_all(&bytes)
            .and_then(|_| f.sync_all())
            .map_err(|_| cinder_journal::Error::Storage)?;
        std::fs::rename(tmp, &self.0).map_err(|_| cinder_journal::Error::Storage)?;
        File::open(self.0.parent().unwrap())
            .and_then(|f| f.sync_all())
            .map_err(|_| cinder_journal::Error::Storage)
    }
}
type Store = Journal<Replicated<FileReplica, FileReplica, LocalWitness>, RecordCipher>;
struct Holds(bool);
impl Admission for Holds {
    fn order_holds(
        &self,
        state: &State,
        i: &orders::Intent,
    ) -> Result<Vec<Reservation>, cinder_api::Error> {
        if self.0 {
            return state
                .order_reservations(i)
                .map_err(|_| cinder_api::Error::Unavailable);
        }
        Ok(vec![
            Reservation {
                resource: Resource::Customer(i.request.account),
                amount: cash(100),
            },
            Reservation {
                resource: Resource::Location(Location::Venue),
                amount: cash(100),
            },
        ])
    }
}
/// Real P18 handler plus real P06 AEAD/replicated journal; fixture venue exposes
/// one retained attempt without making any native network request or signature.
pub struct FixtureHandler {
    store: Mutex<Store>,
    service: Service<Holds>,
    native: Option<Mutex<acceptance::Native>>,
}
impl FixtureHandler {
    /// Key arrives through harness stdin, never parent disk/logs. Reuse on restart
    /// is a test harness responsibility, not a production key-release design.
    pub fn open(root: &Path, key: Zeroizing<[u8; 32]>, wallet: [u8; 32]) -> Result<Self, Error> {
        Self::open_inner(root, key, wallet, [81; 32], None, false).map(|(handler, _)| handler)
    }
    /// Same service/journal with SBF customer custody and synthetic venue house capital.
    /// Only the explicit joined offline harness may choose this fixture setup.
    pub fn open_recovery(
        root: &Path,
        key: Zeroizing<[u8; 32]>,
        wallet: [u8; 32],
        controller: &cinder_pacifica::funding::Controller,
        keys: &[crate::recovery::RecipientKey],
    ) -> Result<Self, Error> {
        let route = controller.route();
        if route.beneficiaries.len() != 1 || route.beneficiaries[0].wallet != wallet {
            return Err(Error);
        }
        let (h, fresh) = Self::open_inner(
            root,
            key,
            wallet,
            route.beneficiaries[0].tokens,
            Some(controller),
            false,
        )?;
        let mut store = h.store.lock().map_err(|_| Error)?;
        if fresh {
            crate::recovery::register_keys(
                controller,
                &mut *store,
                recovery::commit_id()?,
                FixtureClock.now()?,
                keys,
            )?;
        } else {
            crate::recovery::require_registered(controller, &mut *store, keys)?;
        }
        drop(store);
        Ok(h)
    }
    /// Explicit synthetic native ports with joined risk, no seeded customer credit.
    pub fn open_acceptance(
        root: &Path,
        key: Zeroizing<[u8; 32]>,
        wallet: [u8; 32],
        binding: &recovery::Binding,
    ) -> Result<Self, Error> {
        let native = acceptance::Native::new(binding)?;
        if binding.route.beneficiaries.len() != 1
            || binding.route.beneficiaries[0].wallet != wallet
            || binding.route.beneficiaries[0].account != user().bytes()
        {
            return Err(Error);
        }
        let tokens = binding.route.beneficiaries[0].tokens;
        let (mut h, fresh) =
            Self::open_inner(root, key, wallet, tokens, Some(&native.controller), true)?;
        if fresh {
            native.activate(&mut *h.store.lock().map_err(|_| Error)?)?;
        }
        h.native = Some(Mutex::new(native));
        Ok(h)
    }
    /// Private test-harness stdin only. No control is reachable through the relay.
    pub fn acceptance_control(
        &self,
        command: acceptance::Command,
    ) -> Result<serde_json::Value, Error> {
        let mut store = self.store.lock().map_err(|_| Error)?;
        self.native
            .as_ref()
            .ok_or(Error)?
            .lock()
            .map_err(|_| Error)?
            .control(&mut store, command)
    }
    fn open_inner(
        root: &Path,
        key: Zeroizing<[u8; 32]>,
        wallet: [u8; 32],
        tokens: [u8; 32],
        controller: Option<&cinder_pacifica::funding::Controller>,
        acceptance: bool,
    ) -> Result<(Self, bool), Error> {
        let fresh = !root.join("accepted").exists();
        if fresh {
            // A dirty/missing accepted register must never silently initialize.
            if root.read_dir()?.next().is_some() {
                return Err(Error);
            }
            let mut f = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(root.join("accepted"))?;
            f.write_all(&[0])?;
            f.sync_all()?;
            File::open(root)?.sync_all()?;
            FileReplica::create(&root.join("first")).map_err(|_| Error)?;
            FileReplica::create(&root.join("second")).map_err(|_| Error)?;
        }
        let backend = Replicated::new(
            Stream {
                domain: config().domain,
                id: [42; 32],
            },
            1,
            FileReplica::open(&root.join("first")).map_err(|_| Error)?,
            FileReplica::open(&root.join("second")).map_err(|_| Error)?,
            LocalWitness(root.join("accepted")),
        )
        .map_err(|_| Error)?;
        let cipher = RecordCipher::new(key, 1, [42; 32]).map_err(|_| Error)?;
        let mut store = if fresh {
            Journal::create(backend, cipher, config())
        } else {
            Journal::open(backend, cipher, config())
        }
        .map_err(|_| Error)?;
        let service = Service::new(
            Contract {
                owners: vec![OwnerBinding {
                    account: user(),
                    wallet,
                    tokens,
                }],
                maximum_auth_lifetime: 120_000,
                maximum_grant_lifetime: 3_600_000,
            },
            Holds(acceptance),
        )
        .map_err(|_| Error)?;
        let now = FixtureClock.now()?;
        if fresh {
            seed(&mut store, now, controller.is_some(), !acceptance)?;
            if let Some(c) = controller {
                c.bind(&mut store, recovery::commit_id()?, now)
                    .map_err(|_| Error)?;
                c.observe_setup(
                    &mut store,
                    recovery::commit_id()?,
                    now,
                    cinder_pacifica::funding::Setup {
                        account: c.route().broker,
                        observed_at: now,
                        lending_disabled: true,
                        borrowed: "0".into(),
                        interest: "0".into(),
                        complete: true,
                    },
                )
                .map_err(|_| Error)?;
            } else {
                // The legacy transport-only fixture has no native controller.
                // Its readiness is explicitly synthetic, never a production
                // fallback or a replacement for the joined controller's Setup.
                commit(
                    &mut store,
                    now,
                    vec![],
                    vec![Control::Funds(
                        cinder_journal::funds::Action::NativeCreditReady(true),
                    )],
                )?;
            }
        }
        service.initialize(&mut store, now).map_err(|_| Error)?;
        Ok((
            Self {
                store: Mutex::new(store),
                service,
                native: None,
            },
            fresh,
        ))
    }
}
impl Handler for FixtureHandler {
    fn handle(
        &self,
        channel: &Session,
        request: PrivateBytes,
        now: u64,
    ) -> Result<PrivateBytes, Error> {
        let mut store = match self.store.try_lock() {
            Ok(s) => s,
            Err(_) => {
                return cinder_api::Error::Unavailable
                    .encode_private()
                    .map_err(|_| Error);
            }
        };
        let response = self.service.handle(&mut store, channel, &request, now);
        if let Ok(cinder_api::Response::Receipt(r)) = &response {
            let key = RequestKey {
                domain: config().domain,
                account: user(),
                request: r.id,
            };
            if r.outcome == cinder_api::Outcome::Accepted
                && store
                    .state()
                    .map_err(|_| Error)?
                    .orders()
                    .iter()
                    .any(|o| o.intent.request == key && o.attempt.is_none())
            {
                let attempt = AttemptKey {
                    request: key,
                    attempt: AttemptId::new(r.id.bytes()).map_err(|_| Error)?,
                };
                let mut tx = empty(&store, now);
                tx.controls = vec![Control::Order(orders::Action::Prepare { attempt })];
                if self.native.is_none() {
                    tx.controls.push(Control::Expose(attempt));
                }
                let committed = store.commit(tx).map_err(|_| Error)?;
                if committed.receipt.controls.is_some() {
                    return Err(Error);
                }
                if let Some(native) = &self.native {
                    native
                        .lock()
                        .map_err(|_| Error)?
                        .dispatch(&mut store, now)?;
                }
                // Query the same signed request after durable fixture dispatch.
                return self
                    .service
                    .handle(&mut store, channel, &request, now)
                    .and_then(|r| r.encode())
                    .map_err(|_| Error);
            }
        }
        if let Some(native) = &self.native {
            native
                .lock()
                .map_err(|_| Error)?
                .dispatch(&mut store, now)?;
            if matches!(&response, Ok(cinder_api::Response::Receipt(_))) {
                return self
                    .service
                    .handle(&mut store, channel, &request, now)
                    .and_then(|r| r.encode())
                    .map_err(|_| Error);
            }
        }
        match response {
            Ok(r) => r.encode(),
            Err(e) => e.encode_private(),
        }
        .map_err(|_| Error)
    }
}
fn config() -> Config {
    let domain = Domain {
        network: NetworkId::new([1; 32]).unwrap(),
        deployment: DeploymentId::new([2; 32]).unwrap(),
    };
    let quote = AssetUnit {
        asset: AssetId::new([3; 32]).unwrap(),
        precision: PrecisionVersion::new(1).unwrap(),
    };
    let venue = VenueId::new([4; 32]).unwrap();
    let account = VenueAccountId::new([5; 32]).unwrap();
    Config {
        domain,
        quote,
        venue,
        venue_account: account,
        policy: PolicyVersion::new(1).unwrap(),
        sources: [Location::Venue, Location::Vault, Location::Broker]
            .into_iter()
            .enumerate()
            .map(|(n, location)| Source {
                scope: EventScope {
                    domain,
                    venue,
                    account,
                    namespace: NamespaceId::new([6 + n as u8; 32]).unwrap(),
                },
                location,
            })
            .collect(),
        markets: vec![
            Market::new(
                MarketUnit {
                    market: MarketId::new([7; 32]).unwrap(),
                    precision: PrecisionVersion::new(1).unwrap(),
                    quote,
                },
                1,
                1,
            )
            .unwrap(),
        ],
        customers: vec![user()],
    }
}
fn user() -> AccountId {
    AccountId::new([1; 32]).unwrap()
}
fn cash(n: i128) -> QuoteAtoms {
    QuoteAtoms::new(config().quote, n)
}
fn empty(store: &Store, now: u64) -> Transaction {
    let mut id = [0; 32];
    openssl::rand::rand_bytes(&mut id).unwrap();
    Transaction {
        id: CommitId::new(id).unwrap(),
        expected: store.head(),
        at: now,
        evidence: vec![],
        inputs: vec![],
        order_observations: vec![],
        funds_observations: vec![],
        controls: vec![],
    }
}
fn event(n: u8, change: Change) -> Event {
    Event {
        key: RecordKey::Economic(EventKey {
            scope: config().sources[0].scope,
            event: EconomicEventId::new(&[n]).unwrap(),
            leg: 0,
        }),
        policy: config().policy,
        change,
    }
}
fn commit(
    store: &mut Store,
    now: u64,
    events: Vec<Event>,
    controls: Vec<Control>,
) -> Result<(), Error> {
    let mut tx = empty(store, now);
    tx.controls = controls;
    tx.inputs = events
        .into_iter()
        .map(|e| Input {
            source: match &e.key {
                RecordKey::Economic(k) => k.scope,
                _ => config().sources[0].scope,
            },
            source_cut: Some(0),
            authority_epoch: 1,
            observed_at: now,
            raw: PrivateBytes::new(b"synthetic-source-only".to_vec()).unwrap(),
            event: Some(e),
        })
        .collect();
    if store
        .commit(tx)
        .map_err(|_| Error)?
        .receipt
        .controls
        .is_some()
    {
        return Err(Error);
    }
    Ok(())
}
fn seed(store: &mut Store, now: u64, recovery: bool, customer_credit: bool) -> Result<(), Error> {
    let initial = |n, owner, location| {
        let mut e = event(
            n,
            Change::Receipt {
                owner,
                location,
                amount: cash(1000),
            },
        );
        if let RecordKey::Economic(k) = &mut e.key {
            k.scope = config()
                .sources
                .iter()
                .find(|s| s.location == location)
                .unwrap()
                .scope;
        }
        e
    };
    let mut receipts = vec![initial(2, Owner::House, Location::Venue)];
    if customer_credit {
        receipts.insert(
            0,
            initial(
                1,
                Owner::Customer(user()),
                if recovery {
                    Location::Vault
                } else {
                    Location::Venue
                },
            ),
        );
    }
    commit(store, now, receipts, vec![])?;
    let l = store.state().map_err(|_| Error)?.ledger();
    let check = event(
        3,
        Change::Reconcile(NativeCheck {
            expected_version: l.version(),
            cash: Some(l.venue().cash()),
            funding: Some(l.venue().funding()),
            positions: Some(l.venue().positions().to_vec()),
            complete: true,
            resolves: vec![],
        }),
    );
    commit(store, now, vec![check], vec![])?;
    let market = config().markets[0].unit();
    let price = PriceTicks::new(market, 100).unwrap();
    let expiry = now + 86_400_000;
    let cut = collateral::Cut {
        expected_version: store.state().map_err(|_| Error)?.ledger().version(),
        policy: collateral::Policy {
            revision: config().policy,
            markets: vec![collateral::MarginRule {
                market,
                private_bps: 1000,
                native_bps: 1000,
            }],
            evidence: EvidencePolicy {
                max_issue_age: 86_400_000,
                max_mark_age: 86_400_000,
                max_check_age: 86_400_000,
            },
        },
        marks: vec![MarkObservation {
            price,
            evidence: EventKey {
                scope: config().sources[0].scope,
                event: EconomicEventId::new(&[99]).unwrap(),
                leg: 0,
            },
            observed_at: now,
            valid_until: expiry,
            qualified: true,
        }],
    };
    let policy = risk::Policy {
        revision: config().policy,
        markets: vec![risk::MarketRule {
            market,
            maximum_leverage: 100_000,
            maintenance_bps: 500,
            native_maintenance_bps: 500,
            gross_limit: cash(1_000_000),
            net_limit: cash(1_000_000),
        }],
        buffer: cash(0),
        horizon_ms: 50,
        valid_until: expiry,
        paths: vec![risk::Path {
            id: [1; 32],
            steps: vec![risk::Step {
                after_ms: 1,
                marks: vec![price],
                events: vec![],
                liquidity: [Location::Vault, Location::Venue, Location::Broker]
                    .into_iter()
                    .map(|location| risk::Liquidity {
                        location,
                        accessible_bps: 10_000,
                        due: cash(0),
                    })
                    .collect(),
            }],
        }],
    };
    let version = store.state().map_err(|_| Error)?.ledger().version();
    commit(
        store,
        now,
        vec![],
        vec![
            Control::Collateral(cut),
            Control::Risk(risk::Action::Install {
                expected_version: version,
                policy: Box::new(policy),
            }),
        ],
    )
}
