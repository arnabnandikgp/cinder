//! Synthetic unit scopes/accepted-grant port; no runnable v2 service or native I/O.
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
