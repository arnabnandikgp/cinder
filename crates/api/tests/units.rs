//! Synthetic unit scopes/accepted-grant port; no runnable v2 service or native I/O.
#[path = "units/admission.rs"]
mod admission_tests;
#[path = "../../journal/tests/support/mod.rs"]
mod support;
use cinder_api::{ConfidentialChannel, Error, Response, units as api, wire};
use cinder_journal::{
    Backend, Frame, Head,
    encrypted::RecordCipher,
    model::*,
    onboarding,
    sqlite::{Migration, SqliteBackend},
    units,
};
use cinder_kernel::{identity::*, ledger::Owner};
use ed25519_dalek::{Signature, Signer, SigningKey};
use sha2::{Digest, Sha256};
use std::{
    cell::Cell,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use support::Temp;
use zeroize::Zeroizing;

fn key(n: u8) -> SigningKey {
    SigningKey::from_bytes(&[n; 32])
}
fn uid(n: u8) -> UnitId {
    UnitId::new([n; 32]).unwrap()
}
fn domain() -> Domain {
    support::config().domain
}
fn account() -> AccountId {
    onboarding::account(domain(), key(9).verifying_key().to_bytes()).unwrap()
}
fn config() -> units::Configuration {
    let descriptors = [40_u8, 50]
        .into_iter()
        .map(|n| {
            let mut ledger = support::config();
            ledger.customers.clear();
            ledger.venue = VenueId::new([n + 4; 32]).unwrap();
            ledger.venue_account = VenueAccountId::new([n + 5; 32]).unwrap();
            ledger.sources[0].scope.venue = ledger.venue;
            ledger.sources[0].scope.account = ledger.venue_account;
            units::Descriptor {
                id: uid(n),
                environment: [90; 32],
                native_account: [n + 1; 32],
                funding_wallet: [n + 2; 32],
                custody_config: [n + 3; 32],
                payout_vault: [n + 6; 32],
                key_policy: [n + 7; 32],
                ledger,
            }
        })
        .collect();
    units::Configuration::new(domain(), descriptors).unwrap()
}
fn cipher() -> RecordCipher {
    RecordCipher::new(Zeroizing::new([42; 32]), 1, [89; 32]).unwrap()
}
type Store = units::Journal<SqliteBackend, RecordCipher>;
fn create(t: &Temp) -> Store {
    let mut j = Store::create(SqliteBackend::create(&t.db).unwrap(), cipher(), config()).unwrap();
    j.commit(enrollment(j.head())).unwrap();
    j
}
fn enrollment(head: Head) -> units::Transaction {
    units::Transaction {
        id: CommitId::new([1; 32]).unwrap(),
        expected: head,
        at: 1,
        change: units::Change::Enroll {
            wallet: key(9).verifying_key().to_bytes(),
            recipients: vec![
                units::Recipient {
                    unit: uid(40),
                    tokens: [71; 32],
                },
                units::Recipient {
                    unit: uid(50),
                    tokens: [72; 32],
                },
            ],
        },
    }
}
struct Channel {
    domain: Domain,
    binding: [u8; 32],
    expires: u64,
}
impl Default for Channel {
    fn default() -> Self {
        Self {
            domain: domain(),
            binding: [8; 32],
            expires: 100,
        }
    }
}
impl ConfidentialChannel for Channel {
    fn domain(&self) -> Domain {
        self.domain
    }
    fn binding(&self) -> [u8; 32] {
        self.binding
    }
    fn expires_at(&self) -> u64 {
        self.expires
    }
}
fn authorizer() -> api::Authorizer {
    api::Authorizer::new(100, 1000).unwrap()
}
fn request(which: u8, command: wire::Command, signer: u8) -> api::Request {
    let mut r = api::Request {
        unit: uid(which),
        body: wire::Request {
            domain: domain(),
            account: account(),
            id: RequestId::new([20; 32]).unwrap(),
            policy: PolicyVersion::new(1).unwrap(),
            epoch: 1,
            signer: key(signer).verifying_key().to_bytes(),
            session: [8; 32],
            expires_at: 90,
            command,
            signature: [0; 64],
        },
    };
    sign(&mut r, signer);
    r
}
fn sign(r: &mut api::Request, signer: u8) {
    r.body.signature = key(signer).sign(&r.message()).to_bytes();
}
fn grant() -> wire::Grant {
    wire::Grant {
        key: key(10).verifying_key().to_bytes(),
        methods: 7,
        market: MarketId::new([7; 32]).unwrap(),
        maximum_lots: 2,
        maximum_fee: 1,
        maximum_orders: 1,
        expires_at: 80,
    }
}
fn scope(which: u8) -> api::Scope {
    api::Scope {
        domain: domain(),
        unit: uid(which),
        account: account(),
        epoch: 1,
    }
}
// Deliberately returns a supplied cut even for wrong hints: authorizer must check
// returned scope/head. This is a synthetic trusted port, not a shipped grant store.
struct GrantPort {
    cut: api::GrantCut,
    lookups: Cell<usize>,
}
impl api::Grants for GrantPort {
    fn lookup(&self, _: api::Scope, _: [u8; 32], _: Head) -> Result<Option<api::GrantCut>, Error> {
        self.lookups.set(self.lookups.get() + 1);
        Ok(Some(self.cut.clone()))
    }
}
fn port(j: &Store) -> GrantPort {
    GrantPort {
        cut: api::GrantCut {
            head: j.head(),
            scope: scope(40),
            grant: grant(),
            accepted_at: 1,
            used_orders: 0,
        },
        lookups: Cell::new(0),
    }
}
fn check(
    j: &mut Store,
    req: &api::Request,
    port: &impl api::Grants,
) -> Result<api::Checked, Error> {
    authorizer().check(j, &Channel::default(), &req.encode().unwrap(), port, 10)
}
fn order() -> wire::Command {
    wire::Command::Order {
        market: MarketId::new([7; 32]).unwrap(),
        lots: 2,
        minimum: 90,
        maximum: 110,
        fee: 1,
        tif: 2,
        reduce_only: false,
        good_until: 70,
    }
}
fn hex(s: &str) -> Vec<u8> {
    s.as_bytes()
        .chunks_exact(2)
        .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
        .collect()
}

#[test]
fn v2_matches_manual_unit_extensions_of_all_shared_v1_command_vectors() {
    let magic_len = b"CINDER-API\0\x00\x01".len();
    let economic_end = magic_len + 32 * 4 + 4;
    let command_start = economic_end + 8 + 32 + 32 + 8;
    for row in include_str!("fixtures/vectors.tsv").lines() {
        let fields: Vec<_> = row.split('\t').collect();
        let old_message = hex(fields[1]);
        let mut old_wire = old_message.clone();
        old_wire.extend(hex(fields[4]));
        let legacy = wire::Request::decode(&old_wire).unwrap();
        for which in [40, 50] {
            let mut message = old_message[..economic_end].to_vec();
            message[magic_len - 1] = 2;
            message.extend_from_slice(&[which; 32]);
            message.extend_from_slice(&old_message[economic_end..]);
            let mut expected_intent = b"CINDER-API-INTENT\0\x00\x02".to_vec();
            expected_intent.extend_from_slice(&old_message[magic_len..economic_end]);
            expected_intent.extend_from_slice(&[which; 32]);
            expected_intent.extend_from_slice(&old_message[command_start..]);
            let mut req = api::Request {
                unit: uid(which),
                body: legacy.clone(),
            };
            req.body.signature = key(9).sign(&message).to_bytes();
            assert_eq!(&*req.message(), &message);
            assert_eq!(
                req.digest(),
                <[u8; 32]>::from(Sha256::digest(expected_intent))
            );
            key(9)
                .verifying_key()
                .verify_strict(&message, &Signature::from_bytes(&req.body.signature))
                .unwrap();
            assert_eq!(
                api::Request::decode(req.encode().unwrap().as_bytes()).unwrap(),
                req
            );
            assert!(api::Request::decode(&old_wire).is_err());
            assert!(wire::Request::decode(req.encode().unwrap().as_bytes()).is_err());
        }
    }
}

#[test]
fn unit_is_economic_identity_while_auth_refresh_is_not() {
    let original = request(40, order(), 9);
    let mut refreshed = original.clone();
    refreshed.body.session = [7; 32];
    refreshed.body.epoch = 2;
    refreshed.body.expires_at = 99;
    refreshed.body.signer = key(10).verifying_key().to_bytes();
    sign(&mut refreshed, 10);
    assert_eq!(original.digest(), refreshed.digest());
    assert_ne!(original.message(), refreshed.message());
    refreshed.unit = uid(50);
    assert_ne!(original.digest(), refreshed.digest());
    refreshed = original.clone();
    let wire::Command::Order { good_until, .. } = &mut refreshed.body.command else {
        unreachable!()
    };
    *good_until += 1;
    assert_ne!(original.digest(), refreshed.digest());
    assert_ne!(original.digest(), original.body.digest());
}

#[test]
fn strict_v2_parser_refuses_all_truncations_versions_and_zero_unit() {
    let req = request(40, order(), 9);
    let wire = req.encode().unwrap();
    let wire = wire.as_bytes();
    for n in 0..wire.len() {
        assert!(api::Request::decode(&wire[..n]).is_err(), "prefix {n}");
    }
    let mut bad = wire.to_vec();
    bad.push(0);
    assert!(api::Request::decode(&bad).is_err());
    let mut bad = wire.to_vec();
    bad[12] = 3;
    assert!(api::Request::decode(&bad).is_err());
    let mut bad = wire.to_vec();
    bad[145..177].fill(0);
    assert!(api::Request::decode(&bad).is_err());
    assert!(api::Request::decode(&vec![1; wire::MAX_REQUEST + 1]).is_err());
    let mut bad = wire.to_vec(); // Invalid TIF and boolean in the order body.
    let command = 13 + 132 + 32 + 80;
    bad[command + 1 + 32 + 8 + 8 + 8 + 16] = 3;
    assert!(api::Request::decode(&bad).is_err());
    let mut bad = wire.to_vec();
    bad[command + 1 + 32 + 8 + 8 + 8 + 16 + 1] = 2;
    assert!(api::Request::decode(&bad).is_err());
}

#[test]
fn owner_checks_both_units_without_mutation_or_an_implicit_default() {
    let t = Temp::new();
    let mut j = create(&t);
    let head = j.head();
    let before = j.state().unwrap().clone();
    for which in [40, 50] {
        let req = request(which, wire::Command::View, 9);
        let checked = check(&mut j, &req, &api::NoGrants).unwrap();
        assert_eq!(checked.request(), &req);
        assert_eq!(checked.head(), head);
        assert_eq!(checked.principal(), api::Principal::Owner);
        assert!(checked.grant().is_none());
    }
    assert_eq!(j.state().unwrap(), &before);
    assert_eq!(j.head(), head);
    assert!(check(&mut j, &request(99, wire::Command::View, 9), &api::NoGrants).is_err());
}

#[test]
fn signature_scope_and_policy_tampering_fail_before_private_grant_lookup() {
    let t = Temp::new();
    let mut j = create(&t);
    let p = port(&j);
    let original = request(40, order(), 10);
    for change in 0..8 {
        let mut req = original.clone();
        match change {
            0 => req.unit = uid(50),
            1 => req.body.session = [7; 32],
            2 => req.body.policy = PolicyVersion::new(2).unwrap(),
            3 => req.body.domain.deployment = DeploymentId::new([3; 32]).unwrap(),
            4 => req.body.account = AccountId::new([4; 32]).unwrap(),
            5 => req.body.expires_at = 10,
            6 => req.body.signature = key(10).sign(&req.body.message()).to_bytes(), // v1 signature
            7 => req.body.epoch = 0,
            _ => unreachable!(),
        }
        assert_eq!(check(&mut j, &req, &p).unwrap_err(), Error::Unauthorized);
    }
    assert_eq!(p.lookups.get(), 0);
    let mut unregistered = original;
    unregistered.body.account = AccountId::new([4; 32]).unwrap();
    sign(&mut unregistered, 10);
    assert_eq!(
        check(&mut j, &unregistered, &p).unwrap_err(),
        Error::Unauthorized
    );
    assert_eq!(p.lookups.get(), 0);
}

#[test]
fn grant_scope_binds_unit_account_epoch_and_current_shared_head() {
    let t = Temp::new();
    let mut j = create(&t);
    let p = port(&j);
    assert_eq!(
        check(&mut j, &request(40, order(), 10), &p)
            .unwrap()
            .principal(),
        api::Principal::Agent
    );
    // Agent may sign a new B request, but its A grant cannot authorize it.
    assert_eq!(
        check(&mut j, &request(50, order(), 10), &p).unwrap_err(),
        Error::Unauthorized
    );
    let req = request(40, order(), 10);
    for change in 0..8 {
        let mut wrong = port(&j);
        let expected = match change {
            0 => {
                wrong.cut.scope.account = AccountId::new([4; 32]).unwrap();
                Error::Unauthorized
            }
            1 => {
                wrong.cut.scope.epoch = 2;
                Error::Unauthorized
            }
            2 => {
                wrong.cut.head.hash[0] ^= 1;
                Error::Unavailable
            }
            3 => {
                wrong.cut.grant.key = key(11).verifying_key().to_bytes();
                Error::Unauthorized
            }
            4 => {
                wrong.cut.accepted_at = 11;
                Error::Unavailable
            }
            5 => {
                wrong.cut.used_orders = 2;
                Error::Unavailable
            }
            6 => {
                wrong.cut.scope.domain.deployment = DeploymentId::new([3; 32]).unwrap();
                Error::Unauthorized
            }
            7 => {
                // Before wall time but after this head's last accepted event.
                wrong.cut.accepted_at = 2;
                Error::Unavailable
            }
            _ => unreachable!(),
        };
        assert_eq!(check(&mut j, &req, &wrong).unwrap_err(), expected);
    }
}

#[test]
fn read_is_unit_wide_but_trade_cancel_and_owner_only_methods_keep_existing_bounds() {
    let t = Temp::new();
    let mut j = create(&t);
    let mut p = port(&j);
    p.cut.grant.methods = wire::READ;
    for kind in 0..cinder_api::reads::KINDS {
        let req = request(
            40,
            wire::Command::Read(cinder_api::reads::Query {
                kind,
                cursor: [0; 40],
                limit: 64,
            }),
            10,
        );
        check(&mut j, &req, &p).unwrap();
    }
    assert_eq!(
        check(&mut j, &request(40, order(), 10), &p).unwrap_err(),
        Error::Unauthorized
    );
    p.cut.grant.methods = wire::TRADE | wire::CANCEL;
    for change in 0..4 {
        let mut command = order();
        let wire::Command::Order {
            lots,
            fee,
            market,
            good_until,
            ..
        } = &mut command
        else {
            unreachable!()
        };
        // Each refusal exercises one bounded field.
        match change {
            0 => *lots = 3,
            1 => *fee = 2,
            2 => *market = MarketId::new([8; 32]).unwrap(),
            _ => *good_until = 81,
        };
        assert_eq!(
            check(&mut j, &request(40, command, 10), &p).unwrap_err(),
            Error::Unauthorized
        );
    }
    let cancel = wire::Command::Cancel {
        target: RequestId::new([3; 32]).unwrap(),
        attempt: AttemptId::new([4; 32]).unwrap(),
        good_until: 70,
    };
    check(&mut j, &request(40, cancel, 10), &p).unwrap(); // Original ownership is later admission, not preflight.
    for command in [
        wire::Command::Grant(grant()),
        wire::Command::Revoke,
        wire::Command::Payout {
            net: 1,
            maximum_fee: 0,
            allow_partial: false,
            good_until: 70,
        },
        wire::Command::Leverage {
            market: grant().market,
            leverage: 20000,
            good_until: 70,
        },
    ] {
        assert_eq!(
            check(&mut j, &request(40, command, 10), &p).unwrap_err(),
            Error::Unauthorized
        );
    }
    p.cut.grant.expires_at = 10;
    assert_eq!(
        check(&mut j, &request(40, order(), 10), &p).unwrap_err(),
        Error::Unauthorized
    );
}

#[test]
fn scope_preflight_does_not_install_grants_advance_epochs_or_consume_exhausted_quota() {
    let t = Temp::new();
    let mut j = create(&t);
    let head = j.head();
    let before = j.state().unwrap().clone();
    let mut p = port(&j);
    p.cut.used_orders = p.cut.grant.maximum_orders;
    let checked = check(&mut j, &request(40, order(), 10), &p).unwrap();
    assert_eq!(checked.grant().unwrap().used_orders, 1); // Exact retries still need auth; admission rejects new use later.
    check(
        &mut j,
        &request(40, wire::Command::Grant(grant()), 9),
        &api::NoGrants,
    )
    .unwrap();
    check(
        &mut j,
        &request(40, wire::Command::Revoke, 9),
        &api::NoGrants,
    )
    .unwrap();
    assert_eq!(j.head(), head);
    assert_eq!(j.state().unwrap(), &before);
    assert_eq!(
        check(&mut j, &request(40, order(), 10), &api::NoGrants).unwrap_err(),
        Error::Unauthorized
    );
}

#[test]
fn global_revocation_survives_replay_and_invalidates_agent_scopes_in_both_units() {
    let t = Temp::new();
    let mut j = create(&t);
    let mut p = port(&j);
    j.commit(units::Transaction {
        id: CommitId::new([2; 32]).unwrap(),
        expected: j.head(),
        at: 2,
        change: units::Change::AdvanceAuthority {
            account: account(),
            epoch: 2,
        },
    })
    .unwrap();
    p.cut.head = j.head();
    drop(j);
    let mut j = Store::open(
        SqliteBackend::open(&t.db, Migration::None).unwrap(),
        cipher(),
        config(),
    )
    .unwrap();
    for which in [40, 50] {
        assert_eq!(
            check(&mut j, &request(which, wire::Command::View, 10), &p).unwrap_err(),
            Error::Unauthorized
        );
        let mut req = request(which, wire::Command::View, 9);
        req.body.epoch = 2;
        sign(&mut req, 9);
        check(&mut j, &req, &api::NoGrants).unwrap();
        assert_eq!(
            j.state()
                .unwrap()
                .unit(uid(which))
                .unwrap()
                .authority_epoch(account()),
            Some(2)
        );
        assert_eq!(
            j.state()
                .unwrap()
                .unit(uid(which))
                .unwrap()
                .ledger()
                .book(Owner::Customer(account()))
                .unwrap()
                .cash()
                .atoms(),
            0
        );
    }
}

struct WitnessFault {
    inner: SqliteBackend,
    fail: Arc<AtomicBool>,
}
impl Backend for WitnessFault {
    fn load(&mut self) -> Result<Vec<Frame>, cinder_journal::Error> {
        self.inner.load()
    }
    fn append(
        &mut self,
        expected: Option<Head>,
        frame: &Frame,
    ) -> Result<(), cinder_journal::Error> {
        self.inner.append(expected, frame)
    }
    fn check_current(&mut self, _: Head) -> Result<(), cinder_journal::Error> {
        if self.fail.load(Ordering::SeqCst) {
            Err(cinder_journal::Error::Storage)
        } else {
            Ok(())
        }
    }
}

#[test]
fn shared_freshness_failure_fences_every_unit_before_grant_lookup() {
    let t = Temp::new();
    let fail = Arc::new(AtomicBool::new(false));
    let mut j = units::Journal::create(
        WitnessFault {
            inner: SqliteBackend::create(&t.db).unwrap(),
            fail: fail.clone(),
        },
        cipher(),
        config(),
    )
    .unwrap();
    j.commit(enrollment(j.head())).unwrap();
    let req = request(40, wire::Command::View, 9);
    fail.store(true, Ordering::SeqCst);
    assert_eq!(
        authorizer()
            .check(
                &mut j,
                &Channel::default(),
                &req.encode().unwrap(),
                &api::NoGrants,
                10
            )
            .unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(j.state().unwrap_err(), cinder_journal::Error::Poisoned);
    fail.store(false, Ordering::SeqCst);
    assert_eq!(
        authorizer()
            .check(
                &mut j,
                &Channel::default(),
                &request(50, wire::Command::View, 9).encode().unwrap(),
                &api::NoGrants,
                10
            )
            .unwrap_err(),
        Error::Unavailable
    );
    j.reload().unwrap();
    authorizer()
        .check(
            &mut j,
            &Channel::default(),
            &req.encode().unwrap(),
            &api::NoGrants,
            10,
        )
        .unwrap();
}

#[test]
fn owner_grant_limits_and_channel_time_boundaries_are_checked_without_mutation() {
    let t = Temp::new();
    let mut j = create(&t);
    let head = j.head();
    assert!(api::Authorizer::new(0, 1000).is_err());
    assert!(api::Authorizer::new(100, 0).is_err());
    for change in 0..9 {
        let mut g = grant();
        match change {
            0 => g.key = key(9).verifying_key().to_bytes(),
            1 => g.methods = 8,
            2 => g.maximum_lots = 0,
            3 => g.maximum_fee = -1,
            4 => g.maximum_orders = 0,
            5 => g.expires_at = 10,
            6 => g.expires_at = 1011,
            7 => g.market = MarketId::new([8; 32]).unwrap(),
            8 => g.key = [0; 32],
            _ => unreachable!(),
        }
        assert_eq!(
            check(
                &mut j,
                &request(40, wire::Command::Grant(g), 9),
                &api::NoGrants
            )
            .unwrap_err(),
            Error::Invalid
        );
    }
    let req = request(40, wire::Command::View, 9);
    for channel in [
        Channel {
            binding: [0; 32],
            ..Channel::default()
        },
        Channel {
            expires: 90 - 1,
            ..Channel::default()
        },
        Channel {
            domain: Domain {
                deployment: DeploymentId::new([3; 32]).unwrap(),
                ..domain()
            },
            ..Channel::default()
        },
    ] {
        assert_eq!(
            authorizer()
                .check(&mut j, &channel, &req.encode().unwrap(), &api::NoGrants, 10)
                .unwrap_err(),
            Error::Unauthorized
        );
    }
    assert_eq!(
        authorizer()
            .check(
                &mut j,
                &Channel::default(),
                &req.encode().unwrap(),
                &api::NoGrants,
                90
            )
            .unwrap_err(),
        Error::Unauthorized
    );
    assert_eq!(
        authorizer()
            .check(
                &mut j,
                &Channel::default(),
                &req.encode().unwrap(),
                &api::NoGrants,
                0
            )
            .unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(j.head(), head);
}

#[test]
fn replies_bind_the_unit_without_redefining_the_existing_private_body() {
    let response = Response::View(cinder_api::View {
        epoch: 1,
        cash: 100,
        funding: 0,
        held: 20,
        positions: vec![],
        operations: vec![],
    });
    for which in [40, 50] {
        let bytes = api::encode_response(uid(which), &response).unwrap();
        let expected = [
            b"CINDER-API-REPLY\0\x00\x02".as_slice(),
            &[which; 32],
            response.encode().unwrap().as_bytes(),
        ]
        .concat();
        assert_eq!(bytes.as_bytes(), expected);
        let error = api::encode_error(uid(which), Error::Unauthorized).unwrap();
        assert_eq!(
            error.as_bytes(),
            [
                b"CINDER-API-REPLY\0\x00\x02".as_slice(),
                &[which; 32],
                Error::Unauthorized.encode_private().unwrap().as_bytes()
            ]
            .concat()
        );
    }
}

// P31: these requests use real signatures and record AEAD, but no native I/O.
fn history_request(
    id: u8,
    which: u8,
    command: wire::Command,
    signer: u8,
    epoch: u64,
) -> api::Request {
    let mut req = request(which, command, signer);
    req.body.id = RequestId::new([id; 32]).unwrap();
    req.body.epoch = epoch;
    sign(&mut req, signer);
    req
}
fn capture(j: &mut Store, req: &api::Request, at: u64) -> Result<api::history::Captured, Error> {
    authorizer().capture(j, &Channel::default(), &req.encode().unwrap(), at)
}
fn reopen(t: &Temp) -> Store {
    Store::open(
        SqliteBackend::open(&t.db, Migration::None).unwrap(),
        cipher(),
        config(),
    )
    .unwrap()
}
fn install(j: &mut Store, id: u8, which: u8, agent: u8, epoch: u64) {
    let mut g = grant();
    g.key = key(agent).verifying_key().to_bytes();
    let result = capture(
        j,
        &history_request(id, which, wire::Command::Grant(g), 9, epoch),
        10,
    )
    .unwrap();
    assert_eq!(result.effect, units::api::Effect::Grant);
}

#[test]
fn durable_grants_supply_real_head_bound_cuts_in_both_units_after_replay() {
    let t = Temp::new();
    let mut j = create(&t);
    install(&mut j, 30, 40, 10, 1);
    let before_b = j.head();
    assert_eq!(
        capture(&mut j, &history_request(31, 50, order(), 10, 1), 10).unwrap_err(),
        Error::Unauthorized
    );
    assert_eq!(j.head(), before_b);
    install(&mut j, 31, 50, 10, 1);
    let a = history_request(32, 40, order(), 10, 1);
    let b = history_request(33, 50, order(), 10, 1);
    let before = j.state().unwrap().clone();
    assert_eq!(
        capture(&mut j, &a, 11).unwrap().effect,
        units::api::Effect::Recorded
    );
    assert_eq!(
        capture(&mut j, &b, 11).unwrap().effect,
        units::api::Effect::Recorded
    );
    for which in [40, 50] {
        assert_eq!(j.state().unwrap().unit(uid(which)), before.unit(uid(which)));
    }
    let expected = j.state().unwrap().clone();
    let head = j.head();
    drop(j);
    let mut j = reopen(&t);
    assert_eq!(j.head(), head);
    assert_eq!(j.state().unwrap(), &expected);
    let history = api::history::History::load(&mut j, &authorizer()).unwrap();
    for req in [a, b] {
        let checked = authorizer()
            .check(
                &mut j,
                &Channel::default(),
                &req.encode().unwrap(),
                &history,
                12,
            )
            .unwrap();
        let cut = checked.grant().unwrap();
        assert_eq!(cut.scope.unit, req.unit);
        assert_eq!(cut.head, head);
        assert_eq!(cut.accepted_at, 10);
        assert_eq!(cut.used_orders, 0);
        assert!(
            j.state()
                .unwrap()
                .unit(req.unit)
                .unwrap()
                .orders()
                .is_empty()
        );
    }
}

#[test]
fn capture_keeps_original_identity_and_authenticates_before_retry_conflicts() {
    let t = Temp::new();
    let mut j = create(&t);
    let req = history_request(30, 40, order(), 9, 1);
    let first = capture(&mut j, &req, 10).unwrap();
    let head = j.head();
    let mut retry = req.clone();
    retry.body.expires_at = 99;
    sign(&mut retry, 9);
    let again = capture(&mut j, &retry, 11).unwrap();
    assert!(again.duplicate);
    assert_eq!(again.request, first.request);
    assert_eq!(again.at, 10);
    assert_eq!(j.head(), head);
    let mut wrong = retry.clone();
    wrong.unit = uid(50);
    sign(&mut wrong, 9);
    assert_eq!(capture(&mut j, &wrong, 11).unwrap_err(), Error::Conflict);
    wrong.body.signature = [0; 64];
    assert_eq!(
        capture(&mut j, &wrong, 11).unwrap_err(),
        Error::Unauthorized
    );
    let mut changed = req.clone();
    changed.body.command = wire::Command::Payout {
        net: 1,
        maximum_fee: 0,
        allow_partial: false,
        good_until: 70,
    };
    sign(&mut changed, 9);
    assert_eq!(capture(&mut j, &changed, 11).unwrap_err(), Error::Conflict);
    assert_eq!(j.head(), head);
    assert_eq!(j.state().unwrap().api_records().len(), 1);
}

#[test]
fn accepted_revocation_is_atomic_global_and_fresh_retries_do_not_repeat_it() {
    let t = Temp::new();
    let mut j = create(&t);
    install(&mut j, 30, 40, 10, 1);
    install(&mut j, 31, 50, 10, 1);
    let revoke = history_request(32, 40, wire::Command::Revoke, 9, 1);
    assert_eq!(
        capture(&mut j, &revoke, 11).unwrap().effect,
        units::api::Effect::Revoke
    );
    let head = j.head();
    drop(j);
    let mut j = reopen(&t);
    for which in [40, 50] {
        assert_eq!(
            j.state()
                .unwrap()
                .unit(uid(which))
                .unwrap()
                .authority_epoch(account()),
            Some(2)
        );
        assert_eq!(
            capture(&mut j, &history_request(33, which, order(), 10, 1), 12).unwrap_err(),
            Error::Unauthorized
        );
    }
    assert_eq!(
        capture(&mut j, &revoke, 12).unwrap_err(),
        Error::Unauthorized
    );
    let mut retry = revoke.clone();
    retry.body.epoch = 2;
    sign(&mut retry, 9);
    assert!(capture(&mut j, &retry, 12).unwrap().duplicate);
    assert_eq!(j.head(), head);
    // Retrying an old grant with current owner auth does not install it in epoch 2.
    let old = history_request(30, 40, wire::Command::Grant(grant()), 9, 2);
    assert!(capture(&mut j, &old, 12).unwrap().duplicate);
    assert_eq!(
        capture(&mut j, &history_request(34, 40, order(), 10, 2), 12).unwrap_err(),
        Error::Unauthorized
    );
    capture(
        &mut j,
        &history_request(35, 40, wire::Command::Grant(grant()), 9, 2),
        12,
    )
    .unwrap();
    capture(&mut j, &history_request(36, 40, order(), 10, 2), 13).unwrap();
}

#[test]
fn recorded_orders_do_not_spend_agent_quota_and_grants_cannot_be_amended() {
    let t = Temp::new();
    let mut j = create(&t);
    install(&mut j, 30, 40, 10, 1);
    for id in [31, 32] {
        assert_eq!(
            capture(&mut j, &history_request(id, 40, order(), 10, 1), 11)
                .unwrap()
                .effect,
            units::api::Effect::Recorded
        );
    }
    let history = api::history::History::load(&mut j, &authorizer()).unwrap();
    let checked = authorizer()
        .check(
            &mut j,
            &Channel::default(),
            &history_request(33, 40, order(), 10, 1).encode().unwrap(),
            &history,
            12,
        )
        .unwrap();
    assert_eq!(checked.grant().unwrap().used_orders, 0);
    let head = j.head();
    let mut g = grant();
    g.maximum_orders = 2;
    assert_eq!(
        capture(
            &mut j,
            &history_request(34, 40, wire::Command::Grant(g), 9, 1),
            12
        )
        .unwrap_err(),
        Error::Invalid
    );
    assert_eq!(j.head(), head);
    for which in [40, 50] {
        let s = j.state().unwrap().unit(uid(which)).unwrap();
        assert!(s.orders().is_empty());
        assert!(s.holds().is_empty());
        assert!(s.attempts().is_empty());
    }
}

#[test]
fn owner_recovers_expired_grant_record_without_reinstalling_or_revalidating_its_deadline() {
    let t = Temp::new();
    let mut j = create(&t);
    let original = history_request(30, 40, wire::Command::Grant(grant()), 9, 1);
    capture(&mut j, &original, 10).unwrap();
    let head = j.head();
    let mut retry = original.clone();
    retry.body.expires_at = 99;
    sign(&mut retry, 9);
    assert!(capture(&mut j, &retry, 81).unwrap().duplicate);
    assert_eq!(j.head(), head);
    let stale_agent = history_request(31, 40, order(), 10, 1);
    assert_eq!(
        capture(&mut j, &stale_agent, 81).unwrap_err(),
        Error::Unauthorized
    );
    let mut new = retry;
    new.body.id = RequestId::new([32; 32]).unwrap();
    sign(&mut new, 9);
    assert_eq!(capture(&mut j, &new, 81).unwrap_err(), Error::Invalid);
}

#[test]
fn recorded_cancel_and_lookup_keep_unit_and_original_agent_ownership() {
    let t = Temp::new();
    let mut j = create(&t);
    install(&mut j, 30, 40, 10, 1);
    install(&mut j, 31, 40, 11, 1);
    let req = history_request(32, 40, order(), 10, 1);
    capture(&mut j, &req, 11).unwrap();
    let mut takeover = req.clone();
    takeover.body.signer = key(11).verifying_key().to_bytes();
    sign(&mut takeover, 11);
    assert_eq!(
        capture(&mut j, &takeover, 12).unwrap_err(),
        Error::Unauthorized
    );
    takeover.body.signer = key(9).verifying_key().to_bytes();
    sign(&mut takeover, 9);
    assert!(capture(&mut j, &takeover, 12).unwrap().duplicate);
    let query = history_request(90, 40, wire::Command::Operation(req.body.id), 11, 1);
    assert_eq!(
        authorizer()
            .captured_operation(&mut j, &Channel::default(), &query.encode().unwrap(), 12)
            .unwrap()
            .request,
        req
    );
    let wrong = history_request(90, 50, wire::Command::Operation(req.body.id), 9, 1);
    assert_eq!(
        authorizer()
            .captured_operation(&mut j, &Channel::default(), &wrong.encode().unwrap(), 12)
            .unwrap_err(),
        Error::NotFound
    );
    let cancel = wire::Command::Cancel {
        target: req.body.id,
        attempt: AttemptId::new([1; 32]).unwrap(),
        good_until: 70,
    };
    assert_eq!(
        capture(&mut j, &history_request(33, 50, cancel.clone(), 9, 1), 12).unwrap_err(),
        Error::NotFound
    );
    assert_eq!(
        capture(&mut j, &history_request(33, 40, cancel.clone(), 11, 1), 12).unwrap_err(),
        Error::Unauthorized
    );
    assert_eq!(
        capture(&mut j, &history_request(33, 40, cancel, 10, 1), 12)
            .unwrap()
            .effect,
        units::api::Effect::Recorded
    );
    assert!(
        j.state()
            .unwrap()
            .unit(uid(40))
            .unwrap()
            .attempts()
            .is_empty()
    );
}

#[test]
fn history_contract_change_and_cached_grant_head_cannot_reset_authority() {
    let t = Temp::new();
    let mut j = create(&t);
    install(&mut j, 30, 40, 10, 1);
    let old = api::history::History::load(&mut j, &authorizer()).unwrap();
    assert!(
        api::history::History::load(&mut j, &api::Authorizer::new(101, 1000).unwrap()).is_err()
    );
    assert!(
        api::history::History::load(&mut j, &api::Authorizer::new(100, 1001).unwrap()).is_err()
    );
    capture(&mut j, &history_request(31, 40, order(), 9, 1), 11).unwrap();
    assert_eq!(
        authorizer()
            .check(
                &mut j,
                &Channel::default(),
                &history_request(32, 40, order(), 10, 1).encode().unwrap(),
                &old,
                12
            )
            .unwrap_err(),
        Error::Unavailable
    );
    let fresh = api::history::History::load(&mut j, &authorizer()).unwrap();
    authorizer()
        .check(
            &mut j,
            &Channel::default(),
            &history_request(32, 40, order(), 10, 1).encode().unwrap(),
            &fresh,
            12,
        )
        .unwrap();
}

#[test]
fn malformed_trusted_api_record_cannot_become_accepted_grant_evidence() {
    for change in 0..4 {
        let t = Temp::new();
        let mut j = create(&t);
        install(&mut j, 30, 40, 10, 1);
        let mut r = j.state().unwrap().api_records()[0].record.clone();
        let req = history_request(31, 40, order(), 9, 1);
        r.request.request = req.body.id;
        r.digest = req.digest();
        r.wire = req.encode().unwrap();
        r.effect = units::api::Effect::Recorded;
        match change {
            0 => r.digest[0] ^= 1,
            1 => r.signer = key(11).verifying_key().to_bytes(),
            2 => {
                let mut bad = r.wire.as_bytes().to_vec();
                let n = bad.len();
                bad[n - 1] ^= 1;
                r.wire = PrivateBytes::new(bad).unwrap();
            }
            _ => r.effect = units::api::Effect::Grant,
        };
        j.commit(units::Transaction {
            id: CommitId::new([90; 32]).unwrap(),
            expected: j.head(),
            at: 11,
            change: units::Change::Api(Box::new(r)),
        })
        .unwrap();
        assert!(api::history::History::load(&mut j, &authorizer()).is_err());
        drop(j);
        let mut j = reopen(&t);
        assert!(api::history::History::load(&mut j, &authorizer()).is_err());
    }
}

struct ApiAppendFault {
    inner: SqliteBackend,
    fail: Arc<AtomicBool>,
    after: bool,
}
impl Backend for ApiAppendFault {
    fn load(&mut self) -> Result<Vec<Frame>, cinder_journal::Error> {
        self.inner.load()
    }
    fn append(
        &mut self,
        expected: Option<Head>,
        frame: &Frame,
    ) -> Result<(), cinder_journal::Error> {
        if self.fail.load(Ordering::SeqCst) {
            if self.after {
                self.inner.append(expected, frame)?;
            }
            return Err(cinder_journal::Error::Storage);
        }
        self.inner.append(expected, frame)
    }
    fn check_current(&mut self, h: Head) -> Result<(), cinder_journal::Error> {
        self.inner.check_current(h)
    }
}

#[test]
fn uncertain_capture_requires_reload_and_never_repeats_committed_revocation() {
    for after in [false, true] {
        let t = Temp::new();
        let fail = Arc::new(AtomicBool::new(false));
        let mut j = units::Journal::create(
            ApiAppendFault {
                inner: SqliteBackend::create(&t.db).unwrap(),
                fail: fail.clone(),
                after,
            },
            cipher(),
            config(),
        )
        .unwrap();
        j.commit(enrollment(j.head())).unwrap();
        let req = history_request(30, 40, wire::Command::Revoke, 9, 1);
        fail.store(true, Ordering::SeqCst);
        assert_eq!(
            authorizer()
                .capture(&mut j, &Channel::default(), &req.encode().unwrap(), 10)
                .unwrap_err(),
            Error::Unavailable
        );
        assert!(j.state().is_err());
        fail.store(false, Ordering::SeqCst);
        assert!(api::history::History::load(&mut j, &authorizer()).is_err());
        j.reload().unwrap();
        let mut retry = req.clone();
        retry.body.epoch = if after { 2 } else { 1 };
        sign(&mut retry, 9);
        let result = authorizer()
            .capture(&mut j, &Channel::default(), &retry.encode().unwrap(), 11)
            .unwrap();
        assert_eq!(result.duplicate, after);
        assert_eq!(j.state().unwrap().api_records().len(), 1);
        for which in [40, 50] {
            assert_eq!(
                j.state()
                    .unwrap()
                    .unit(uid(which))
                    .unwrap()
                    .authority_epoch(account()),
                Some(2)
            );
        }
        assert_eq!(j.head().sequence, 2);
    }
}

#[test]
fn durable_api_records_remain_ciphertext_and_private_debug_is_redacted() {
    let t = Temp::new();
    let mut j = create(&t);
    install(&mut j, 30, 40, 10, 1);
    let req = history_request(31, 40, order(), 10, 1);
    let captured = capture(&mut j, &req, 11).unwrap();
    let history = api::history::History::load(&mut j, &authorizer()).unwrap();
    assert_eq!(format!("{captured:?}"), "Captured([PRIVATE])");
    assert_eq!(format!("{history:?}"), "ApiHistory([PRIVATE])");
    let wire = req.encode().unwrap();
    drop(j);
    let mut backend = SqliteBackend::open(&t.db, Migration::None).unwrap();
    for frame in backend.load().unwrap() {
        assert!(
            !frame
                .opaque
                .as_bytes()
                .windows(wire.as_bytes().len())
                .any(|b| b == wire.as_bytes())
        );
    }
}
