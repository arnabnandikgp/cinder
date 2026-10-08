//! Offline index/authority regressions with real AEAD records and SQLite. The
//! counted freshness port tests call ordering, not Nitro/witness qualification.
#[path = "../../journal/tests/support/mod.rs"]
mod support;
use super::*;
use cinder_journal::read::Source;
use cinder_journal::{Frame, Head, encrypted::RecordCipher, sqlite::SqliteBackend};
use ed25519_dalek::{Signer, SigningKey};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use support::{Temp, user};
use zeroize::Zeroizing;

#[derive(Default)]
struct Checks {
    freshness: AtomicUsize,
    refuse: AtomicBool,
    uncertain_append: AtomicBool,
}
struct Counted {
    sqlite: SqliteBackend,
    checks: Arc<Checks>,
}
impl Backend for Counted {
    fn load(&mut self) -> Result<Vec<Frame>, cinder_journal::Error> {
        self.sqlite.load()
    }
    fn append(
        &mut self,
        expected: Option<Head>,
        frame: &Frame,
    ) -> Result<(), cinder_journal::Error> {
        self.sqlite.append(expected, frame)?;
        if self.checks.uncertain_append.swap(false, Ordering::SeqCst) {
            Err(cinder_journal::Error::Storage)
        } else {
            Ok(())
        }
    }
    fn check_current(&mut self, expected: Head) -> Result<(), cinder_journal::Error> {
        self.checks.freshness.fetch_add(1, Ordering::SeqCst);
        if self.checks.refuse.load(Ordering::SeqCst) {
            return Err(cinder_journal::Error::Storage);
        }
        self.sqlite.check_current(expected)
    }
}
type Store = Journal<Counted, RecordCipher>;
struct NoOrders;
impl Admission for NoOrders {
    fn order_holds(&self, _: &State, _: &orders::Intent) -> Result<Vec<Reservation>, Error> {
        Err(Error::Unavailable)
    }
}
struct Channel;
impl ConfidentialChannel for Channel {
    fn domain(&self) -> Domain {
        support::config().domain
    }
    fn binding(&self) -> [u8; 32] {
        [8; 32]
    }
    fn expires_at(&self) -> u64 {
        100
    }
}
fn wallet(n: u8) -> SigningKey {
    SigningKey::from_bytes(&[8 + n; 32])
}
fn service() -> Service<NoOrders> {
    Service::new(
        Contract {
            owners: (1..=2)
                .map(|n| OwnerBinding {
                    account: user(n),
                    wallet: wallet(n).verifying_key().to_bytes(),
                    tokens: [80 + n; 32],
                })
                .collect(),
            maximum_auth_lifetime: 100,
            maximum_grant_lifetime: 1000,
        },
        NoOrders,
    )
    .unwrap()
}
fn store(t: &Temp, checks: &Arc<Checks>) -> Store {
    let mut s = Journal::create(
        Counted {
            sqlite: SqliteBackend::create(&t.db).unwrap(),
            checks: Arc::clone(checks),
        },
        RecordCipher::new(Zeroizing::new([99; 32]), 1, [42; 32]).unwrap(),
        support::config(),
    )
    .unwrap();
    let tx = support::transaction(
        s.head(),
        1,
        vec![support::receipt(1, Owner::Customer(user(1)), 100)],
        vec![],
    );
    s.commit(tx).unwrap();
    service().initialize(&mut s, 10).unwrap();
    s
}
fn request(n: u8, customer: u8, epoch: u64, command: Command, signer: &SigningKey) -> Request {
    let mut req = Request {
        domain: support::config().domain,
        account: user(customer),
        id: RequestId::new([n; 32]).unwrap(),
        policy: support::config().policy,
        epoch,
        signer: signer.verifying_key().to_bytes(),
        session: [8; 32],
        expires_at: 100,
        command,
        signature: [0; 64],
    };
    req.signature = signer.sign(&req.message()).to_bytes();
    req
}
fn owner(n: u8, command: Command) -> Request {
    request(n, 1, 1, command, &wallet(1))
}
fn grant(agent: &SigningKey) -> Command {
    Command::Grant(wire::Grant {
        key: agent.verifying_key().to_bytes(),
        methods: wire::READ,
        market: support::q(0).unit().market,
        maximum_lots: 1,
        maximum_fee: 1,
        maximum_orders: 1,
        expires_at: 90,
    })
}
fn call(
    api: &Service<NoOrders>,
    s: &mut Store,
    req: &Request,
    now: u64,
) -> Result<Response, Error> {
    api.handle(s, &Channel, &req.encode().unwrap(), now)
}

#[test]
fn exact_head_reuses_index_but_every_warm_request_checks_freshness() {
    let t = Temp::new();
    let checks = Arc::new(Checks::default());
    let mut s = store(&t, &checks);
    let api = service();
    call(&api, &mut s, &owner(30, grant(&wallet(3))), 10).unwrap();
    let first = api.records(&s).unwrap();
    let n = checks.freshness.load(Ordering::SeqCst);
    for _ in 0..3 {
        call(&api, &mut s, &owner(31, Command::View), 10).unwrap();
        assert!(Arc::ptr_eq(&first, &api.records(&s).unwrap()));
    }
    assert_eq!(checks.freshness.load(Ordering::SeqCst), n + 3);
    checks.refuse.store(true, Ordering::SeqCst);
    assert_eq!(
        call(&api, &mut s, &owner(31, Command::View), 10),
        Err(Error::Unavailable)
    );
    assert!(api.records(&s).is_err());
}

#[test]
fn non_api_head_advances_rebuild_and_contract_changes_never_use_old_index() {
    let t = Temp::new();
    let checks = Arc::new(Checks::default());
    let mut s = store(&t, &checks);
    let mut api = service();
    let old = api.records(&s).unwrap();
    s.commit(support::transaction(s.head(), 50, vec![], vec![]))
        .unwrap();
    let current = api.records(&s).unwrap();
    assert!(!Arc::ptr_eq(&old, &current));
    assert_eq!(current.len(), old.len());
    api.contract.maximum_auth_lifetime += 1;
    assert!(api.records(&s).is_err());
}

#[test]
fn warm_grant_rechecks_expiry_epoch_and_authorization_before_id_lookup() {
    let t = Temp::new();
    let checks = Arc::new(Checks::default());
    let mut s = store(&t, &checks);
    let api = service();
    let agent = wallet(3);
    call(&api, &mut s, &owner(30, grant(&agent)), 10).unwrap();
    let read = request(31, 1, 1, Command::View, &agent);
    call(&api, &mut s, &read, 10).unwrap();
    let cached = api.records(&s).unwrap();
    assert_eq!(call(&api, &mut s, &read, 90), Err(Error::Unauthorized));
    assert!(Arc::ptr_eq(&cached, &api.records(&s).unwrap()));
    let before = checks.freshness.load(Ordering::SeqCst);
    let mut invalid = read.clone();
    invalid.signature[0] ^= 1;
    assert_eq!(call(&api, &mut s, &invalid, 10), Err(Error::Unauthorized));
    assert_eq!(checks.freshness.load(Ordering::SeqCst), before);
    call(&api, &mut s, &owner(32, Command::Revoke), 10).unwrap();
    assert_eq!(call(&api, &mut s, &read, 10), Err(Error::Unauthorized));
    let lookup = request(
        33,
        1,
        2,
        Command::Operation(RequestId::new([30; 32]).unwrap()),
        &agent,
    );
    assert_eq!(call(&api, &mut s, &lookup, 10), Err(Error::Unauthorized));
    let reconnect = request(34, 1, 2, Command::View, &wallet(1));
    call(&api, &mut s, &reconnect, 10).unwrap();
    assert!(!Arc::ptr_eq(&cached, &api.records(&s).unwrap()));
}

#[test]
fn independent_projection_keeps_grant_expiry_mutation_exclusion_and_contract_binding() {
    let t = Temp::new();
    let checks = Arc::new(Checks::default());
    let mut s = store(&t, &checks);
    let mut api = service();
    let agent = wallet(3);
    call(&api, &mut s, &owner(30, grant(&agent)), 10).unwrap();
    let binding = cinder_journal::read::Binding {
        stream: cinder_journal::replicated::Stream {
            domain: s.configuration().domain,
            id: [42; 32],
        },
        epoch: 1,
        api: api.release_commitment(s.configuration()).unwrap(),
    };
    let reader = s.attach_reader(binding).unwrap();
    struct Witness(cinder_journal::Head);
    impl cinder_journal::read::Witness for Witness {
        fn read(
            &self,
            _: cinder_journal::replicated::Stream,
        ) -> Result<cinder_journal::replicated::Anchor, cinder_journal::Error> {
            Ok(cinder_journal::replicated::Anchor {
                epoch: 1,
                head: Some(self.0),
            })
        }
    }
    let v = reader
        .verify(reader.capture().unwrap(), &Witness(s.head()))
        .unwrap();
    let read = request(31, 1, 1, Command::View, &agent).encode().unwrap();
    assert!(api.handle_read(&v, &Channel, &read, 10).is_ok());
    assert!(api.revalidate_read(&v, &Channel, &read, 89).is_ok());
    assert_eq!(
        api.revalidate_read(&v, &Channel, &read, 90),
        Err(Error::Unauthorized)
    );
    let mutation = owner(32, Command::Revoke).encode().unwrap();
    assert_eq!(
        api.authenticate_read(v.configuration(), &Channel, &mutation, 10),
        Err(Error::Invalid)
    );
    assert_eq!(
        api.handle_read(&v, &Channel, &mutation, 10),
        Err(Error::Invalid)
    );
    api.contract.maximum_auth_lifetime += 1;
    assert_eq!(
        api.handle_read(&v, &Channel, &read, 10),
        Err(Error::Unavailable)
    );
}

#[test]
fn unknown_acceptance_cannot_serve_warm_index_and_replay_restores_original_operation() {
    let t = Temp::new();
    let checks = Arc::new(Checks::default());
    let mut s = store(&t, &checks);
    let api = service();
    let old = api.records(&s).unwrap();
    let req = owner(30, grant(&wallet(3)));
    checks.uncertain_append.store(true, Ordering::SeqCst);
    assert_eq!(call(&api, &mut s, &req, 10), Err(Error::Unavailable));
    assert_eq!(
        call(&api, &mut s, &owner(31, Command::View), 10),
        Err(Error::Unavailable)
    );
    assert!(api.records(&s).is_err());
    s.reload().unwrap();
    let rebuilt = api.records(&s).unwrap();
    assert!(!Arc::ptr_eq(&old, &rebuilt));
    assert_eq!(rebuilt.len(), 1);
    let lookup = owner(31, Command::Operation(req.id));
    let recovered = call(&api, &mut s, &lookup, 10).unwrap();
    assert_eq!(recovered, call(&service(), &mut s, &lookup, 10).unwrap());
    let head = s.head();
    assert_eq!(call(&api, &mut s, &req, 10).unwrap(), recovered);
    assert_eq!(s.head(), head);
    assert!(s.state().unwrap().attempts().is_empty());
}

#[test]
fn malformed_new_history_refuses_instead_of_falling_back_to_warm_index() {
    let t = Temp::new();
    let checks = Arc::new(Checks::default());
    let mut s = store(&t, &checks);
    let api = service();
    let _warm = api.records(&s).unwrap();
    let fingerprint = api.release_commitment(s.configuration()).unwrap();
    let mut tx = support::transaction(s.head(), 50, vec![], vec![]);
    tx.evidence
        .push(PrivateBytes::new([RECORD, &fingerprint, b"malformed"].concat()).unwrap());
    s.commit(tx).unwrap();
    assert_eq!(
        call(&api, &mut s, &owner(31, Command::View), 10),
        Err(Error::Unavailable)
    );
    assert!(api.records(&s).is_err());
}

#[test]
fn identical_request_ids_remain_account_scoped_and_poisoned_index_latch_fails_closed() {
    let t = Temp::new();
    let checks = Arc::new(Checks::default());
    let mut s = store(&t, &checks);
    let api = service();
    let first = owner(30, grant(&wallet(3)));
    let second = request(30, 2, 1, grant(&wallet(4)), &wallet(2));
    call(&api, &mut s, &first, 10).unwrap();
    call(&api, &mut s, &second, 10).unwrap();
    let index = api.records(&s).unwrap();
    assert_eq!(index.find(user(1), first.id).unwrap().request, first);
    assert_eq!(index.find(user(2), second.id).unwrap().request, second);
    assert_eq!(
        index.grant(&request(31, 2, 1, Command::View, &wallet(3))),
        None
    );
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = api.records.lock().unwrap();
        panic!("test-only poison");
    }));
    assert_eq!(
        call(&api, &mut s, &owner(31, Command::View), 10),
        Err(Error::Unavailable)
    );
}
