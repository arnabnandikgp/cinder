//! Synthetic chain/native qualification through the actual selected controller.
//! No RPC, venue requests, real wallets/funds or recovery activation.
#[path = "../../journal/tests/support/mod.rs"]
mod support;
use cinder_journal::{
    controller::Controller as _,
    model::*,
    sqlite::{Migration, SqliteBackend},
    units, *,
};
use cinder_kernel::{
    identity::*,
    ledger::{evidence::*, *},
};
use cinder_pacifica::{
    execution::Origin,
    funding::{
        self,
        escrow::Routing,
        recovery::{CustodyState, CustomerState},
        *,
    },
    profile::*,
};
use ed25519_dalek::SigningKey;
use zeroize::Zeroizing;
type Store = units::Journal<SqliteBackend, support::FixtureProtection>;
fn uid(n: u8) -> UnitId {
    UnitId::new([n; 32]).unwrap()
}
fn wallet() -> [u8; 32] {
    SigningKey::from_bytes(&[33; 32]).verifying_key().to_bytes()
}
fn broker() -> [u8; 32] {
    SigningKey::from_bytes(&[9; 32]).verifying_key().to_bytes()
}
fn route() -> Route {
    let domain = support::config().domain.deployment.bytes();
    let pool = [18; 32];
    let program = [20; 32];
    let mint = [23; 32];
    let config = chain::pda(program, &[b"cinder_vault", &domain, &pool, &mint]).unwrap();
    Route {
        domain,
        pool,
        program,
        config,
        vault: chain::pda(program, &[b"tokens", &config]).unwrap(),
        mint,
        funds: [19; 32],
        beneficiaries: vec![],
        broker: broker(),
        broker_tokens: [24; 32],
        venue_program: [25; 32],
        venue_vault: [26; 32],
        epoch: 1,
        decimals: 0,
        withdrawal: Level::Qualified,
        chain: Level::Qualified,
        settings: Level::Qualified,
        withdrawal_cost: 10,
        maximum_movement: 1000,
        maximum_fee: 5,
        setup_max_age: 1000,
    }
}
fn cfg() -> units::Configuration {
    let r = route();
    units::Configuration::new(
        support::config().domain,
        [40_u8, 50]
            .into_iter()
            .map(|n| {
                let mut c = support::config();
                c.customers.clear();
                c.venue = VenueId::new([n + 1; 32]).unwrap();
                c.venue_account = VenueAccountId::new([n + 2; 32]).unwrap();
                for s in &mut c.sources {
                    s.scope.venue = c.venue;
                    s.scope.account = c.venue_account;
                }
                for (tag, location) in [(8, Location::Vault), (9, Location::Broker)] {
                    let mut s = c.sources[0];
                    s.scope.namespace = NamespaceId::new([tag; 32]).unwrap();
                    s.location = location;
                    c.sources.push(s);
                }
                let mut descriptor = units::Descriptor {
                    id: uid(n),
                    environment: Origin::Testnet.commitment(),
                    native_account: if n == 40 { broker() } else { [n + 3; 32] },
                    funding_wallet: if n == 40 { broker() } else { [n + 4; 32] },
                    custody_config: if n == 40 { r.config } else { [n + 5; 32] },
                    payout_vault: if n == 40 { r.vault } else { [n + 6; 32] },
                    key_policy: [n + 7; 32],
                    ledger: c,
                };
                if n == 40 {
                    descriptor.key_policy = controller_for(descriptor.ledger.clone(), r.clone())
                        .release_commitment(&descriptor.ledger)
                        .unwrap();
                }
                descriptor
            })
            .collect(),
    )
    .unwrap()
}
fn controller() -> funding::Controller {
    controller_for(cfg().units()[0].ledger.clone(), route())
}
fn controller_for(c: Config, r: Route) -> funding::Controller {
    let venue = c.venue.bytes();
    let p = Profile {
        source: c.sources[0].scope,
        account: bs58::encode(broker()).into_string(),
        environment: Origin::Testnet.url().into(),
        revision: 1,
        evidence: "synthetic unit funds fixture".into(),
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
        config: c,
    };
    funding::Controller::new_escrow(
        p,
        r.clone(),
        Routing {
            venue,
            route: chain::pda(r.program, &[b"venue_route", &r.config]).unwrap(),
        },
        Zeroizing::new([9; 32]),
    )
    .unwrap()
}

#[test]
fn unit_funding_policy_pin_refuses_changed_limits_epoch_and_fees_before_binding() {
    let t = support::Temp::new();
    let mut j = fixture(&t);
    let before = j.head();
    for change in 0..4 {
        let mut r = route();
        match change {
            0 => r.maximum_movement += 1,
            1 => r.maximum_fee += 1,
            2 => r.epoch += 1,
            _ => r.setup_max_age += 1,
        }
        let controller = controller_for(cfg().units()[0].ledger.clone(), r);
        assert!(
            controller
                .bind(&mut j.select(uid(40)).unwrap(), id(2), 10)
                .is_err()
        );
        assert_eq!(j.head(), before);
    }
    controller()
        .bind(&mut j.select(uid(40)).unwrap(), id(2), 10)
        .unwrap();
}
fn fixture(t: &support::Temp) -> Store {
    let mut j = Store::create(
        SqliteBackend::create(&t.db).unwrap(),
        support::FixtureProtection,
        cfg(),
    )
    .unwrap();
    j.commit(units::Transaction {
        id: CommitId::new([1; 32]).unwrap(),
        expected: j.head(),
        at: 1,
        change: units::Change::Enroll {
            wallet: wallet(),
            recipients: vec![
                units::Recipient {
                    unit: uid(40),
                    tokens: [34; 32],
                },
                units::Recipient {
                    unit: uid(50),
                    tokens: [35; 32],
                },
            ],
        },
    })
    .unwrap();
    j
}
fn id(n: u8) -> CommitId {
    CommitId::new([n; 32]).unwrap()
}
#[test]
fn native_funding_controller_binds_only_its_unit_and_preserves_original_empty_account_ingress() {
    let t = support::Temp::new();
    let mut j = fixture(&t);
    let c = controller();
    assert!(c.bind(&mut j.select(uid(50)).unwrap(), id(2), 10).is_err());
    c.bind(&mut j.select(uid(40)).unwrap(), id(2), 10).unwrap();
    assert!(
        c.observe_setup(
            &mut j.select(uid(40)).unwrap(),
            id(3),
            10,
            Setup {
                account: broker(),
                observed_at: 10,
                lending_disabled: true,
                borrowed: "0".into(),
                interest: "0".into(),
                complete: true
            }
        )
        .unwrap()
    );
    let original = chain::Escrow::derive(&route(), wallet(), [80; 32], 1000).unwrap();
    let account = onboarding::account(support::config().domain, wallet()).unwrap();
    let batch = cinder_journal::escrow::Batch {
        request: RequestKey {
            domain: support::config().domain,
            account,
            request: RequestId::new([80; 32]).unwrap(),
        },
        route: c.escrow_routing().unwrap().route,
        batch: original.batch,
        operation: [80; 32],
        owner: wallet(),
        amount: support::cash(100),
        epoch: 1,
        expires_at_slot: 1000,
        signature: [81; 64],
    };
    c.observe_escrow_deposit(
        &mut j.select(uid(40)).unwrap(),
        id(4),
        10,
        batch.clone(),
        support::raw(b"qualified original owner deposit fixture"),
    )
    .unwrap();
    let before = j.state().unwrap().unit(uid(50)).unwrap().clone();
    let attempt = c
        .prepare_next_escrow(&mut j.select(uid(40)).unwrap(), 10)
        .unwrap()
        .unwrap();
    assert_eq!(attempt.request, batch.request);
    assert_eq!(
        j.state()
            .unwrap()
            .unit(uid(40))
            .unwrap()
            .ledger()
            .book(Owner::Customer(account))
            .unwrap()
            .cash()
            .atoms(),
        0
    );
    assert_eq!(j.state().unwrap().unit(uid(50)).unwrap(), &before);
    let head = j.head();
    assert_eq!(
        c.prepare_next_escrow(&mut j.select(uid(40)).unwrap(), 10)
            .unwrap(),
        Some(attempt)
    );
    assert_eq!(j.head(), head);
    drop(j);
    let mut j = Store::open(
        SqliteBackend::open(&t.db, Migration::None).unwrap(),
        support::FixtureProtection,
        cfg(),
    )
    .unwrap();
    assert_eq!(
        c.prepare_next_escrow(&mut j.select(uid(40)).unwrap(), 10)
            .unwrap(),
        Some(attempt)
    );
}
#[test]
fn unit_recovery_reuses_final_backing_and_lifetime_paid_counters_not_another_units_observation() {
    let t = support::Temp::new();
    let mut j = fixture(&t);
    let c = controller();
    c.bind(&mut j.select(uid(40)).unwrap(), id(2), 10).unwrap();
    j.commit(units::Transaction {
        id: id(3),
        expected: j.head(),
        at: 10,
        change: units::Change::BeginRecovery {
            versions: cfg()
                .units()
                .iter()
                .map(|u| {
                    (
                        u.id,
                        j.state().unwrap().unit(u.id).unwrap().ledger().version(),
                    )
                })
                .collect(),
            authority_epoch: 7,
            valid_until: 1000,
            fence: [99; 32],
        },
    })
    .unwrap();
    let cfg = cfg();
    let a = &cfg.units()[0].ledger;
    let l = j.state().unwrap().unit(uid(40)).unwrap().ledger();
    let check = EventKey {
        scope: a.sources[0].scope,
        event: EconomicEventId::new(&[4]).unwrap(),
        leg: 0,
    };
    let tx = Transaction {
        id: id(4),
        expected: j.head(),
        at: 10,
        evidence: vec![],
        inputs: vec![Input {
            source: check.scope,
            source_cut: Some(100),
            authority_epoch: 1,
            observed_at: 10,
            raw: support::raw(b"qualified zero-asset final native check"),
            event: Some(Event {
                key: RecordKey::Economic(check.clone()),
                policy: a.policy,
                change: Change::Reconcile(NativeCheck {
                    expected_version: l.version(),
                    cash: Some(l.venue().cash()),
                    funding: Some(l.venue().funding()),
                    positions: Some(l.venue().positions().to_vec()),
                    complete: true,
                    resolves: vec![],
                }),
            }),
        }],
        order_observations: vec![],
        funds_observations: vec![],
        controls: vec![],
    };
    j.select(uid(40)).unwrap().commit(tx).unwrap();
    let r = route();
    let observed = CustodyState {
        network: cfg.domain().network.bytes(),
        program: r.program,
        config: r.config,
        vault: r.vault,
        mint: r.mint,
        domain: r.domain,
        pool: r.pool,
        broker: r.broker,
        broker_tokens: r.broker_tokens,
        decimals: r.decimals,
        epoch: 2,
        mode: 1,
        normal_paid: 0,
        funding_sequence: 0,
        vault_amount: 0,
        finalized_slot: 200,
        customers: vec![CustomerState {
            wallet: wallet(),
            tokens: [34; 32],
            paid: 0,
            payout_sequence: 0,
        }],
    };
    let head = j.head();
    assert_eq!(
        c.recovery_backing(
            &mut j.select(uid(40)).unwrap(),
            head,
            check.clone(),
            &observed
        )
        .unwrap()
        .total(),
        0
    );
    assert!(
        c.recovery_backing(
            &mut j.select(uid(50)).unwrap(),
            head,
            check.clone(),
            &observed
        )
        .is_err()
    );
    let mut changed = observed.clone();
    changed.customers[0].paid = 1;
    assert!(
        c.recovery_backing(
            &mut j.select(uid(40)).unwrap(),
            head,
            check.clone(),
            &changed
        )
        .is_err()
    );
    changed = observed;
    changed.config = [22; 32];
    assert!(
        c.recovery_backing(&mut j.select(uid(40)).unwrap(), head, check, &changed)
            .is_err()
    );
}
