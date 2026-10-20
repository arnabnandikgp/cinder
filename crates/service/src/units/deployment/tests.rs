//! Explicit offline ports only; actual deployment constructors/RecordCipher and
//! owner signatures, not hardware, native credit or production qualification.
use super::*;
use crate::{
    boot::Slot,
    test_support,
    transport::{Identity, Server, Session},
};
use cinder_api::{ConfidentialChannel, wire::Command};
use cinder_journal::{Frame, Head, model::PrivateBytes, replicated::Anchor};
use openssl::{
    pkey::{Id, PKey},
    sign::Signer,
};
use std::sync::{Mutex, atomic::AtomicU64};

fn public(n: u8) -> [u8; 32] {
    PKey::private_key_from_raw_bytes(&[n; 32], Id::ED25519)
        .unwrap()
        .raw_public_key()
        .unwrap()
        .try_into()
        .unwrap()
}
fn unit(n: u8) -> Unit {
    let mut ledger = test_support::config();
    ledger.customers.clear();
    ledger.venue = VenueId::new([n + 1; 32]).unwrap();
    ledger.venue_account = VenueAccountId::new([n + 2; 32]).unwrap();
    ledger.sources[0].scope.venue = ledger.venue;
    ledger.sources[0].scope.account = ledger.venue_account;
    Unit {
        ledger: cinder_journal::wire::encode_config(&ledger).unwrap(),
        native_account: public(n),
        funding_wallet: public(n),
        custody_config: [n + 3; 32],
        payout_vault: [n + 4; 32],
        key_policy: [n + 5; 32],
    }
}
fn profiles() -> (Configuration, Pacifica, Bulk) {
    (
        Configuration {
            privacy_nonce: [90; 32], // explicit disposable test blinding only.
            owners: vec![Owner {
                wallet: public(9),
                pacifica_tokens: [71; 32],
                bulk_tokens: [72; 32],
            }],
            auth_ms: 2000,
            grant_ms: 10000,
        },
        Pacifica {
            unit: unit(50),
            source: 0,
            revision: 1,
            evidence: "offline explicit unit assembly".into(),
            precision: pp::Level::Qualified,
            markets: vec![pp::Mapping {
                symbol: "BTC".into(),
                market: 0,
                size: pp::Grid { places: 0, step: 1 },
                price: pp::Grid { places: 0, step: 1 },
            }],
            quote_places: 0,
            perp_tag: 0,
            execution: pacifica::Policy {
                revision: 1,
                evidence: "offline unknown execution".into(),
                execution: pp::Level::Unknown,
                origin: pacifica::Origin::Testnet,
                expiry_ms: 3000,
                credits: 1000,
                cleanup_reserve: 100,
                read_cost: 120,
            },
            trading_epoch: 1,
            fills: None,
        },
        Bulk {
            unit: unit(40),
            source: 0,
            revision: 1,
            evidence: [88; 32],
            quote_places: 0,
            markets: vec![BulkMarket {
                symbol: "BTC-USD".into(),
                market: 0,
                size_step: "1".into(),
                price_step: "1".into(),
            }],
            expiry_ms: 3000,
            credits: 100,
            cleanup_reserve: 10,
            trading_epoch: 1,
            qualification: None,
        },
    )
}
fn input() -> (Manifest, BTreeMap<Role, Zeroizing<Vec<u8>>>) {
    let (c, p, b) = profiles();
    let mut m = crate::boot::qualification_manifest();
    m.version = 7;
    m.history = Some(crate::boot::HistoryPolicy {
        record_bytes: 65536,
        history_bytes: 1048576,
        records: 64,
        put_requests: 128,
        put_bytes: 16777216,
    });
    m.units = Some(Policy {
        registry: [1; 32],
        pacifica: [50; 32],
        bulk: [40; 32],
        bulk_port: 9010,
        bulk_root: m.venue_root.clone(),
        bulk_root_hash: m.venue_root_hash,
        worker: None,
    });
    m.units.as_mut().unwrap().registry = Configuration::registry(&m, &p, &b).unwrap();
    let witness = Credential {
        access: "PRIVATEWITNESS".into(),
        secret: "private offline credential".into(),
        token: "offline token".into(),
        expires: 999999,
    };
    let keys = BTreeMap::from([
        (
            Role::Configuration,
            Zeroizing::new(serde_cbor::to_vec(&c).unwrap()),
        ),
        (
            Role::PacificaConfiguration,
            Zeroizing::new(serde_cbor::to_vec(&p).unwrap()),
        ),
        (
            Role::BulkConfiguration,
            Zeroizing::new(serde_cbor::to_vec(&b).unwrap()),
        ),
        (Role::Storage, Zeroizing::new(vec![42; 32])),
        (Role::Trading, Zeroizing::new(vec![11; 32])),
        (Role::BulkTrading, Zeroizing::new(vec![12; 32])),
        (
            Role::Witness,
            Zeroizing::new(serde_cbor::to_vec(&witness).unwrap()),
        ),
    ]);
    let template = m.slots[0].clone();
    m.slots = keys
        .iter()
        .enumerate()
        .map(|(i, (role, body))| {
            let mut endpoint = template.endpoint.clone();
            endpoint.resource = format!(
                "arn:aws:kms:us-east-1:123456789012:key/{:08}-1111-2222-3333-444444444444",
                i + 1
            );
            Slot {
                role: *role,
                endpoint,
                plaintext_hash: sha256(body),
            }
        })
        .collect();
    m.application = crate::application::construct(&m, keys.clone())
        .unwrap()
        .commitment();
    (m, keys)
}
fn worker_input(trading: bool) -> (Manifest, BTreeMap<Role, Zeroizing<Vec<u8>>>) {
    use cinder_kernel::ledger::Location;
    let (mut m, mut keys) = input();
    m.version = 8;
    m.gates.native_reads = true;
    m.gates.trading = trading;
    m.units.as_mut().unwrap().worker = Some(super::super::worker::Policy {
        poll_ms: 1000,
        pacifica_reads: 1,
        bulk_reads: 1,
        pacifica_dispatches: u16::from(trading),
        bulk_dispatches: u16::from(trading),
    });
    let add_sources = |unit: &mut Unit| {
        let mut c = cinder_journal::wire::decode_config(&unit.ledger).unwrap();
        for (n, location) in [(80, Location::Vault), (81, Location::Broker)] {
            let mut source = c.sources[0];
            source.scope.namespace = NamespaceId::new([n; 32]).unwrap();
            source.location = location;
            c.sources.push(source);
        }
        unit.ledger = cinder_journal::wire::encode_config(&c).unwrap();
    };
    let mut p: Pacifica = serde_cbor::from_slice(&keys[&Role::PacificaConfiguration]).unwrap();
    add_sources(&mut p.unit);
    p.fills = Some(if trading {
        pp::Level::Qualified
    } else {
        pp::Level::Unknown
    });
    if trading {
        p.execution.execution = pp::Level::Qualified;
    }
    let mut b: Bulk = serde_cbor::from_slice(&keys[&Role::BulkConfiguration]).unwrap();
    add_sources(&mut b.unit);
    b.qualification = Some(if trading {
        bulk::Level::Qualified
    } else {
        bulk::Level::Unknown
    });
    keys.insert(
        Role::PacificaConfiguration,
        Zeroizing::new(serde_cbor::to_vec(&p).unwrap()),
    );
    keys.insert(
        Role::BulkConfiguration,
        Zeroizing::new(serde_cbor::to_vec(&b).unwrap()),
    );
    crate::application::prepare(&mut m, &keys).unwrap();
    (m, keys)
}

#[test]
fn measured_worker_policy_is_explicit_bounded_and_versioned() {
    let (m, keys) = worker_input(false);
    assert_ne!(m.digest().unwrap(), input().0.digest().unwrap());
    for (role, body) in &keys {
        assert!(m.wrap(*role, body).unwrap().starts_with(b"CKR8"));
    }
    for case in 0..11 {
        let mut bad = m.clone();
        let p = bad.units.as_mut().unwrap();
        match case {
            0 => p.worker = None,
            1 => p.worker.as_mut().unwrap().poll_ms = 999,
            2 => p.worker.as_mut().unwrap().poll_ms = 60001,
            3 => p.worker.as_mut().unwrap().pacifica_reads = 33,
            4 => p.worker.as_mut().unwrap().bulk_reads = 33,
            5 => p.worker.as_mut().unwrap().pacifica_dispatches = 17,
            6 => p.worker.as_mut().unwrap().bulk_dispatches = 17,
            7 => bad.gates.native_reads = false,
            8 => bad.gates.trading = true,
            9 => bad.gates.funding = true,
            _ => bad.version = 7,
        }
        assert!(bad.validate().is_err(), "case {case}");
    }
    // An omitted optional field preserves the exact version-7 private encoding.
    let (_, p, b) = profiles();
    for value in [
        serde_cbor::to_vec(&p).unwrap(),
        serde_cbor::to_vec(&b).unwrap(),
        serde_cbor::to_vec(&input().0.units.unwrap()).unwrap(),
    ] {
        let serde_cbor::Value::Map(fields) = serde_cbor::from_slice(&value).unwrap() else {
            panic!("map");
        };
        for name in ["fills", "qualification", "worker"] {
            assert!(!fields.contains_key(&serde_cbor::Value::Text(name.into())));
        }
    }
}

#[test]
fn worker_shipping_configuration_requires_explicit_qualification_and_all_locations() {
    for trading in [false, true] {
        let (m, keys) = worker_input(trading);
        for case in 0..6 {
            let mut changed = m.clone();
            let mut k = keys.clone();
            let mut p: Pacifica = serde_cbor::from_slice(&k[&Role::PacificaConfiguration]).unwrap();
            let mut b: Bulk = serde_cbor::from_slice(&k[&Role::BulkConfiguration]).unwrap();
            match case {
                0 => p.fills = None,
                1 => b.qualification = None,
                2 => {
                    let mut c = cinder_journal::wire::decode_config(&b.unit.ledger).unwrap();
                    c.sources.pop();
                    b.unit.ledger = cinder_journal::wire::encode_config(&c).unwrap();
                }
                3 => p.source = 1,
                4 => {
                    changed.gates.trading = true;
                    changed
                        .units
                        .as_mut()
                        .unwrap()
                        .worker
                        .as_mut()
                        .unwrap()
                        .pacifica_dispatches = 1;
                    p.fills = Some(pp::Level::Unknown);
                }
                _ => {
                    changed.gates.trading = true;
                    changed
                        .units
                        .as_mut()
                        .unwrap()
                        .worker
                        .as_mut()
                        .unwrap()
                        .bulk_dispatches = 1;
                    b.qualification = Some(bulk::Level::Unknown);
                }
            }
            k.insert(
                Role::PacificaConfiguration,
                Zeroizing::new(serde_cbor::to_vec(&p).unwrap()),
            );
            k.insert(
                Role::BulkConfiguration,
                Zeroizing::new(serde_cbor::to_vec(&b).unwrap()),
            );
            assert!(
                crate::application::prepare(&mut changed, &k).is_err(),
                "trading {trading}, case {case}"
            );
        }
    }
}

#[test]
fn worker_enabled_empty_startup_does_not_invent_financial_readiness() {
    let (m, keys) = worker_input(true);
    let app = attach(
        &m,
        keys,
        shared(),
        Arc::new(Time(AtomicU64::new(100))),
        Arc::new(AtomicBool::new(false)),
        true,
    )
    .unwrap();
    let journal = app.handler.journal.lock().unwrap();
    let state = journal.state().unwrap();
    for id in [40, 50] {
        let state = state.unit(UnitId::new([id; 32]).unwrap()).unwrap();
        assert!(!state.native_funding_ready());
        assert!(state.orders().is_empty());
        assert!(state.attempts().is_empty());
        assert!(state.holds().is_empty());
    }
}
fn loaded(m: &Manifest, keys: BTreeMap<Role, Zeroizing<Vec<u8>>>) -> Loaded {
    match crate::application::construct(m, keys).unwrap() {
        crate::application::Loaded::Units(v) => *v,
        _ => panic!("wrong measured mode"),
    }
}
#[derive(Default)]
struct Shared {
    frames: Mutex<Vec<Frame>>,
    epoch: AtomicU64,
}
struct Memory(Arc<Shared>);
impl Backend for Memory {
    fn load(&mut self) -> Result<Vec<Frame>, cinder_journal::Error> {
        Ok(self.0.frames.lock().unwrap().clone())
    }
    fn check_current(&mut self, h: Head) -> Result<(), cinder_journal::Error> {
        if self.0.frames.lock().unwrap().last().map(|f| f.head) == Some(h) {
            Ok(())
        } else {
            Err(cinder_journal::Error::Stale)
        }
    }
    fn append(
        &mut self,
        expected: Option<Head>,
        frame: &Frame,
    ) -> Result<(), cinder_journal::Error> {
        let mut frames = self.0.frames.lock().unwrap();
        if frames.last().map(|f| f.head) != expected {
            return Err(cinder_journal::Error::Stale);
        }
        frames.push(frame.clone());
        Ok(())
    }
}
struct Fresh(Arc<Shared>);
impl Witness for Fresh {
    fn read(&self, _: Stream) -> Result<Anchor, cinder_journal::Error> {
        Ok(Anchor {
            epoch: self.0.epoch.load(Ordering::SeqCst),
            head: self.0.frames.lock().unwrap().last().map(|f| f.head),
        })
    }
}
struct Time(AtomicU64);
impl Clock for Time {
    fn now(&self) -> Result<u64, Error> {
        Ok(self.0.load(Ordering::SeqCst))
    }
}
fn attach(
    m: &Manifest,
    keys: BTreeMap<Role, Zeroizing<Vec<u8>>>,
    shared: Arc<Shared>,
    time: Arc<Time>,
    stop: Arc<AtomicBool>,
    fresh: bool,
) -> Result<Instance<Memory>, Error> {
    loaded(m, keys).attach(
        m,
        Memory(shared.clone()),
        fresh,
        Arc::new(Fresh(shared)),
        time,
        stop,
    )
}
fn request(
    unit: u8,
    id: u8,
    epoch: u64,
    session: &Session,
    command: Command,
    now: u64,
) -> PrivateBytes {
    let mut request = cinder_api::units::Request {
        unit: UnitId::new([unit; 32]).unwrap(),
        body: cinder_api::wire::Request {
            domain: test_support::config().domain,
            account: cinder_journal::onboarding::account(test_support::config().domain, public(9))
                .unwrap(),
            id: RequestId::new([id; 32]).unwrap(),
            policy: PolicyVersion::new(1).unwrap(),
            epoch,
            signer: public(9),
            session: session.binding(),
            expires_at: now + 1000,
            command,
            signature: [0; 64],
        },
    };
    request.body.signature = Signer::new_without_digest(
        &PKey::private_key_from_raw_bytes(&[9; 32], Id::ED25519).unwrap(),
    )
    .unwrap()
    .sign_oneshot_to_vec(&request.message())
    .unwrap()
    .try_into()
    .unwrap();
    request.encode().unwrap()
}
fn shared() -> Arc<Shared> {
    Arc::new(Shared {
        epoch: AtomicU64::new(1),
        ..Shared::default()
    })
}
fn session() -> Session {
    Session::established(test_support::config().domain, [8; 32], 120000)
}

fn two_owners(m: &mut Manifest, keys: &mut BTreeMap<Role, Zeroizing<Vec<u8>>>) {
    let mut c: Configuration = serde_cbor::from_slice(&keys[&Role::Configuration]).unwrap();
    c.owners.push(Owner {
        wallet: public(10),
        pacifica_tokens: [73; 32],
        bulk_tokens: [74; 32],
    });
    keys.insert(
        Role::Configuration,
        Zeroizing::new(serde_cbor::to_vec(&c).unwrap()),
    );
    crate::application::prepare(m, keys).unwrap();
}

#[test]
fn startup_recipient_aliases_are_rejected_before_preparation_or_enrollment() {
    let (mut m, mut keys) = input();
    two_owners(&mut m, &mut keys);
    for unit in [0, 1] {
        let mut k = keys.clone();
        let mut c: Configuration = serde_cbor::from_slice(&k[&Role::Configuration]).unwrap();
        if unit == 0 {
            c.owners[1].pacifica_tokens = c.owners[0].pacifica_tokens;
        } else {
            c.owners[1].bulk_tokens = c.owners[0].bulk_tokens;
        }
        k.insert(
            Role::Configuration,
            Zeroizing::new(serde_cbor::to_vec(&c).unwrap()),
        );
        assert!(crate::application::prepare(&mut m.clone(), &k).is_err());
        assert!(crate::application::construct(&m, k).is_err());
    }
    // The same address in different units is not a same-unit alias.
    let mut c: Configuration = serde_cbor::from_slice(&keys[&Role::Configuration]).unwrap();
    c.owners[1].pacifica_tokens = c.owners[0].bulk_tokens;
    c.owners[1].bulk_tokens = c.owners[0].pacifica_tokens;
    keys.insert(
        Role::Configuration,
        Zeroizing::new(serde_cbor::to_vec(&c).unwrap()),
    );
    crate::application::prepare(&mut m, &keys).unwrap();
    let app = attach(
        &m,
        keys,
        shared(),
        Arc::new(Time(AtomicU64::new(100))),
        Arc::new(AtomicBool::new(false)),
        true,
    )
    .unwrap();
    assert_eq!(
        app.handler
            .journal
            .lock()
            .unwrap()
            .state()
            .unwrap()
            .customers()
            .len(),
        2
    );
}

struct Interrupted {
    memory: Memory,
    time: Arc<Time>,
    stop: Arc<AtomicBool>,
    deadline: u64,
    case: u8,
}
impl Interrupted {
    fn interrupt(&self) {
        if self.case.is_multiple_of(2) {
            self.time.0.store(self.deadline, Ordering::SeqCst);
        } else {
            self.stop.store(true, Ordering::SeqCst);
        }
    }
}
impl Backend for Interrupted {
    fn load(&mut self) -> Result<Vec<Frame>, cinder_journal::Error> {
        let frames = self.memory.load()?;
        if self.case < 2 {
            self.interrupt();
        }
        Ok(frames)
    }
    fn check_current(&mut self, head: Head) -> Result<(), cinder_journal::Error> {
        self.memory.check_current(head)
    }
    fn append(
        &mut self,
        expected: Option<Head>,
        frame: &Frame,
    ) -> Result<(), cinder_journal::Error> {
        self.memory.append(expected, frame)?;
        if self.case >= 2 && frame.head.sequence == 1 {
            self.interrupt();
        }
        Ok(())
    }
}

#[test]
fn startup_lease_and_stop_are_rechecked_after_load_and_between_enrollments() {
    let (mut m, mut keys) = input();
    two_owners(&mut m, &mut keys);
    for case in 0..4 {
        let shared = shared();
        let time = Arc::new(Time(AtomicU64::new(100)));
        let stop = Arc::new(AtomicBool::new(false));
        let backend = Interrupted {
            memory: Memory(shared.clone()),
            time: time.clone(),
            stop: stop.clone(),
            deadline: 100 + m.gates.maximum_boot_ms,
            case,
        };
        assert!(
            loaded(&m, keys.clone())
                .attach(
                    &m,
                    backend,
                    true,
                    Arc::new(Fresh(shared.clone())),
                    time.clone(),
                    stop,
                )
                .is_err()
        );
        assert_eq!(
            shared.frames.lock().unwrap().len(),
            if case < 2 { 1 } else { 2 },
            "no expired/stopped enrollment; case {case}"
        );
        // A new authorized boot resumes the accepted prefix, never resets it.
        time.0
            .store(100 + m.gates.maximum_boot_ms, Ordering::SeqCst);
        let app = attach(
            &m,
            keys.clone(),
            shared.clone(),
            time,
            Arc::new(AtomicBool::new(false)),
            false,
        )
        .unwrap();
        assert_eq!(shared.frames.lock().unwrap().len(), 3);
        assert_eq!(
            app.handler
                .journal
                .lock()
                .unwrap()
                .state()
                .unwrap()
                .customers()
                .len(),
            2
        );
    }
}

#[test]
fn measured_contract_has_exact_roles_ports_and_no_activation_or_legacy_fallback() {
    let (m, keys) = input();
    m.validate().unwrap();
    for (r, body) in &keys {
        assert!(m.wrap(*r, body).unwrap().starts_with(b"CKR7"));
    }
    assert!(m.wrap(Role::Broker, &[1; 32]).is_err());
    for change in 0..10 {
        let mut bad = m.clone();
        match change {
            0 => bad.units = None,
            1 => bad.units.as_mut().unwrap().bulk = [50; 32],
            2 => bad.units.as_mut().unwrap().bulk_port = bad.venue_port,
            3 => bad.units.as_mut().unwrap().bulk_root_hash = [4; 32],
            4 => bad.slots[0].role = Role::Broker,
            5 => bad.slots.pop().map(|_| ()).unwrap(),
            6 => bad.gates.trading = true,
            7 => bad.gates.funding = true,
            8 => bad.gates.native_reads = true,
            _ => bad.history = None,
        }
        assert!(bad.validate().is_err(), "case {change}");
    }
    let mut legacy = crate::boot::qualification_manifest();
    let encoded = serde_cbor::to_vec(&legacy).unwrap();
    let serde_cbor::Value::Map(fields) = serde_cbor::from_slice(&encoded).unwrap() else {
        panic!("manifest map")
    };
    assert!(!fields.contains_key(&serde_cbor::Value::Text("units".into())));
    legacy.units = m.units.clone();
    assert!(legacy.validate().is_err());
    let mut bad = keys.clone();
    bad.insert(
        Role::Configuration,
        Zeroizing::new(b"not legacy or unit CBOR".to_vec()),
    );
    assert!(crate::application::construct(&m, bad).is_err());
}

#[test]
fn actual_controllers_bind_private_configuration_keys_registry_and_egress() {
    let (m, keys) = input();
    for change in 0..10 {
        let mut k = keys.clone();
        let mut manifest = m.clone();
        match change {
            0 => {
                k.insert(Role::BulkTrading, k[&Role::Trading].clone());
            }
            1 => {
                k.insert(Role::BulkTrading, Zeroizing::new(vec![50; 32]));
            }
            2 => {
                k.insert(Role::Trading, Zeroizing::new(vec![9; 32]));
            }
            3 => {
                k.remove(&Role::BulkConfiguration);
            }
            4 => {
                k.insert(Role::Broker, Zeroizing::new(vec![15; 32]));
            }
            5 => {
                manifest.units.as_mut().unwrap().registry = [99; 32];
            }
            6 => {
                manifest.units.as_mut().unwrap().pacifica = [51; 32];
            }
            7 => {
                let mut p: Pacifica =
                    serde_cbor::from_slice(&k[&Role::PacificaConfiguration]).unwrap();
                p.source = 100;
                k.insert(
                    Role::PacificaConfiguration,
                    Zeroizing::new(serde_cbor::to_vec(&p).unwrap()),
                );
            }
            8 => {
                let mut p: Pacifica =
                    serde_cbor::from_slice(&k[&Role::PacificaConfiguration]).unwrap();
                p.execution.execution = pp::Level::Qualified;
                k.insert(
                    Role::PacificaConfiguration,
                    Zeroizing::new(serde_cbor::to_vec(&p).unwrap()),
                );
            }
            _ => {
                let mut c: Configuration =
                    serde_cbor::from_slice(&k[&Role::Configuration]).unwrap();
                c.privacy_nonce = [0; 32];
                k.insert(
                    Role::Configuration,
                    Zeroizing::new(serde_cbor::to_vec(&c).unwrap()),
                );
            }
        }
        assert!(
            crate::application::construct(&manifest, k).is_err(),
            "case {change}"
        );
    }
    let original = loaded(&m, keys.clone()).commitment();
    let mut k = keys;
    k.insert(Role::BulkTrading, Zeroizing::new(vec![13; 32]));
    assert_ne!(loaded(&m, k).commitment(), original);
    let mut changed = m.clone();
    changed.units.as_mut().unwrap().bulk_port += 1;
    assert_ne!(loaded(&changed, input().1).commitment(), original);
    // Identity/role keys never appear in the public measured policy.
    assert!(
        !serde_cbor::to_vec(&m)
            .unwrap()
            .windows(32)
            .any(|s| s == public(9))
    );
}

#[test]
fn one_encrypted_history_reopens_without_reenrollment_or_epoch_reset() {
    let (m, keys) = input();
    let shared = shared();
    let time = Arc::new(Time(AtomicU64::new(100)));
    let app = attach(
        &m,
        keys.clone(),
        shared.clone(),
        time.clone(),
        Arc::new(AtomicBool::new(false)),
        true,
    )
    .unwrap();
    let session = session();
    for unit in [40, 50] {
        let reply = app
            .handler()
            .handle(
                &session,
                request(unit, 20, 1, &session, Command::View, 100),
                100,
            )
            .unwrap();
        assert_view(&reply, unit, 1);
    }
    app.handler()
        .handle(
            &session,
            request(40, 21, 1, &session, Command::Revoke, 100),
            100,
        )
        .unwrap();
    let head = app.handler.journal.lock().unwrap().head();
    app.fence();
    drop(app);
    // Ciphertext does not reveal the owner's public verification key.
    assert!(
        shared.frames.lock().unwrap().iter().all(|f| !f
            .opaque
            .as_bytes()
            .windows(32)
            .any(|s| s == public(9)))
    );
    let app = attach(
        &m,
        keys.clone(),
        shared.clone(),
        time,
        Arc::new(AtomicBool::new(false)),
        false,
    )
    .unwrap();
    assert_eq!(app.handler.journal.lock().unwrap().head(), head);
    for unit in [40, 50] {
        let reply = app
            .handler()
            .handle(
                &session,
                request(unit, 22, 2, &session, Command::View, 100),
                100,
            )
            .unwrap();
        assert_view(&reply, unit, 2);
    }
    let account =
        cinder_journal::onboarding::account(test_support::config().domain, public(9)).unwrap();
    let guard = app.handler.journal.lock().unwrap();
    assert_eq!(guard.state().unwrap().customers()[0].epoch, 2);
    for u in guard.configuration().units() {
        assert_eq!(
            guard
                .state()
                .unwrap()
                .unit(u.id)
                .unwrap()
                .ledger()
                .book(cinder_kernel::ledger::Owner::Customer(account))
                .unwrap()
                .cash()
                .atoms(),
            0
        );
    }
}
fn assert_view(reply: &PrivateBytes, unit: u8, epoch: u64) {
    let prefix = [
        b"CINDER-API-REPLY\0\x00\x02".as_slice(),
        &[unit; 32],
        b"CINDER-API-REPLY\0\x00\x01\x00",
        &epoch.to_be_bytes(),
    ]
    .concat();
    assert!(
        reply.as_bytes().starts_with(&prefix),
        "authenticated unit view"
    );
}

#[test]
fn manifest_substitution_deadline_external_stop_and_independent_epoch_fence() {
    let (m, keys) = input();
    let loaded = loaded(&m, keys.clone());
    let mut changed = m.clone();
    changed.epoch += 1;
    assert!(loaded.matches(&changed).is_err());
    for case in 0..3 {
        let shared = shared();
        let time = Arc::new(Time(AtomicU64::new(100)));
        let stop = Arc::new(AtomicBool::new(false));
        let app = attach(
            &m,
            keys.clone(),
            shared.clone(),
            time.clone(),
            stop.clone(),
            true,
        )
        .unwrap();
        match case {
            0 => time
                .0
                .store(100 + m.gates.maximum_boot_ms, Ordering::SeqCst),
            1 => stop.store(true, Ordering::SeqCst),
            _ => shared.epoch.store(2, Ordering::SeqCst),
        }
        let session = session();
        assert!(
            app.handler()
                .handle(
                    &session,
                    request(40, 20, 1, &session, Command::View, 100),
                    100
                )
                .is_err()
        );
        time.0.store(100, Ordering::SeqCst);
        stop.store(false, Ordering::SeqCst);
        shared.epoch.store(1, Ordering::SeqCst);
        assert!(app.tick().is_err(), "boot fence is sticky");
    }
}

#[test]
fn same_real_handler_is_accepted_by_both_confidential_carriers() {
    let (m, keys) = input();
    let shared = shared();
    let app = attach(
        &m,
        keys,
        shared,
        Arc::new(Time(AtomicU64::new(100))),
        Arc::new(AtomicBool::new(false)),
        true,
    )
    .unwrap();
    let attester = Arc::new(crate::fixture::FixtureAttester::new().unwrap());
    let clock = Arc::new(crate::fixture::FixtureClock);
    let policy = crate::fixture::policy();
    Server::new(
        Identity::generate(clock.as_ref()).unwrap(),
        policy.clone(),
        attester.clone(),
        clock.clone(),
        app.handler(),
    )
    .unwrap();
    crate::web::Server::new(policy, clock.clone(), attester, clock, app.handler()).unwrap();
}

#[test]
fn trusted_preparation_derives_inventory_and_checks_every_plaintext_before_output() {
    let (mut m, keys) = input();
    m.units.as_mut().unwrap().registry = [99; 32];
    m.application = [99; 32];
    crate::application::prepare(&mut m, &keys).unwrap();
    assert_eq!(
        m.units.as_ref().unwrap().registry,
        input().0.units.unwrap().registry
    );
    assert_eq!(
        m.application,
        crate::application::construct(&m, keys.clone())
            .unwrap()
            .commitment()
    );
    for (role, body) in &keys {
        assert!(m.wrap(*role, body).unwrap().len() <= 4096);
    }
    let mut missing = keys.clone();
    missing.remove(&Role::BulkTrading);
    assert!(crate::application::prepare(&mut m.clone(), &missing).is_err());
    let mut oversized = keys.clone();
    let mut witness: Credential = serde_cbor::from_slice(&oversized[&Role::Witness]).unwrap();
    witness.token = "x".repeat(5000);
    oversized.insert(
        Role::Witness,
        Zeroizing::new(serde_cbor::to_vec(&witness).unwrap()),
    );
    assert!(crate::application::prepare(&mut m.clone(), &oversized).is_err());
    // The global unit configuration cannot be interpreted as a legacy config.
    let legacy = crate::boot::qualification_manifest();
    assert!(crate::application::construct(&legacy, keys).is_err());
}

#[test]
fn restore_rejects_changed_recipient_future_time_wrong_key_and_nonempty_create() {
    let (m, keys) = input();
    let shared = shared();
    let time = Arc::new(Time(AtomicU64::new(100)));
    let app = attach(
        &m,
        keys.clone(),
        shared.clone(),
        time.clone(),
        Arc::new(AtomicBool::new(false)),
        true,
    )
    .unwrap();
    app.fence();
    drop(app);
    let head = shared.frames.lock().unwrap().last().unwrap().head;
    for case in 0..5 {
        let mut changed = m.clone();
        let mut k = keys.clone();
        match case {
            0 => {
                let mut c: Configuration =
                    serde_cbor::from_slice(&k[&Role::Configuration]).unwrap();
                c.owners[0].pacifica_tokens = [73; 32];
                k.insert(
                    Role::Configuration,
                    Zeroizing::new(serde_cbor::to_vec(&c).unwrap()),
                );
            }
            1 => time.0.store(99, Ordering::SeqCst),
            2 => {
                k.insert(Role::Storage, Zeroizing::new(vec![43; 32]));
            }
            3 => changed.generation += 1,
            _ => {}
        }
        crate::application::prepare(&mut changed, &k).unwrap();
        assert!(
            attach(
                &changed,
                k,
                shared.clone(),
                time.clone(),
                Arc::new(AtomicBool::new(false)),
                case == 4
            )
            .is_err(),
            "case {case}"
        );
        time.0.store(100, Ordering::SeqCst);
        assert_eq!(shared.frames.lock().unwrap().last().unwrap().head, head);
    }
}
