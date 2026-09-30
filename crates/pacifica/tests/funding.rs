//! Offline joined funds tests; public synthetic keys and qualified fake evidence.
#[path = "../../journal/tests/support/mod.rs"]
mod support;
use cinder_journal::{collateral, funds as lifecycle, model::*, orders, sqlite::*, *};
use cinder_kernel::{
    amounts::*,
    codec::Canonical,
    identity::*,
    ledger::{evidence::*, funds::*, *},
};
use cinder_pacifica::{
    Error as AdapterError,
    execution::{Dispatch, Gateway, Origin, Outbound, Policy, Reply, Transport},
    funding::*,
    profile::*,
};
use ed25519_dalek::{Signature, SigningKey, VerifyingKey};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use support::{FixtureProtection, Store, Temp, raw, user};
use zeroize::Zeroizing;

#[test]
fn marginal_forecast_counts_prepared_and_partial_ingress_once_without_trading_credit() {
    let t = Temp::new();
    let (mut j, c, _) = setup(&t, false);
    assert_eq!(c.funding_need(&mut j, 100, 20).unwrap().additional, 20);
    prepare(&mut j, 10, Rail::Release, 20, 0);
    assert_eq!(c.funding_need(&mut j, 100, 20).unwrap().pending_ingress, 20);
    let p = expose(&mut j, &c, 10, Rail::Release, 60);
    c.observe_chain(&mut j, id(60), 100, chain(&p, 60)).unwrap();
    let need = c.funding_need(&mut j, 100, 20).unwrap();
    assert_eq!(
        (need.broker_usable, need.pending_ingress, need.additional),
        (20, 0, 0)
    );
    prepare(&mut j, 11, Rail::Deposit, 20, 0);
    let need = c.funding_need(&mut j, 100, 20).unwrap();
    assert_eq!(
        (need.broker_usable, need.pending_ingress, need.additional),
        (0, 20, 0)
    );
    let p = expose(&mut j, &c, 11, Rail::Deposit, 61);
    c.observe_chain(&mut j, id(61), 100, chain(&p, 61)).unwrap();
    credit(&c, &mut j, 62, 8, false);
    let need = c.funding_need(&mut j, 100, 20).unwrap();
    assert_eq!(
        (need.venue_usable, need.pending_ingress, need.additional),
        (8, 12, 0)
    );
    let intent = orders::Intent {
        request: a(20).request,
        quantity: support::q(1),
        minimum: support::p(90),
        maximum: support::p(110),
        maximum_fee_per_lot: atoms(0),
        reduce_only: false,
        time_in_force: orders::TimeInForce::GoodTilCancelled,
        policy: config().policy,
        authority_epoch: 1,
        expires_at: 1000,
    };
    let approval = orders::Approval {
        account: user(1),
        intent_hash: intent.digest().unwrap(),
        authority_epoch: 1,
    };
    let t = tx(
        &j,
        70,
        vec![],
        vec![Control::Order(orders::Action::Accept {
            intent: Box::new(intent),
            approval,
            reservations: vec![],
        })],
    );
    assert_eq!(
        j.commit(t).unwrap().receipt.controls,
        Some(ControlError::Unqualified)
    );
    credit(&c, &mut j, 63, 12, true);
    let need = c.funding_need(&mut j, 100, 20).unwrap();
    assert_eq!(
        (need.venue_usable, need.pending_ingress, need.additional),
        (20, 0, 0)
    );
    assert!(j.state().unwrap().native_funding_ready());
    bridge(&j);
}

#[test]
fn exact_wire_must_be_durable_and_cannot_be_replaced_after_restart() {
    let t = Temp::new();
    let (mut j, c, _) = setup(&t, false);
    prepare(&mut j, 10, Rail::Release, 20, 0);
    let action = c
        .expose_chain(
            &mut j,
            Dispatch {
                attempt: a(10),
                commit: id(40),
                at: 100,
            },
            Rail::Release,
            counters(Rail::Release, 0, 0),
        )
        .unwrap();
    let p = action.plan().clone();
    assert!(c.observe_chain(&mut j, id(60), 100, chain(&p, 60)).is_err());
    let binding = Sha256::digest(
        c.original_chain_contract(&mut j, a(10), 100)
            .unwrap()
            .as_bytes(),
    )
    .into();
    let good = VerifiedWire {
        attempt: a(10),
        binding,
        signature: [60; 64],
        wire: raw(b"verified synthetic original wire"),
    };
    c.persist_wire(&mut j, id(80), 100, action, good).unwrap();
    drop(j);
    let mut j = open(&t);
    assert!(
        c.expose_chain(
            &mut j,
            Dispatch {
                attempt: a(10),
                commit: id(81),
                at: 100
            },
            Rail::Release,
            counters(Rail::Release, 0, 0)
        )
        .is_err()
    );
    assert!(c.observe_chain(&mut j, id(61), 100, chain(&p, 61)).is_err());
    c.observe_chain(&mut j, id(60), 100, chain(&p, 60)).unwrap();
    bridge(&j);
}

struct KillTransport {
    marker: std::path::PathBuf,
}
#[test]
fn original_chain_capability_rechecks_grant_freeze_setup_and_wire_binding() {
    for case in 0..4 {
        let t = Temp::new();
        let (mut j, c, _) = setup(&t, false);
        prepare(&mut j, 10, Rail::Release, 20, 0);
        let action = c
            .expose_chain(
                &mut j,
                Dispatch {
                    attempt: a(10),
                    commit: id(40),
                    at: 100,
                },
                Rail::Release,
                counters(Rail::Release, 0, 0),
            )
            .unwrap();
        let plan = action.plan().clone();
        let binding = Sha256::digest(action.encode().unwrap().as_bytes()).into();
        let mut wire = VerifiedWire {
            attempt: a(10),
            binding,
            signature: [60; 64],
            wire: raw(b"synthetic original signed wire"),
        };
        match case {
            0 => {
                let tx = tx(
                    &j,
                    50,
                    vec![],
                    vec![Control::Order(orders::Action::AdvanceAuthority {
                        account: user(1),
                        epoch: 2,
                    })],
                );
                assert_eq!(j.commit(tx).unwrap().receipt.controls, None);
            }
            1 => {
                let tx = tx(
                    &j,
                    50,
                    vec![],
                    vec![Control::Funds(lifecycle::Action::Freeze)],
                );
                assert_eq!(j.commit(tx).unwrap().receipt.controls, None);
            }
            2 => {
                let mut s = good_setup();
                s.lending_disabled = false;
                assert!(!c.observe_setup(&mut j, id(50), 100, s).unwrap());
            }
            _ => wire.binding = [1; 32],
        }
        assert!(c.persist_wire(&mut j, id(80), 100, action, wire).is_err());
        assert!(j.transaction(id(80)).is_none());
        assert!(
            c.observe_chain(&mut j, id(60), 100, chain(&plan, 60))
                .is_err()
        );
        assert_eq!(j.state().unwrap().ledger().vault(), atoms(120));
        bridge(&j);
    }
}
impl Transport for KillTransport {
    fn post(&mut self, request: Outbound) -> Reply {
        // Validate the signature just like normal tests, then stop at the exact
        // external-effect boundary before any reply can be journaled.
        Fake::default().post(request);
        std::fs::write(&self.marker, b"one POST exposed").unwrap();
        loop {
            std::thread::park();
        }
    }
}
#[test]
#[ignore = "invoked only by bounded killed-child test"]
fn funding_post_child() {
    let db = std::env::var_os("CINDER_P16_TEST_DB").expect("parent-owned disposable path");
    let marker = std::env::var_os("CINDER_P16_TEST_MARKER").expect("parent-owned marker");
    let mut j = Journal::open(
        SqliteBackend::open(std::path::Path::new(&db), Migration::None).unwrap(),
        FixtureProtection,
        config(),
    )
    .unwrap();
    controller()
        .withdraw(
            &mut j,
            &gateway(),
            Dispatch {
                attempt: a(10),
                commit: id(40),
                at: 100,
            },
            &mut KillTransport {
                marker: marker.into(),
            },
        )
        .unwrap();
}
#[test]
fn kill_after_withdrawal_post_before_reply_reconciles_original_without_second_send() {
    use std::{
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let t = Temp::new();
    let (mut j, c, g) = setup(&t, true);
    prepare(&mut j, 10, Rail::Withdraw, 19, 1);
    drop(j);
    let marker = t.root.join("post-exposed");
    struct Child(std::process::Child);
    impl Drop for Child {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let mut child = Child(
        Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "funding_post_child", "--ignored", "--nocapture"])
            .env("CINDER_P16_TEST_DB", &t.db)
            .env("CINDER_P16_TEST_MARKER", &marker)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    let started = Instant::now();
    while !marker.exists() {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "child POST boundary timeout"
        );
        assert!(
            child.0.try_wait().unwrap().is_none(),
            "child exited before POST exposure"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    child.0.kill().unwrap();
    child.0.wait().unwrap();
    drop(child);
    let mut j = open(&t);
    assert!(
        j.state()
            .unwrap()
            .attempts()
            .iter()
            .any(|entry| entry.key == a(10) && entry.possibly_exposed)
    );
    assert!(
        j.state()
            .unwrap()
            .holds()
            .iter()
            .any(|h| h.request == a(10).request && h.active)
    );
    let mut f = Fake::default();
    assert!(
        c.withdraw(
            &mut j,
            &g,
            Dispatch {
                attempt: a(10),
                commit: id(41),
                at: 101
            },
            &mut f
        )
        .is_err()
    );
    assert_eq!(f.calls, 0);
    c.observe_withdrawal(&mut j, id(60), 101, withdrawal(10, 20, 19))
        .unwrap();
    assert_eq!(j.state().unwrap().ledger().broker(), atoms(19));
    c.observe_withdrawal(&mut j, id(61), 101, withdrawal(10, 20, 19))
        .unwrap();
    assert_eq!(j.state().unwrap().ledger().broker(), atoms(19));
    bridge(&j);
}
#[test]
fn broker_risk_requires_explicit_third_location_and_separate_accessibility() {
    use cinder_journal::risk;
    let t = Temp::new();
    let (mut j, c, _) = setup(&t, false);
    prepare(&mut j, 10, Rail::Release, 20, 0);
    let p = expose(&mut j, &c, 10, Rail::Release, 60);
    c.observe_chain(&mut j, id(60), 100, chain(&p, 60)).unwrap();
    let locations = |broker: bool| {
        let mut locations = vec![Location::Vault, Location::Venue];
        if broker {
            locations.push(Location::Broker)
        };
        locations
            .into_iter()
            .map(|location| risk::Liquidity {
                location,
                accessible_bps: 10000,
                due: atoms(0),
            })
            .collect()
    };
    let mut policy = risk::Policy {
        revision: config().policy,
        markets: vec![risk::MarketRule {
            market: config().markets[0].unit(),
            maximum_leverage: 100000,
            maintenance_bps: 500,
            native_maintenance_bps: 500,
            gross_limit: atoms(1000),
            net_limit: atoms(1000),
        }],
        buffer: atoms(0),
        horizon_ms: 100,
        valid_until: 1000,
        paths: vec![risk::Path {
            id: [1; 32],
            steps: vec![risk::Step {
                after_ms: 1,
                marks: vec![support::p(100)],
                events: vec![],
                liquidity: locations(false),
            }],
        }],
    };
    let install = |j: &Store, n, p: risk::Policy| {
        tx(
            j,
            n,
            vec![],
            vec![Control::Risk(risk::Action::Install {
                expected_version: j.state().unwrap().ledger().version(),
                policy: Box::new(p),
            })],
        )
    };
    let tx = install(&j, 70, policy.clone());
    assert_eq!(
        j.commit(tx).unwrap().receipt.controls,
        Some(ControlError::Invalid)
    );
    policy.paths[0].steps[0].liquidity = locations(true);
    let tx = install(&j, 71, policy.clone());
    assert_eq!(j.commit(tx).unwrap().receipt.controls, None);
    let report = j.state().unwrap().risk_report().unwrap();
    assert_eq!(report.current.broker_free, atoms(20));
    assert_eq!(report.current.venue_free, atoms(0));
    let broker = policy.paths[0].steps[0]
        .liquidity
        .iter_mut()
        .find(|x| x.location == Location::Broker)
        .unwrap();
    broker.accessible_bps = 0;
    broker.due = atoms(1);
    policy.revision = PolicyVersion::new(2).unwrap();
    let tx = install(&j, 72, policy);
    assert_eq!(j.commit(tx).unwrap().receipt.controls, None);
    let report = j.state().unwrap().risk_report().unwrap();
    assert!(!report.admissible);
    assert!(report.paths[0].prefixes[1].flags.illiquid);
    assert_eq!(report.paths[0].prefixes[1].broker_free, atoms(-1));
    bridge(&j);
}

fn config() -> Config {
    let mut c = support::config();
    for (tag, location) in [(8, Location::Vault), (9, Location::Broker)] {
        c.sources.push(Source {
            scope: EventScope {
                namespace: NamespaceId::new([tag; 32]).unwrap(),
                ..c.sources[0].scope
            },
            location,
        });
    }
    c
}
fn account() -> [u8; 32] {
    SigningKey::from_bytes(&[9; 32]).verifying_key().to_bytes()
}
fn profile() -> Profile {
    Profile {
        config: config(),
        source: config().sources[0].scope,
        account: bs58::encode(account()).into_string(),
        environment: Origin::Testnet.url().into(),
        revision: 1,
        evidence: "offline funds fixture".into(),
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
    }
}
fn route() -> Route {
    Route {
        domain: config().domain.deployment.bytes(),
        pool: [18; 32],
        funds: [19; 32],
        beneficiaries: vec![
            Beneficiary {
                account: user(1).bytes(),
                wallet: [11; 32],
                tokens: [34; 32],
            },
            Beneficiary {
                account: user(2).bytes(),
                wallet: [12; 32],
                tokens: [35; 32],
            },
        ],
        program: [20; 32],
        config: [21; 32],
        vault: [22; 32],
        mint: [23; 32],
        broker: account(),
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
        setup_max_age: 10_000,
    }
}
fn controller() -> Controller {
    Controller::new(profile(), route(), Zeroizing::new([9; 32])).unwrap()
}
fn gateway() -> Gateway {
    Gateway::new(
        profile(),
        Policy {
            revision: 1,
            evidence: "offline shared credits".into(),
            execution: Level::Qualified,
            origin: Origin::Testnet,
            expiry_ms: 30_000,
            credits: 100,
            cleanup_reserve: 20,
            read_cost: 10,
        },
        Zeroizing::new([7; 32]),
        1,
    )
    .unwrap()
}
fn id(n: u8) -> CommitId {
    CommitId::new([n; 32]).unwrap()
}
fn a(n: u8) -> AttemptKey {
    support::attempt(n)
}
fn atoms(n: i128) -> QuoteAtoms {
    QuoteAtoms::new(config().quote, n)
}
fn source(location: Location) -> EventScope {
    config()
        .sources
        .into_iter()
        .find(|s| s.location == location)
        .unwrap()
        .scope
}
fn key(n: u8, location: Location) -> EventKey {
    EventKey {
        scope: source(location),
        event: EconomicEventId::new(&[n]).unwrap(),
        leg: 0,
    }
}
fn tx(j: &Store, n: u8, events: Vec<(Location, Change)>, controls: Vec<Control>) -> Transaction {
    Transaction {
        id: id(n),
        expected: j.head(),
        at: 100,
        evidence: vec![],
        inputs: events
            .into_iter()
            .enumerate()
            .map(|(i, (l, c))| {
                let k = EventKey {
                    leg: u32::try_from(i).unwrap(),
                    ..key(n, l)
                };
                Input {
                    source: k.scope,
                    source_cut: Some(100),
                    authority_epoch: 1,
                    observed_at: 100,
                    raw: raw(b"synthetic trusted observation"),
                    event: Some(Event {
                        key: RecordKey::Economic(k),
                        policy: config().policy,
                        change: c,
                    }),
                }
            })
            .collect(),
        order_observations: vec![],
        funds_observations: vec![],
        controls,
    }
}
fn open(t: &Temp) -> Store {
    Journal::open(
        SqliteBackend::open(&t.db, Migration::None).unwrap(),
        FixtureProtection,
        config(),
    )
    .unwrap()
}
fn setup(t: &Temp, native: bool) -> (Store, Controller, Gateway) {
    let mut j = Journal::create(
        SqliteBackend::create(&t.db).unwrap(),
        FixtureProtection,
        config(),
    )
    .unwrap();
    let c = controller();
    let g = gateway();
    let customer_location = if native {
        Location::Venue
    } else {
        Location::Vault
    };
    let t = tx(
        &j,
        1,
        vec![
            (
                customer_location,
                Change::Receipt {
                    owner: Owner::Customer(user(1)),
                    location: customer_location,
                    amount: atoms(20),
                },
            ),
            (
                Location::Vault,
                Change::Receipt {
                    owner: Owner::House,
                    location: Location::Vault,
                    amount: atoms(100),
                },
            ),
        ],
        vec![Control::Order(orders::Action::AdvanceAuthority {
            account: user(1),
            epoch: 1,
        })],
    );
    assert_eq!(j.commit(t).unwrap().receipt.controls, None);
    let l = j.state().unwrap().ledger();
    let version = l.version();
    let check = NativeCheck {
        expected_version: version,
        cash: Some(l.venue().cash()),
        funding: Some(l.venue().funding()),
        positions: Some(l.venue().positions().to_vec()),
        complete: true,
        resolves: vec![],
    };
    let cut = collateral::Cut {
        expected_version: version + 1,
        policy: collateral::Policy {
            revision: config().policy,
            markets: vec![collateral::MarginRule {
                market: config().markets[0].unit(),
                private_bps: 1000,
                native_bps: 1000,
            }],
            evidence: EvidencePolicy {
                max_issue_age: 10_000,
                max_mark_age: 10_000,
                max_check_age: 10_000,
            },
        },
        marks: vec![MarkObservation {
            price: support::p(100),
            evidence: key(2, Location::Venue),
            observed_at: 100,
            valid_until: 10_100,
            qualified: true,
        }],
    };
    let t = tx(
        &j,
        2,
        vec![(Location::Venue, Change::Reconcile(check))],
        vec![Control::Collateral(cut)],
    );
    assert_eq!(j.commit(t).unwrap().receipt.controls, None);
    c.bind(&mut j, id(3), 100).unwrap();
    g.activate(&mut j, id(4), 100).unwrap();
    assert!(c.observe_setup(&mut j, id(5), 100, good_setup()).unwrap());
    (j, c, g)
}
fn good_setup() -> Setup {
    Setup {
        account: account(),
        observed_at: 100,
        lending_disabled: true,
        borrowed: "0".into(),
        interest: "0".into(),
        complete: true,
    }
}
fn prepare(j: &mut Store, n: u8, rail: Rail, net: i128, fee: i128) {
    let (source, destination) = match rail {
        Rail::Release => (Location::Vault, Destination::Location(Location::Broker)),
        Rail::Deposit => (Location::Broker, Destination::Location(Location::Venue)),
        Rail::Withdraw => (Location::Venue, Destination::Location(Location::Broker)),
        Rail::Return => (Location::Broker, Destination::Location(Location::Vault)),
        Rail::Payout => (Location::Vault, Destination::Recipient([11; 32])),
    };
    let i = lifecycle::Intent {
        request: a(n).request,
        source,
        destination,
        net: atoms(net),
        maximum_fee: atoms(fee),
        fee_payer: Owner::House,
        allow_partial: false,
        policy: config().policy,
        authority_epoch: 1,
        expires_at: 10_000,
    };
    let approval = orders::Approval {
        account: user(1),
        intent_hash: i.digest().unwrap(),
        authority_epoch: 1,
    };
    let t = tx(
        j,
        n,
        vec![],
        vec![
            Control::Funds(lifecycle::Action::Accept {
                intent: Box::new(i),
                approval,
            }),
            Control::Funds(lifecycle::Action::Prepare {
                attempt: a(n),
                net: atoms(net),
            }),
        ],
    );
    assert_eq!(j.commit(t).unwrap().receipt.controls, None);
}
fn counters(rail: Rail, paid: u64, sequence: u64) -> Counters {
    Counters {
        paid,
        sequence,
        recipient_tokens: if rail == Rail::Payout {
            [34; 32]
        } else {
            [0; 32]
        },
        expires_at_slot: 500,
    }
}
fn expose(j: &mut Store, c: &Controller, n: u8, rail: Rail, signature: u8) -> Plan {
    let action = c
        .expose_chain(
            j,
            Dispatch {
                attempt: a(n),
                commit: id(n + 30),
                at: 100,
            },
            rail,
            counters(rail, 0, 0),
        )
        .unwrap();
    signed_fixture(c, j, action, signature)
}
fn chain(p: &Plan, n: u8) -> ChainReceipt {
    let r = route();
    let (program, from, to) = match p.rail() {
        Rail::Release => (r.program, r.vault, r.broker_tokens),
        Rail::Deposit => (r.venue_program, r.broker_tokens, r.venue_vault),
        Rail::Return => (r.program, r.broker_tokens, r.vault),
        Rail::Payout => (r.program, r.vault, p.counters().recipient_tokens),
        Rail::Withdraw => (r.venue_program, r.venue_vault, r.broker_tokens),
    };
    ChainReceipt {
        network: config().domain.network.bytes(),
        attempt: p.attempt().unwrap(),
        mint: r.mint,
        program,
        source: from,
        destination: to,
        amount: p.amount(),
        succeeded: true,
        signature: [n; 64],
        slot: 200,
        operation: if p.rail() == Rail::Deposit {
            None
        } else {
            Some(p.operation())
        },
        epoch: Some(p.epoch()),
        config: Some(r.config),
        paid: if p.rail() == Rail::Payout {
            Some(p.counters().paid + p.amount())
        } else {
            None
        },
        sequence: Some(p.counters().sequence + 1),
        raw: raw(b"finalized synthetic token/program receipt"),
    }
}
// Only a synthetic trusted codec port here. Real signed Solana wires are checked
// independently by the TypeScript client tests; this fixture is NOT crypto proof.
fn signed_fixture(c: &Controller, j: &mut Store, action: ChainAction, signature: u8) -> Plan {
    let plan = action.plan().clone();
    let contract = action.encode().unwrap();
    let wire = VerifiedWire {
        attempt: plan.attempt().unwrap(),
        binding: Sha256::digest(contract.as_bytes()).into(),
        signature: [signature; 64],
        wire: raw(b"synthetic codec-verified wire"),
    };
    let mut h = Sha256::new();
    h.update(b"synthetic wire commit");
    h.update(plan.attempt().unwrap().encode());
    let wire_id = CommitId::new(h.finalize().into()).unwrap();
    c.persist_wire(j, wire_id, 100, action, wire).unwrap();
    plan
}
fn bridge(j: &Store) {
    let l = j.state().unwrap().ledger();
    l.check_bridge().unwrap();
    let d = l.diagnostics(&[support::p(100)]).unwrap();
    assert_eq!(
        d.net_assets,
        d.customer_claims.checked_add(d.house_equity).unwrap()
    );
}
fn credit(c: &Controller, j: &mut Store, n: u8, amount: u64, final_credit: bool) {
    c.observe_credit(
        j,
        id(n),
        100,
        Credit {
            attempt: a(11),
            account: account(),
            deposit_signature: [61; 64],
            event: EconomicEventId::new(&[n]).unwrap(),
            cut: 300,
            amount,
            fee: 0,
            final_credit,
            raw: raw(b"operation-linked native deposit credit"),
        },
    )
    .unwrap();
}
#[derive(Default)]
struct Fake {
    calls: usize,
    status: Option<u16>,
}
impl Transport for Fake {
    fn post(&mut self, r: Outbound) -> Reply {
        self.calls += 1;
        assert_eq!(r.origin(), Origin::Testnet);
        assert_eq!(r.path(), "/api/v1/account/withdraw");
        let mut b: Value = serde_json::from_slice(r.body()).unwrap();
        let signature: [u8; 64] = bs58::decode(b["signature"].as_str().unwrap())
            .into_vec()
            .unwrap()
            .try_into()
            .unwrap();
        let preimage = json!({"data":{"amount":b["amount"].take(),"idempotency_key":b["idempotency_key"].take()},"expiry_window":b["expiry_window"],"timestamp":b["timestamp"],"type":"withdraw"});
        VerifyingKey::from_bytes(&account())
            .unwrap()
            .verify_strict(
                serde_json::to_string(&preimage).unwrap().as_bytes(),
                &Signature::from_bytes(&signature),
            )
            .unwrap();
        assert_eq!(b["account"], bs58::encode(account()).into_string());
        assert!(b.get("agent_wallet").is_none());
        match self.status {
            Some(status) => Reply::Response {
                status,
                body: raw(br#"{"success":true,"data":{"batch_nonce":42}}"#),
                received_at: 100,
                retry_after_ms: None,
            },
            None => Reply::Unknown,
        }
    }
}
fn withdraw(c: &Controller, j: &mut Store, g: &Gateway, n: u8, fake: &mut Fake) {
    c.withdraw(
        j,
        g,
        Dispatch {
            attempt: a(n),
            commit: id(n + 30),
            at: 100,
        },
        fake,
    )
    .unwrap();
}
fn payment(n: u8, net: u64) -> ChainReceipt {
    ChainReceipt {
        network: config().domain.network.bytes(),
        attempt: a(n),
        mint: route().mint,
        program: route().venue_program,
        source: route().venue_vault,
        destination: route().broker_tokens,
        amount: net,
        succeeded: true,
        signature: [77; 64],
        slot: 400,
        operation: None,
        epoch: None,
        config: None,
        paid: None,
        sequence: None,
        raw: raw(b"finalized owner-wallet transfer"),
    }
}
fn withdrawal(n: u8, gross: u64, net: u64) -> Withdrawal {
    Withdrawal {
        attempt: a(n),
        idempotency_key: withdrawal_id(a(n)),
        event: EconomicEventId::new(&[88]).unwrap(),
        complete: true,
        cut: 350,
        gross,
        payment: payment(n, net),
        raw: raw(b"qualified complete original-intent withdrawal history"),
    }
}

#[test]
fn complete_round_trip_uses_one_ledger_and_actual_fees_with_no_second_credit() {
    let t = Temp::new();
    let (mut j, c, g) = setup(&t, false);
    prepare(&mut j, 10, Rail::Release, 20, 0);
    let release = expose(&mut j, &c, 10, Rail::Release, 60);
    c.observe_chain(&mut j, id(60), 100, chain(&release, 60))
        .unwrap();
    bridge(&j);
    assert_eq!(j.state().unwrap().ledger().broker(), atoms(20));
    prepare(&mut j, 11, Rail::Deposit, 20, 0);
    let deposit = expose(&mut j, &c, 11, Rail::Deposit, 61);
    assert!(!j.state().unwrap().native_funding_ready());
    c.observe_chain(&mut j, id(61), 100, chain(&deposit, 61))
        .unwrap();
    bridge(&j);
    assert_eq!(j.state().unwrap().ledger().venue().cash(), atoms(0));
    assert_eq!(j.state().unwrap().ledger().in_transit().unwrap(), atoms(20));
    credit(&c, &mut j, 62, 8, false);
    assert!(!j.state().unwrap().native_funding_ready());
    bridge(&j);
    credit(&c, &mut j, 63, 12, true);
    assert!(j.state().unwrap().native_funding_ready());
    bridge(&j);
    assert_eq!(
        j.state()
            .unwrap()
            .ledger()
            .book(Owner::Customer(user(1)))
            .unwrap()
            .cash(),
        atoms(20)
    );
    prepare(&mut j, 12, Rail::Withdraw, 19, 1);
    let mut fake = Fake {
        status: Some(200),
        ..Fake::default()
    };
    withdraw(&c, &mut j, &g, 12, &mut fake);
    assert_eq!(j.state().unwrap().ledger().venue().cash(), atoms(20));
    assert!(!j.state().unwrap().funds().last().unwrap().terminal);
    drop(j);
    let mut j = open(&t);
    assert!(
        c.withdraw(
            &mut j,
            &g,
            Dispatch {
                attempt: a(12),
                commit: id(90),
                at: 100
            },
            &mut fake
        )
        .is_err()
    );
    assert_eq!(fake.calls, 1);
    c.observe_withdrawal(&mut j, id(64), 100, withdrawal(12, 20, 19))
        .unwrap();
    bridge(&j);
    assert_eq!(
        j.state()
            .unwrap()
            .ledger()
            .book(Owner::House)
            .unwrap()
            .cash(),
        atoms(99)
    );
    assert_eq!(
        j.state()
            .unwrap()
            .ledger()
            .book(Owner::Customer(user(1)))
            .unwrap()
            .cash(),
        atoms(20)
    );
    prepare(&mut j, 13, Rail::Return, 19, 0);
    let returned = expose(&mut j, &c, 13, Rail::Return, 65);
    c.observe_chain(&mut j, id(65), 100, chain(&returned, 65))
        .unwrap();
    bridge(&j);
    prepare(&mut j, 14, Rail::Payout, 19, 0);
    let payout = expose(&mut j, &c, 14, Rail::Payout, 66);
    assert_eq!(payout.customer(), [11; 32]);
    c.observe_chain(&mut j, id(66), 100, chain(&payout, 66))
        .unwrap();
    bridge(&j);
    assert_eq!(
        j.state().unwrap().ledger().paid(user(1)).unwrap(),
        atoms(19)
    );
    assert_eq!(
        j.state()
            .unwrap()
            .ledger()
            .book(Owner::Customer(user(1)))
            .unwrap()
            .cash(),
        atoms(1)
    );
    let residual = c.residuals(&mut j, 100).unwrap();
    assert_eq!(residual.vault, atoms(100));
    assert_eq!(residual.broker, atoms(0));
    assert_eq!(residual.native, atoms(0));
    assert_eq!(residual.transit, atoms(0));
    assert!(residual.unresolved.is_empty());
    c.observe_chain(&mut j, id(67), 100, chain(&payout, 66))
        .unwrap();
    assert_eq!(
        j.state().unwrap().ledger().paid(user(1)).unwrap(),
        atoms(19)
    );
    bridge(&j);
}
#[test]
fn immutable_routes_private_identity_mapping_and_qualified_caps_have_no_fallback() {
    for f in [0, 1, 2, 3, 4, 5, 6, 7] {
        let mut r = route();
        match f {
            0 => r.chain = Level::Observed,
            1 => r.withdrawal = Level::Documented,
            2 => r.settings = Level::Unknown,
            3 => r.broker = [1; 32],
            4 => r.broker_tokens = r.vault,
            5 => r.decimals = 1,
            6 => r.beneficiaries[1].wallet = r.beneficiaries[0].wallet,
            _ => r.maximum_fee = r.maximum_movement,
        };
        assert!(Controller::new(profile(), r, Zeroizing::new([9; 32])).is_err());
    }
    let t = Temp::new();
    let (mut j, c, _) = setup(&t, false);
    assert!(c.bind(&mut j, id(50), 100).is_err());
    let mut r = route();
    r.epoch = 2;
    let changed = Controller::new(profile(), r, Zeroizing::new([9; 32])).unwrap();
    assert!(changed.residuals(&mut j, 100).is_err());
}
#[test]
fn missing_lending_disable_debt_interest_and_stale_setup_block_funding_and_native_orders() {
    for case in 0..5 {
        let t = Temp::new();
        let (mut j, c, _) = setup(&t, false);
        let mut s = good_setup();
        match case {
            0 => s.lending_disabled = false,
            1 => s.borrowed = "1".into(),
            2 => s.interest = "1".into(),
            3 => s.complete = false,
            _ => s.borrowed = "not-a-number".into(),
        };
        assert!(!c.observe_setup(&mut j, id(6), 100, s).unwrap());
        assert!(!j.state().unwrap().native_funding_ready());
        prepare(&mut j, 10, Rail::Release, 20, 0);
        assert!(
            c.expose_chain(
                &mut j,
                Dispatch {
                    attempt: a(10),
                    commit: id(40),
                    at: 100
                },
                Rail::Release,
                counters(Rail::Release, 0, 0)
            )
            .is_err()
        );
    }
    let t = Temp::new();
    let (mut j, c, _) = setup(&t, false);
    prepare(&mut j, 10, Rail::Release, 20, 0);
    assert!(
        c.expose_chain(
            &mut j,
            Dispatch {
                attempt: a(10),
                commit: id(40),
                at: 10_101
            },
            Rail::Release,
            counters(Rail::Release, 0, 0)
        )
        .is_err()
    );
}
#[test]
fn wrong_network_program_mint_route_receipt_epoch_counters_never_post_a_credit() {
    for case in 0..9 {
        let t = Temp::new();
        let (mut j, c, _) = setup(&t, false);
        prepare(&mut j, 10, Rail::Release, 20, 0);
        let p = expose(&mut j, &c, 10, Rail::Release, 60);
        let mut r = chain(&p, 60);
        match case {
            0 => r.network = [99; 32],
            1 => r.program = [99; 32],
            2 => r.mint = [99; 32],
            3 => r.source = [99; 32],
            4 => r.destination = [99; 32],
            5 => r.operation = Some([99; 32]),
            6 => r.epoch = Some(2),
            7 => r.sequence = Some(2),
            _ => r.config = Some([99; 32]),
        };
        let before = j.state().unwrap().clone();
        assert!(c.observe_chain(&mut j, id(60), 100, r).is_err());
        assert_eq!(j.state().unwrap(), &before);
        assert_eq!(j.state().unwrap().ledger().broker(), atoms(0));
    }
}
#[test]
fn finalized_failed_cpi_closes_zero_effect_without_counter_or_claim_consumption() {
    let t = Temp::new();
    let (mut j, c, _) = setup(&t, false);
    prepare(&mut j, 10, Rail::Payout, 19, 0);
    let p = expose(&mut j, &c, 10, Rail::Payout, 60);
    let mut r = chain(&p, 60);
    r.succeeded = false;
    r.amount = 0;
    r.operation = None;
    r.epoch = None;
    r.config = None;
    r.paid = None;
    r.sequence = None;
    c.observe_chain(&mut j, id(60), 100, r).unwrap();
    assert_eq!(j.state().unwrap().ledger().paid(user(1)).unwrap(), atoms(0));
    assert_eq!(j.state().unwrap().ledger().vault(), atoms(120));
    assert!(j.state().unwrap().funds()[0].terminal);
    bridge(&j);
}
#[test]
fn payout_uses_persistent_paid_cas_and_public_wallet_not_private_account_id() {
    let t = Temp::new();
    let (mut j, c, _) = setup(&t, false);
    prepare(&mut j, 10, Rail::Payout, 5, 0);
    let p = expose(&mut j, &c, 10, Rail::Payout, 60);
    c.observe_chain(&mut j, id(60), 100, chain(&p, 60)).unwrap();
    prepare(&mut j, 11, Rail::Payout, 5, 0);
    assert!(
        c.expose_chain(
            &mut j,
            Dispatch {
                attempt: a(11),
                commit: id(41),
                at: 100
            },
            Rail::Payout,
            counters(Rail::Payout, 0, 0)
        )
        .is_err()
    );
    let action = c
        .expose_chain(
            &mut j,
            Dispatch {
                attempt: a(11),
                commit: id(41),
                at: 100,
            },
            Rail::Payout,
            counters(Rail::Payout, 5, 1),
        )
        .unwrap();
    let p = signed_fixture(&c, &mut j, action, 61);
    let mut r = chain(&p, 61);
    r.paid = Some(5);
    assert!(c.observe_chain(&mut j, id(61), 100, r).is_err());
    c.observe_chain(&mut j, id(61), 100, chain(&p, 61)).unwrap();
    assert_eq!(
        j.state().unwrap().ledger().paid(user(1)).unwrap(),
        atoms(10)
    );
}
#[test]
fn withdrawal_ack_timeout_error_and_empty_observations_never_release_or_resend() {
    for status in [None, Some(200), Some(409), Some(429), Some(500)] {
        let t = Temp::new();
        let (mut j, c, g) = setup(&t, true);
        prepare(&mut j, 10, Rail::Withdraw, 19, 1);
        let mut f = Fake {
            status,
            ..Fake::default()
        };
        withdraw(&c, &mut j, &g, 10, &mut f);
        drop(j);
        let mut j = open(&t);
        assert_eq!(j.state().unwrap().ledger().venue().cash(), atoms(20));
        assert_eq!(
            j.state()
                .unwrap()
                .reserved(Resource::Location(Location::Venue))
                .unwrap(),
            atoms(20)
        );
        assert!(
            c.withdraw(
                &mut j,
                &g,
                Dispatch {
                    attempt: a(10),
                    commit: id(91),
                    at: 100
                },
                &mut f
            )
            .is_err()
        );
        assert_eq!(f.calls, 1);
        let mut w = withdrawal(10, 20, 19);
        w.complete = false;
        assert!(c.observe_withdrawal(&mut j, id(60), 100, w).is_err());
    }
}
#[test]
fn withdrawal_matches_original_uuid_and_owner_tokens_and_accounts_actual_overrun() {
    let t = Temp::new();
    let (mut j, c, g) = setup(&t, true);
    prepare(&mut j, 10, Rail::Withdraw, 19, 1);
    withdraw(&c, &mut j, &g, 10, &mut Fake::default());
    let mut w = withdrawal(10, 20, 19);
    w.idempotency_key = withdrawal_id(a(11));
    assert!(c.observe_withdrawal(&mut j, id(60), 100, w).is_err());
    let mut w = withdrawal(10, 20, 19);
    w.payment.destination = [99; 32];
    assert!(c.observe_withdrawal(&mut j, id(60), 100, w).is_err());
    // A real extra fee belongs to house and contains the broken bound, not erased data.
    assert!(
        c.observe_withdrawal(&mut j, id(60), 100, withdrawal(10, 20, 18))
            .is_err()
    );
    bridge(&j);
    assert_eq!(j.state().unwrap().ledger().broker(), atoms(18));
    assert_eq!(
        j.state()
            .unwrap()
            .ledger()
            .book(Owner::House)
            .unwrap()
            .cash(),
        atoms(98)
    );
    assert!(j.state().unwrap().funds()[0].faulted);
}
#[test]
fn withdrawal_uses_shared_read_cleanup_credits_and_rate_limit_backoff() {
    let t = Temp::new();
    let (mut j, c, g) = setup(&t, true);
    for n in 100..110 {
        g.reserve_read(&mut j, id(n), 100, true).unwrap();
    }
    prepare(&mut j, 10, Rail::Withdraw, 19, 1);
    let mut f = Fake::default();
    assert_eq!(
        c.withdraw(
            &mut j,
            &g,
            Dispatch {
                attempt: a(10),
                commit: id(40),
                at: 100
            },
            &mut f
        ),
        Err(AdapterError::Limit)
    );
    assert_eq!(f.calls, 0);
    let t = Temp::new();
    let (mut j, c, g) = setup(&t, true);
    prepare(&mut j, 10, Rail::Withdraw, 19, 1);
    withdraw(
        &c,
        &mut j,
        &g,
        10,
        &mut Fake {
            status: Some(429),
            ..Fake::default()
        },
    );
    assert!(g.reserve_read(&mut j, id(100), 100, true).is_err());
}
#[test]
fn persisted_native_key_fence_prevents_funding_signature() {
    let t = Temp::new();
    let (mut j, c, g) = setup(&t, true);
    prepare(&mut j, 10, Rail::Withdraw, 19, 1);
    g.deactivate(&mut j, id(90), 100).unwrap();
    let mut f = Fake::default();
    assert!(
        c.withdraw(
            &mut j,
            &g,
            Dispatch {
                attempt: a(10),
                commit: id(40),
                at: 100
            },
            &mut f
        )
        .is_err()
    );
    assert_eq!(f.calls, 0);
}
#[test]
fn partial_native_credit_preserves_transit_and_requires_original_chain_correlation() {
    let t = Temp::new();
    let (mut j, c, _) = setup(&t, false);
    prepare(&mut j, 10, Rail::Release, 20, 0);
    let p = expose(&mut j, &c, 10, Rail::Release, 60);
    c.observe_chain(&mut j, id(60), 100, chain(&p, 60)).unwrap();
    prepare(&mut j, 11, Rail::Deposit, 20, 0);
    let p = expose(&mut j, &c, 11, Rail::Deposit, 61);
    c.observe_chain(&mut j, id(61), 100, chain(&p, 61)).unwrap();
    let wrong = Credit {
        attempt: a(11),
        account: account(),
        deposit_signature: [99; 64],
        event: EconomicEventId::new(&[62]).unwrap(),
        cut: 300,
        amount: 20,
        fee: 0,
        final_credit: true,
        raw: raw(b"wrong original deposit"),
    };
    assert!(c.observe_credit(&mut j, id(62), 100, wrong).is_err());
    credit(&c, &mut j, 62, 8, false);
    let r = c.residuals(&mut j, 100).unwrap();
    assert_eq!(r.native, atoms(8));
    assert_eq!(r.transit, atoms(12));
    assert_eq!(r.unresolved, [a(11)]);
    assert!(!j.state().unwrap().native_funding_ready());
    bridge(&j);
}
#[test]
fn broker_source_is_nonnegative_and_not_venue_or_vault_payout_capacity() {
    let t = Temp::new();
    let (mut j, c, _) = setup(&t, false);
    prepare(&mut j, 10, Rail::Release, 20, 0);
    let p = expose(&mut j, &c, 10, Rail::Release, 60);
    c.observe_chain(&mut j, id(60), 100, chain(&p, 60)).unwrap();
    assert_eq!(j.state().unwrap().ledger().vault(), atoms(100));
    assert_eq!(j.state().unwrap().ledger().broker(), atoms(20));
    assert_eq!(j.state().unwrap().ledger().venue().cash(), atoms(0));
    let i = lifecycle::Intent {
        request: a(11).request,
        source: Location::Broker,
        destination: Destination::Location(Location::Venue),
        net: atoms(21),
        maximum_fee: atoms(0),
        fee_payer: Owner::House,
        allow_partial: false,
        policy: config().policy,
        authority_epoch: 1,
        expires_at: 1000,
    };
    let approval = orders::Approval {
        account: user(1),
        intent_hash: i.digest().unwrap(),
        authority_epoch: 1,
    };
    let t = tx(
        &j,
        11,
        vec![],
        vec![Control::Funds(lifecycle::Action::Accept {
            intent: Box::new(i),
            approval,
        })],
    );
    assert_eq!(
        j.commit(t).unwrap().receipt.controls,
        Some(ControlError::Capacity)
    );
    bridge(&j);
}
