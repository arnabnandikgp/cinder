//! Existing accepted intents through the actual chain codec/signer/retention
//! boundary. Only RPC, authenticated setup replies and clock are synthetic.
use super::*;
use cinder_pacifica::{
    funding::{demo, setup},
    reads,
};

pub(crate) fn policy() -> demo::Policy {
    demo::Policy {
        revision: 1,
        maximum_reads: 8,
        maximum_pages: 2,
        interval_ms: 1000,
        maximum_backoff_ms: 4000,
        lifetime_ms: 9000,
        initial_setup: Some(demo::InitialSetup {
            revision: 1,
            exclusive_control: true,
        }),
    }
}
pub(crate) fn gateway() -> Gateway {
    Gateway::new(
        profile(),
        Policy {
            revision: 1,
            evidence: "OFFLINE approved demo port fixture only".into(),
            execution: Level::Qualified,
            origin: Origin::Testnet,
            expiry_ms: 3000,
            credits: 10000,
            cleanup_reserve: 100,
            read_cost: reads::MIN_READ_COST,
        },
        Zeroizing::new([10; 32]),
        1,
    )
    .unwrap()
}
pub(crate) fn qualify<B: Backend, P: Protection>(
    j: &mut Journal<B, P>,
    c: &Controller,
    g: &Gateway,
) {
    c.observe_setup(
        j,
        id(40),
        100,
        Setup {
            account: route().broker,
            observed_at: 100,
            lending_disabled: true,
            borrowed: "0".into(),
            interest: "0".into(),
            complete: false,
        },
    )
    .unwrap();
    g.initialize_reads(j, id(41), 100).unwrap();
    let bodies = [
        json!({"success":true,"data":{"auto_lend_disabled":true,"margin_settings":[],"spot_settings":[]}}),
        json!({"success":true,"data":{"borrowed":"0","pending_interest":"0","spot_balances":[],"updated_at":100}}),
        json!({"success":true,"data":{
            "balance":"0","account_equity":"0","available_to_spend":"0","available_to_withdraw":"0",
            "pending_balance":"0","pending_interest":"0","total_margin_used":"0","cross_mmr":"0",
            "spot_collateral":"0","spot_market_value":"0","cross_account_equity":null,
            "positions_count":0,"orders_count":0,"stop_orders_count":0,"spot_balances":[],"updated_at":100
        }}),
    ];
    for (i, body) in bodies.into_iter().enumerate() {
        let at = 100 + i as u64 * 1000;
        let setup::Step::Request(request, completion) = c
            .prepare_setup_poll(
                j,
                g,
                &policy(),
                setup::Poll {
                    reservation: id(50 + i as u8 * 2),
                    evidence: id(51 + i as u8 * 2),
                    at,
                },
            )
            .unwrap_or_else(|_| panic!("setup preparation refused at round index {i}"))
        else {
            panic!("expected setup GET");
        };
        assert_eq!(request.consume(at).unwrap().origin(), Origin::Testnet);
        reads::complete(
            j,
            g,
            *completion,
            Reply::Response {
                status: 200,
                received_at: at,
                retry_after_ms: None,
                body: PrivateBytes::new(serde_json::to_vec(&body).unwrap()).unwrap(),
            },
        )
        .unwrap();
    }
    assert!(
        c.observe_setup_reads(j, g, &policy(), id(60), 2100)
            .unwrap()
    );
    assert!(c.bound(j, 2100).unwrap());
    assert!(!j.state().unwrap().native_funding_ready());
}
fn new_store(temp: &support::Temp, location: Location) -> (support::Store, Controller) {
    seed_at(
        Journal::create(
            SqliteBackend::create(&temp.db).unwrap(),
            support::FixtureProtection,
            config(),
        )
        .unwrap(),
        location,
    )
}
#[test]
fn demo_original_release_and_deposit_use_real_simulation_signing_and_one_shot_wire_boundary() {
    for rail in [Rail::Release, Rail::Deposit] {
        let temp = support::Temp::new();
        let (mut j, c) = new_store(
            &temp,
            if rail == Rail::Release {
                Location::Vault
            } else {
                Location::Broker
            },
        );
        prepare(&mut j, 10, rail);
        let g = gateway();
        qualify(&mut j, &c, &g);
        let f = fake(10);
        f.0.lock().unwrap().at = 2100;
        let clock = Arc::new(Advancing(AtomicU64::new(2100)));
        let mut p = loaded(&c, vec![]).with_transport(f.clone(), clock).unwrap();
        // An approved demo snapshot is not strong setup. The original strong
        // path still refuses without exposing a plan, signature or new hold.
        let head = j.head();
        assert!(p.issue(&mut j, &c, support::attempt(10)).is_err());
        assert_eq!(j.head(), head);
        assert!(!j.state().unwrap().attempts()[0].possibly_exposed);
        assert!(
            p.issue_demo_ingress(&mut j, &c, &g, &policy(), support::attempt(10))
                .unwrap()
                == Outcome::Submitted
        );
        let retained = c
            .retained_wire(&mut j, support::attempt(10), 2100)
            .unwrap()
            .unwrap();
        assert_eq!(
            retained.wire.as_bytes(),
            f.0.lock().unwrap().wire.as_ref().unwrap()
        );
        f.0.lock().unwrap().contract = Some(
            c.original_chain_contract(&mut j, support::attempt(10), 2100)
                .unwrap()
                .as_bytes()
                .to_vec(),
        );
        assert!(
            p.issue_demo_ingress(&mut j, &c, &g, &policy(), support::attempt(10))
                .is_err()
        );
        assert!(p.issue(&mut j, &c, support::attempt(10)).is_err());
        assert!(
            p.reconcile(&mut j, &c, support::attempt(10)).unwrap()
                == if rail == Rail::Release {
                    Outcome::Settled
                } else {
                    Outcome::Pending
                }
        );
        assert!(!j.state().unwrap().native_funding_ready());
        assert_eq!(j.state().unwrap().ledger().venue().cash().atoms(), 0);
        assert_eq!(j.state().unwrap().funds().len(), 1);
        let rpc = f.0.lock().unwrap();
        assert_eq!(
            rpc.requests
                .iter()
                .filter(|r| r["method"] == "simulateTransaction")
                .count(),
            1
        );
        assert_eq!(
            rpc.requests
                .iter()
                .filter(|r| r["method"] == "sendTransaction")
                .count(),
            1
        );
    }
}
#[test]
fn demo_issuance_refuses_unapproved_setup_and_non_ingress_before_rpc_or_exposure() {
    for rail in [Rail::Release, Rail::Deposit, Rail::Return, Rail::Payout] {
        let temp = support::Temp::new();
        let (mut j, c) = new_store(
            &temp,
            if rail == Rail::Deposit || rail == Rail::Return {
                Location::Broker
            } else {
                Location::Vault
            },
        );
        prepare(&mut j, 10, rail);
        let f = fake(10);
        let mut p = port(&c, &f);
        let head = j.head();
        assert!(
            p.issue_demo_ingress(&mut j, &c, &gateway(), &policy(), support::attempt(10))
                .is_err()
        );
        assert_eq!(j.head(), head);
        assert!(f.0.lock().unwrap().requests.is_empty());
        assert!(!j.state().unwrap().attempts()[0].possibly_exposed);
        assert!(
            c.retained_wire(&mut j, support::attempt(10), 100)
                .unwrap()
                .is_none()
        );
    }
}
#[test]
fn demo_lost_submission_reply_replays_original_without_reauthorizing_or_resending() {
    let temp = support::Temp::new();
    let (mut j, c) = new_store(&temp, Location::Vault);
    prepare(&mut j, 10, Rail::Release);
    let g = gateway();
    qualify(&mut j, &c, &g);
    let f = fake(10);
    {
        let mut rpc = f.0.lock().unwrap();
        rpc.at = 2100;
        rpc.lose_ack = true;
    }
    let clock = Arc::new(Advancing(AtomicU64::new(2100)));
    let mut p = loaded(&c, vec![])
        .with_transport(f.clone(), clock.clone())
        .unwrap();
    assert!(
        p.issue_demo_ingress(&mut j, &c, &g, &policy(), support::attempt(10))
            .unwrap()
            == Outcome::Unknown
    );
    f.0.lock().unwrap().contract = Some(
        c.original_chain_contract(&mut j, support::attempt(10), 2100)
            .unwrap()
            .as_bytes()
            .to_vec(),
    );
    let before = j.state().unwrap().clone();
    drop(j);
    drop(p);
    let mut j = Journal::open(
        SqliteBackend::open(&temp.db, Migration::None).unwrap(),
        support::FixtureProtection,
        config(),
    )
    .unwrap();
    assert_eq!(j.state().unwrap(), &before);
    let mut p = loaded(&c, vec![]).with_transport(f.clone(), clock).unwrap();
    assert!(p.reconcile(&mut j, &c, support::attempt(10)).unwrap() == Outcome::Settled);
    assert!(
        p.issue_demo_ingress(&mut j, &c, &g, &policy(), support::attempt(10))
            .is_err()
    );
    assert_eq!(
        f.0.lock()
            .unwrap()
            .requests
            .iter()
            .filter(|r| r["method"] == "sendTransaction")
            .count(),
        1
    );
}
#[test]
fn demo_changed_policy_expiry_freeze_or_authority_refuses_before_rpc() {
    for case in 0..5 {
        let temp = support::Temp::new();
        let (mut j, c) = new_store(&temp, Location::Vault);
        prepare(&mut j, 10, Rail::Release);
        let g = gateway();
        qualify(&mut j, &c, &g);
        let mut policy = policy();
        let mut at = 2100;
        match case {
            0 => policy.revision += 1,
            1 => policy.initial_setup = None,
            2 => at = 9100,
            3 | 4 => {
                let mut t = tx(
                    &j,
                    90,
                    vec![],
                    vec![if case == 3 {
                        Control::Funds(lifecycle::Action::Freeze)
                    } else {
                        Control::Order(orders::Action::AdvanceAuthority {
                            account: support::user(1),
                            epoch: 2,
                        })
                    }],
                );
                t.at = at;
                assert_eq!(j.commit(t).unwrap().receipt.controls, None);
            }
            _ => unreachable!(),
        }
        let f = fake(10);
        f.0.lock().unwrap().at = at;
        let mut p = loaded(&c, vec![])
            .with_transport(f.clone(), Arc::new(Advancing(AtomicU64::new(at))))
            .unwrap();
        let head = j.head();
        assert!(
            p.issue_demo_ingress(&mut j, &c, &g, &policy, support::attempt(10))
                .is_err(),
            "case {case}"
        );
        assert_eq!(j.head(), head);
        assert!(f.0.lock().unwrap().requests.is_empty());
        assert!(!j.state().unwrap().attempts()[0].possibly_exposed);
    }
}
#[test]
fn demo_simulation_or_fee_failure_retains_unsent_plan_without_signed_wire_or_submission() {
    for fail_sim in [true, false] {
        let temp = support::Temp::new();
        let (mut j, c) = new_store(&temp, Location::Vault);
        prepare(&mut j, 10, Rail::Release);
        let g = gateway();
        qualify(&mut j, &c, &g);
        let f = fake(10);
        {
            let mut r = f.0.lock().unwrap();
            r.at = 2100;
            r.fail_sim = fail_sim;
            if !fail_sim {
                r.fee = 6001;
            }
        }
        let mut p = loaded(&c, vec![])
            .with_transport(f.clone(), Arc::new(Advancing(AtomicU64::new(2100))))
            .unwrap();
        assert!(
            p.issue_demo_ingress(&mut j, &c, &g, &policy(), support::attempt(10))
                .is_err()
        );
        assert!(j.state().unwrap().attempts()[0].possibly_exposed);
        assert!(
            c.retained_wire(&mut j, support::attempt(10), 2100)
                .unwrap()
                .is_none()
        );
        assert!(f.0.lock().unwrap().wire.is_none());
        assert!(
            f.0.lock()
                .unwrap()
                .requests
                .iter()
                .all(|r| r["method"] != "sendTransaction")
        );
        assert!(
            p.issue_demo_ingress(&mut j, &c, &g, &policy(), support::attempt(10))
                .is_err()
        );
        // Verified original no-wire closure is the existing reconciliation path,
        // not a retry or a timeout-based release of an uncertain signed send.
        assert!(p.reconcile(&mut j, &c, support::attempt(10)).unwrap() == Outcome::Pending);
        let mut expired = loaded(&c, vec![])
            .with_transport(f.clone(), Arc::new(Advancing(AtomicU64::new(10000))))
            .unwrap();
        assert!(expired.reconcile(&mut j, &c, support::attempt(10)).unwrap() == Outcome::Settled);
    }
}
