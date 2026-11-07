//! Synthetic owner signatures and local encrypted storage; no external calls.
#[path = "../../journal/tests/support/mod.rs"]
mod support;
use cinder_api::{
    onboarding as old,
    units::{self as api, onboarding as bootstrap},
    *,
};
use cinder_journal::{
    encrypted::RecordCipher,
    model::*,
    orders,
    sqlite::{Migration, SqliteBackend},
    units,
};
use cinder_kernel::{identity::*, ledger::Owner};
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;
type Store = units::Journal<SqliteBackend, RecordCipher>;
fn uid(n: u8) -> UnitId {
    UnitId::new([n; 32]).unwrap()
}
fn pda(program: [u8; 32], seeds: &[&[u8]]) -> [u8; 32] {
    for bump in (0..=255_u8).rev() {
        let mut h = Sha256::new();
        for s in seeds {
            h.update(s);
        }
        h.update([bump]);
        h.update(program);
        h.update(b"ProgramDerivedAddress");
        let key = h.finalize().into();
        if VerifyingKey::from_bytes(&key).is_err() {
            return key;
        }
    }
    panic!("synthetic PDA");
}
fn fixture() -> (units::Configuration, Vec<(UnitId, old::Policy)>) {
    let domain = support::config().domain;
    let mut descriptors = vec![];
    let mut policies = vec![];
    for n in [40_u8, 50] {
        let mut ledger = support::config();
        ledger.customers.clear();
        ledger.venue = VenueId::new([n + 1; 32]).unwrap();
        ledger.venue_account = VenueAccountId::new([n + 2; 32]).unwrap();
        for s in &mut ledger.sources {
            s.scope.venue = ledger.venue;
            s.scope.account = ledger.venue_account;
        }
        let program = [11; 32];
        let pool = [n; 32];
        let mint = [n + 3; 32];
        let config = pda(
            program,
            &[b"cinder_vault", &domain.deployment.bytes(), &pool, &mint],
        );
        let route = pda(program, &[b"venue_route", &config]);
        descriptors.push(units::Descriptor {
            id: uid(n),
            environment: [90; 32],
            native_account: [n + 4; 32],
            funding_wallet: [n + 5; 32],
            custody_config: config,
            payout_vault: pda(program, &[b"tokens", &config]),
            key_policy: [n + 7; 32],
            ledger,
        });
        policies.push((
            uid(n),
            old::Policy {
                maximum_accounts: 2,
                custody_domain: domain.deployment.bytes(),
                pool,
                venue: [n + 1; 32],
                program,
                config,
                route,
                mint,
                decimals: 0,
                epoch: 1,
                maximum_deposit: 1000,
            },
        ));
    }
    (
        units::Configuration::new(domain, descriptors).unwrap(),
        policies,
    )
}
struct NoRisk;
impl Admission for NoRisk {
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
        1000
    }
}
fn cipher() -> RecordCipher {
    RecordCipher::new(Zeroizing::new([66; 32]), 1, [67; 32]).unwrap()
}
fn service() -> (api::service::Service<NoRisk>, [u8; 32]) {
    let (cfg, p) = fixture();
    let c = bootstrap::Contract::new(cfg, p).unwrap();
    let hash = c.commitment();
    (
        api::service::Service::new(api::Authorizer::new(1000, 10000).unwrap(), NoRisk)
            .with_onboarding(c),
        hash,
    )
}
fn request(
    unit: u8,
    contract: [u8; 32],
    key: &SigningKey,
    command: old::Command,
) -> bootstrap::Request {
    let mut r = bootstrap::Request {
        unit: uid(unit),
        contract,
        body: old::Request {
            domain: support::config().domain,
            policy: support::config().policy,
            wallet: key.verifying_key().to_bytes(),
            session: [8; 32],
            expires_at: 900,
            command,
            signature: [0; 64],
        },
    };
    r.body.signature = key.sign(&r.message()).to_bytes();
    r
}
fn open(
    s: &api::service::Service<NoRisk>,
    j: &mut Store,
    r: &bootstrap::Request,
    now: u64,
) -> old::Discovery {
    let (unit, result) = s.onboard(j, &Channel, &r.encode().unwrap(), now).unwrap();
    assert_eq!(unit, r.unit);
    if let Response::Discovery(d) = result {
        *d
    } else {
        panic!("discovery")
    }
}
#[test]
fn owner_enrolls_all_units_once_with_canonical_recipients_and_original_verified_proof() {
    let t = support::Temp::new();
    let (s, contract) = service();
    let key = SigningKey::from_bytes(&[33; 32]);
    let mut j =
        Store::create(SqliteBackend::create(&t.db).unwrap(), cipher(), fixture().0).unwrap();
    s.initialize(&mut j).unwrap();
    let r = request(40, contract, &key, old::Command::Open);
    let a = open(&s, &mut j, &r, 10);
    assert_eq!(a.epoch, 1);
    let head = j.head();
    let b = open(
        &s,
        &mut j,
        &request(50, contract, &key, old::Command::Open),
        11,
    );
    assert_eq!(a.account, b.account);
    assert_ne!(a.recipient, b.recipient);
    assert_eq!(j.head(), head);
    assert!(
        matches!(&j.transactions().unwrap().next().unwrap().0.change,units::Change::EnrollOwner{proof,..} if proof==&r.encode().unwrap())
    );
    for n in [40, 50] {
        let state = j.state().unwrap().unit(uid(n)).unwrap();
        assert_eq!(
            state
                .ledger()
                .book(Owner::Customer(a.account))
                .unwrap()
                .cash()
                .atoms(),
            0
        );
    }
    drop(j);
    let mut j = Store::open(
        SqliteBackend::open(&t.db, Migration::None).unwrap(),
        cipher(),
        fixture().0,
    )
    .unwrap();
    s.initialize(&mut j).unwrap();
    assert_eq!(open(&s, &mut j, &r, 12).account, a.account);
}
#[test]
fn bootstrap_refuses_v1_signatures_changed_unit_contract_session_and_route_inventory() {
    let t = support::Temp::new();
    let (s, contract) = service();
    let key = SigningKey::from_bytes(&[33; 32]);
    let mut j =
        Store::create(SqliteBackend::create(&t.db).unwrap(), cipher(), fixture().0).unwrap();
    let head = j.head();
    let mut r = request(40, contract, &key, old::Command::Open);
    r.body.signature = key.sign(&r.body.message()).to_bytes();
    assert!(
        s.onboard(&mut j, &Channel, &r.encode().unwrap(), 10)
            .is_err()
    );
    for mutate in [0, 1, 2] {
        let mut r = request(40, contract, &key, old::Command::Open);
        match mutate {
            0 => r.unit = uid(50),
            1 => r.contract = [7; 32],
            _ => r.body.session = [7; 32],
        };
        assert!(
            s.onboard(&mut j, &Channel, &r.encode().unwrap(), 10)
                .is_err()
        );
    }
    assert_eq!(j.head(), head);
    let (cfg, mut p) = fixture();
    p.swap(0, 1);
    assert!(bootstrap::Contract::new(cfg, p).is_err());
}
#[test]
fn original_deposit_notices_and_global_revoke_remain_scoped_without_credit() {
    let t = support::Temp::new();
    let (s, contract) = service();
    let key = SigningKey::from_bytes(&[33; 32]);
    let mut j =
        Store::create(SqliteBackend::create(&t.db).unwrap(), cipher(), fixture().0).unwrap();
    let owner = open(
        &s,
        &mut j,
        &request(40, contract, &key, old::Command::Open),
        10,
    );
    let notice = || old::Command::Deposit {
        epoch: 1,
        operation: [80; 32],
        signature: [81; 64],
    };
    open(&s, &mut j, &request(40, contract, &key, notice()), 11);
    assert!(
        j.state()
            .unwrap()
            .unit(uid(50))
            .unwrap()
            .deposit_notices()
            .is_empty()
    );
    let head = j.head();
    open(&s, &mut j, &request(40, contract, &key, notice()), 12);
    assert_eq!(j.head(), head);
    j.commit(units::Transaction {
        id: CommitId::new([90; 32]).unwrap(),
        expected: j.head(),
        at: 13,
        change: units::Change::AdvanceAuthority {
            account: owner.account,
            epoch: 2,
        },
    })
    .unwrap();
    assert!(
        s.onboard(
            &mut j,
            &Channel,
            &request(50, contract, &key, notice()).encode().unwrap(),
            14
        )
        .is_err()
    );
    assert_eq!(
        open(
            &s,
            &mut j,
            &request(50, contract, &key, old::Command::Open),
            14
        )
        .epoch,
        2
    );
    for n in [40, 50] {
        assert_eq!(
            j.state()
                .unwrap()
                .unit(uid(n))
                .unwrap()
                .ledger()
                .book(Owner::Customer(owner.account))
                .unwrap()
                .cash()
                .atoms(),
            0
        );
    }
}
