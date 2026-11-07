//! Disposable local identities; no network, wallet file or native funds.
#[path = "../../journal/tests/support/mod.rs"]
mod support;
use cinder_api::{
    onboarding::{self, Command, Request},
    *,
};
use cinder_journal::{
    Journal,
    encrypted::RecordCipher,
    model::*,
    orders,
    sqlite::{Migration, SqliteBackend},
};
use cinder_kernel::{
    identity::*,
    ledger::{Config, Owner},
};
use ed25519_dalek::{Signer, SigningKey};
use support::Temp;
use zeroize::Zeroizing;
struct NoRisk;
impl Admission for NoRisk {
    fn order_holds(&self, _: &State, _: &orders::Intent) -> Result<Vec<Reservation>, Error> {
        Err(Error::Unavailable)
    }
}
struct Channel([u8; 32]);
impl ConfidentialChannel for Channel {
    fn domain(&self) -> Domain {
        config().domain
    }
    fn binding(&self) -> [u8; 32] {
        self.0
    }
    fn expires_at(&self) -> u64 {
        1000
    }
}
fn config() -> Config {
    let mut c = support::config();
    c.customers.clear();
    c
}
fn cipher() -> RecordCipher {
    RecordCipher::new(Zeroizing::new([66; 32]), 1, [67; 32]).unwrap()
}
fn policy() -> onboarding::Policy {
    // Independently generated canonical Solana PDA fixture; client crosscheck owns derivation.
    onboarding::Policy {
        maximum_accounts: 2,
        custody_domain: [2; 32],
        pool: [10; 32],
        venue: [4; 32],
        program: [11; 32],
        config: CONFIG,
        route: ROUTE,
        mint: [12; 32],
        decimals: 6,
        epoch: 1,
        maximum_deposit: 1000,
    }
}
const CONFIG: [u8; 32] = [
    10, 202, 37, 42, 249, 75, 148, 198, 48, 45, 50, 92, 230, 197, 7, 221, 20, 232, 86, 24, 60, 205,
    12, 217, 151, 232, 29, 77, 218, 50, 205, 210,
];
const ROUTE: [u8; 32] = [
    231, 173, 79, 188, 30, 16, 149, 159, 72, 54, 204, 49, 176, 119, 81, 163, 144, 146, 225, 161,
    33, 132, 0, 59, 30, 78, 157, 47, 203, 114, 35, 132,
];
fn service() -> Service<NoRisk> {
    Service::new_onboarding(
        Contract {
            owners: vec![],
            maximum_auth_lifetime: 1000,
            maximum_grant_lifetime: 10000,
        },
        NoRisk,
        policy(),
    )
    .unwrap()
}
fn request(key: &SigningKey, session: [u8; 32], command: Command) -> Request {
    let mut r = Request {
        domain: config().domain,
        policy: config().policy,
        wallet: key.verifying_key().to_bytes(),
        session,
        expires_at: 900,
        command,
        signature: [0; 64],
    };
    r.signature = key.sign(&r.message()).to_bytes();
    r
}
fn open(
    service: &Service<NoRisk>,
    store: &mut Journal<SqliteBackend, RecordCipher>,
    key: &SigningKey,
    at: u64,
) -> onboarding::Discovery {
    match service
        .handle(
            store,
            &Channel([8; 32]),
            &request(key, [8; 32], Command::Open).encode().unwrap(),
            at,
        )
        .unwrap()
    {
        Response::Discovery(d) => *d,
        _ => panic!("wrong response"),
    }
}
#[test]
fn fresh_wallet_registration_is_empty_idempotent_private_and_survives_restart() {
    let t = Temp::new();
    let s = service();
    let mut j = Journal::create(SqliteBackend::create(&t.db).unwrap(), cipher(), config()).unwrap();
    s.initialize(&mut j, 1).unwrap();
    let a = SigningKey::from_bytes(&[33; 32]);
    let b = SigningKey::from_bytes(&[34; 32]);
    let version = j.state().unwrap().ledger().version();
    let d = open(&s, &mut j, &a, 2);
    assert_eq!(j.state().unwrap().ledger().version(), version);
    let head = j.head();
    assert_eq!(open(&s, &mut j, &a, 3).account, d.account);
    assert_eq!(j.head(), head);
    assert_eq!(d.epoch, 1);
    assert_eq!(
        j.state()
            .unwrap()
            .ledger()
            .book(Owner::Customer(d.account))
            .unwrap()
            .cash()
            .atoms(),
        0
    );
    assert_eq!(j.state().unwrap().ledger().vault().atoms(), 0);
    assert_eq!(j.configuration().customers.len(), 0);
    let e = open(&s, &mut j, &b, 4);
    assert_ne!(d.account, e.account);
    assert_ne!(d.recipient, e.recipient);
    drop(j);
    let mut j = Journal::open(
        SqliteBackend::open(&t.db, Migration::None).unwrap(),
        cipher(),
        config(),
    )
    .unwrap();
    s.initialize(&mut j, 5).unwrap();
    assert_eq!(open(&s, &mut j, &a, 6).account, d.account);
    assert_eq!(j.state().unwrap().ledger().customer_ids().count(), 2);
    let c = SigningKey::from_bytes(&[35; 32]);
    assert_eq!(
        s.handle(
            &mut j,
            &Channel([8; 32]),
            &request(&c, [8; 32], Command::Open).encode().unwrap(),
            7
        ),
        Err(Error::Unavailable)
    );
}
#[test]
fn invalid_owner_proofs_and_cross_session_replays_never_register_or_disclose() {
    let t = Temp::new();
    let s = service();
    let mut j = Journal::create(SqliteBackend::create(&t.db).unwrap(), cipher(), config()).unwrap();
    s.initialize(&mut j, 1).unwrap();
    let a = SigningKey::from_bytes(&[33; 32]);
    let good = request(&a, [8; 32], Command::Open);
    let head = j.head();
    for n in 0..6 {
        let mut r = good.clone();
        match n {
            0 => r.session = [9; 32],
            1 => r.policy = PolicyVersion::new(2).unwrap(),
            2 => r.domain.deployment = DeploymentId::new([9; 32]).unwrap(),
            3 => r.wallet = SigningKey::from_bytes(&[34; 32]).verifying_key().to_bytes(),
            4 => r.expires_at = 2,
            _ => r.signature[0] ^= 1,
        };
        if n < 3 || n == 4 {
            r.signature = a.sign(&r.message()).to_bytes();
        }
        assert!(
            s.handle(&mut j, &Channel([8; 32]), &r.encode().unwrap(), 2)
                .is_err()
        );
        assert_eq!(j.head(), head);
    }
    let d = open(&s, &mut j, &a, 3);
    let mut r = wire::Request {
        domain: config().domain,
        account: d.account,
        id: RequestId::new([55; 32]).unwrap(),
        policy: config().policy,
        epoch: 1,
        signer: SigningKey::from_bytes(&[34; 32]).verifying_key().to_bytes(),
        session: [8; 32],
        expires_at: 900,
        command: wire::Command::View,
        signature: [0; 64],
    };
    r.signature = SigningKey::from_bytes(&[34; 32])
        .sign(&r.message())
        .to_bytes();
    assert_eq!(
        s.handle(&mut j, &Channel([8; 32]), &r.encode().unwrap(), 4),
        Err(Error::Unauthorized)
    );
}
#[test]
fn signed_deposit_notice_is_a_durable_hint_not_money_and_conflicts_do_not_replace_it() {
    let t = Temp::new();
    let s = service();
    let mut j = Journal::create(SqliteBackend::create(&t.db).unwrap(), cipher(), config()).unwrap();
    s.initialize(&mut j, 1).unwrap();
    let a = SigningKey::from_bytes(&[33; 32]);
    let d = open(&s, &mut j, &a, 2);
    let command = Command::Deposit {
        epoch: 1,
        operation: [44; 32],
        signature: [45; 64],
    };
    let send = |j: &mut Journal<SqliteBackend, RecordCipher>, c, at| {
        s.handle(
            j,
            &Channel([8; 32]),
            &request(&a, [8; 32], c).encode().unwrap(),
            at,
        )
    };
    send(&mut j, command.clone(), 3).unwrap();
    let head = j.head();
    send(&mut j, command, 4).unwrap();
    assert_eq!(j.head(), head);
    assert_eq!(j.state().unwrap().deposit_notices().len(), 1);
    assert!(j.state().unwrap().escrows().is_empty());
    assert_eq!(
        j.state()
            .unwrap()
            .ledger()
            .book(Owner::Customer(d.account))
            .unwrap()
            .cash()
            .atoms(),
        0
    );
    assert_eq!(
        send(
            &mut j,
            Command::Deposit {
                epoch: 1,
                operation: [44; 32],
                signature: [46; 64]
            },
            5
        ),
        Err(Error::Conflict)
    );
    assert_eq!(
        send(
            &mut j,
            Command::Deposit {
                epoch: 2,
                operation: [47; 32],
                signature: [48; 64]
            },
            5
        ),
        Err(Error::Unauthorized)
    );
    assert_eq!(j.head(), head);
}

#[test]
fn another_owner_cannot_squat_a_public_deposit_signature() {
    let t = Temp::new();
    let s = service();
    let mut j = Journal::create(SqliteBackend::create(&t.db).unwrap(), cipher(), config()).unwrap();
    s.initialize(&mut j, 1).unwrap();
    let a = SigningKey::from_bytes(&[33; 32]);
    let b = SigningKey::from_bytes(&[34; 32]);
    let alice = open(&s, &mut j, &a, 2);
    let bob = open(&s, &mut j, &b, 3);
    for (key, at) in [(&b, 4), (&a, 5)] {
        let r = request(
            key,
            [8; 32],
            Command::Deposit {
                epoch: 1,
                operation: [44; 32],
                signature: [45; 64],
            },
        );
        assert!(matches!(
            s.handle(&mut j, &Channel([8; 32]), &r.encode().unwrap(), at),
            Ok(Response::Discovery(_))
        ));
    }
    assert_eq!(j.state().unwrap().deposit_notices().len(), 2);
    for (key, account, at) in [(&a, alice.account, 6), (&b, bob.account, 7)] {
        assert_eq!(open(&s, &mut j, key, at).pending_deposits, vec![[44; 32]]);
        assert_eq!(
            j.state()
                .unwrap()
                .ledger()
                .book(Owner::Customer(account))
                .unwrap()
                .cash()
                .atoms(),
            0
        );
    }
    assert!(j.state().unwrap().escrows().is_empty());
}

#[test]
fn concurrent_registration_loser_reloads_the_same_account_without_a_second_book() {
    let t = Temp::new();
    let s = service();
    let mut first =
        Journal::create(SqliteBackend::create(&t.db).unwrap(), cipher(), config()).unwrap();
    s.initialize(&mut first, 1).unwrap();
    let mut stale = Journal::open(
        SqliteBackend::open(&t.db, Migration::None).unwrap(),
        cipher(),
        config(),
    )
    .unwrap();
    let a = SigningKey::from_bytes(&[33; 32]);
    let d = open(&s, &mut first, &a, 2);
    assert_eq!(
        s.handle(
            &mut stale,
            &Channel([8; 32]),
            &request(&a, [8; 32], Command::Open).encode().unwrap(),
            3
        ),
        Err(Error::Unavailable)
    );
    drop(stale);
    let mut resumed = Journal::open(
        SqliteBackend::open(&t.db, Migration::None).unwrap(),
        cipher(),
        config(),
    )
    .unwrap();
    let head = resumed.head();
    assert_eq!(open(&s, &mut resumed, &a, 4).account, d.account);
    assert_eq!(resumed.head(), head);
    assert_eq!(resumed.state().unwrap().registrations().len(), 1);
    assert_eq!(resumed.state().unwrap().ledger().vault().atoms(), 0);
}

#[test]
fn uncertain_registration_commit_recovers_without_reset_or_duplicate_enrollment() {
    use cinder_journal::{Backend, Error as JournalError, Frame, Head};
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    struct Uncertain {
        inner: SqliteBackend,
        fail: Arc<AtomicBool>,
    }
    impl Backend for Uncertain {
        fn load(&mut self) -> Result<Vec<Frame>, JournalError> {
            self.inner.load()
        }
        fn append(&mut self, expected: Option<Head>, frame: &Frame) -> Result<(), JournalError> {
            self.inner.append(expected, frame)?;
            if self.fail.swap(false, Ordering::SeqCst) {
                Err(JournalError::Storage)
            } else {
                Ok(())
            }
        }
    }
    let t = Temp::new();
    let s = service();
    let fail = Arc::new(AtomicBool::new(false));
    let mut j = Journal::create(
        Uncertain {
            inner: SqliteBackend::create(&t.db).unwrap(),
            fail: fail.clone(),
        },
        cipher(),
        config(),
    )
    .unwrap();
    s.initialize(&mut j, 1).unwrap();
    let a = SigningKey::from_bytes(&[33; 32]);
    let original = request(&a, [8; 32], Command::Open).encode().unwrap();
    fail.store(true, Ordering::SeqCst);
    assert_eq!(
        s.handle(&mut j, &Channel([8; 32]), &original, 2),
        Err(Error::Unavailable)
    );
    assert!(j.state().is_err());
    drop(j);
    let mut j = Journal::open(
        SqliteBackend::open(&t.db, Migration::None).unwrap(),
        cipher(),
        config(),
    )
    .unwrap();
    let head = j.head();
    let d = open(&s, &mut j, &a, 3);
    assert_eq!(d.epoch, 1);
    assert_eq!(j.head(), head);
    assert_eq!(j.state().unwrap().registrations().len(), 1);
    assert_eq!(
        j.state()
            .unwrap()
            .ledger()
            .book(Owner::Customer(d.account))
            .unwrap()
            .cash()
            .atoms(),
        0
    );
}

#[test]
fn revoke_advances_epoch_and_rediscovery_never_resets_it_or_restores_agent_authority() {
    let t = Temp::new();
    let s = service();
    let mut j = Journal::create(SqliteBackend::create(&t.db).unwrap(), cipher(), config()).unwrap();
    s.initialize(&mut j, 1).unwrap();
    let a = SigningKey::from_bytes(&[33; 32]);
    let d = open(&s, &mut j, &a, 2);
    let mut r = wire::Request {
        domain: config().domain,
        account: d.account,
        id: RequestId::new([99; 32]).unwrap(),
        policy: config().policy,
        epoch: 1,
        signer: a.verifying_key().to_bytes(),
        session: [8; 32],
        expires_at: 900,
        command: wire::Command::Revoke,
        signature: [0; 64],
    };
    r.signature = a.sign(&r.message()).to_bytes();
    s.handle(&mut j, &Channel([8; 32]), &r.encode().unwrap(), 3)
        .unwrap();
    let head = j.head();
    assert_eq!(open(&s, &mut j, &a, 4).epoch, 2);
    assert_eq!(j.head(), head);
    assert_eq!(
        s.handle(
            &mut j,
            &Channel([8; 32]),
            &request(
                &a,
                [8; 32],
                Command::Deposit {
                    epoch: 1,
                    operation: [55; 32],
                    signature: [56; 64]
                }
            )
            .encode()
            .unwrap(),
            5
        ),
        Err(Error::Unauthorized)
    );
}

#[test]
fn existing_owner_deposit_hints_remain_recordable_during_freeze_and_recovery() {
    for mode in 0..3 {
        let t = Temp::new();
        let s = service();
        let mut j =
            Journal::create(SqliteBackend::create(&t.db).unwrap(), cipher(), config()).unwrap();
        s.initialize(&mut j, 1).unwrap();
        let a = SigningKey::from_bytes(&[33; 32]);
        let d = open(&s, &mut j, &a, 2);
        let mut tx = support::transaction(j.head(), 70, vec![], vec![]);
        tx.at = 3;
        if mode == 0 {
            tx.controls
                .push(Control::Funds(cinder_journal::funds::Action::Freeze));
        } else {
            tx.controls
                .push(Control::Recovery(cinder_journal::recovery::Action::Begin {
                    expected_version: j.state().unwrap().ledger().version(),
                    authority_epoch: 9,
                    valid_until: 900,
                    fence: [71; 32],
                }));
            if mode == 2 {
                tx.controls
                    .push(Control::Recovery(cinder_journal::recovery::Action::Seal {
                        expected_version: j.state().unwrap().ledger().version(),
                    }));
            }
        }
        assert_eq!(j.commit(tx).unwrap().receipt.controls, None);
        let ledger = j.state().unwrap().ledger().clone();
        let r = request(
            &a,
            [8; 32],
            Command::Deposit {
                epoch: d.epoch,
                operation: [72; 32],
                signature: [73; 64],
            },
        );
        assert!(matches!(
            s.handle(&mut j, &Channel([8; 32]), &r.encode().unwrap(), 4),
            Ok(Response::Discovery(_))
        ));
        assert_eq!(j.state().unwrap().ledger(), &ledger);
        assert!(j.state().unwrap().escrows().is_empty());
        let head = j.head();
        s.handle(&mut j, &Channel([8; 32]), &r.encode().unwrap(), 5)
            .unwrap();
        assert_eq!(j.head(), head);
        let b = SigningKey::from_bytes(&[34; 32]);
        assert!(
            s.handle(
                &mut j,
                &Channel([8; 32]),
                &request(&b, [8; 32], Command::Open).encode().unwrap(),
                6
            )
            .is_err()
        );
        drop(j);
        let mut j = Journal::open(
            SqliteBackend::open(&t.db, Migration::None).unwrap(),
            cipher(),
            config(),
        )
        .unwrap();
        assert_eq!(open(&s, &mut j, &a, 7).pending_deposits, vec![[72; 32]]);
    }
}

#[test]
fn freeze_and_complete_recovery_key_registry_refuse_new_accounts_without_hiding_existing_owners() {
    for registry in [false, true] {
        let t = Temp::new();
        let s = service();
        let mut j =
            Journal::create(SqliteBackend::create(&t.db).unwrap(), cipher(), config()).unwrap();
        s.initialize(&mut j, 1).unwrap();
        let a = SigningKey::from_bytes(&[33; 32]);
        let d = open(&s, &mut j, &a, 2);
        let mut tx = support::transaction(j.head(), 70, vec![], vec![]);
        tx.at = 3;
        if registry {
            tx.evidence.push(
                PrivateBytes::new(
                    b"CINDER_RECOVERY_KEYS_V1\0fixture complete enrollment seal".to_vec(),
                )
                .unwrap(),
            );
        } else {
            tx.controls
                .push(Control::Funds(cinder_journal::funds::Action::Freeze));
        }
        assert_eq!(j.commit(tx).unwrap().receipt.controls, None);
        let b = SigningKey::from_bytes(&[34; 32]);
        assert_eq!(
            s.handle(
                &mut j,
                &Channel([8; 32]),
                &request(&b, [8; 32], Command::Open).encode().unwrap(),
                4
            ),
            Err(Error::Unavailable)
        );
        assert_eq!(j.state().unwrap().registrations().len(), 1);
        assert_eq!(open(&s, &mut j, &a, 5).account, d.account);
    }
}
