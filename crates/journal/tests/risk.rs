//! Synthetic joined admission, not calibrated leverage/capital or live venue evidence.
mod support;
use cinder_journal::{collateral, funds, model::*, orders, risk::*, wire, *};
use cinder_kernel::{
    identity::*,
    ledger::{
        economics::EconomicChange,
        evidence::{Disposition, EvidencePolicy, MarkObservation, NativeCheck},
        *,
    },
    position::Position,
};
use support::*;

fn commit(s: &mut Store, events: Vec<Event>, controls: Vec<Control>) -> Committed {
    let tx = transaction(
        s.head(),
        u8::try_from(s.head().sequence + 1).unwrap(),
        events,
        controls,
    );
    assert_eq!(
        wire::decode_transaction(&wire::encode_transaction(&tx).unwrap()).unwrap(),
        tx
    );
    s.commit(tx).unwrap()
}
fn fresh(s: &Store, price: u64) -> Control {
    Control::Collateral(collateral::Cut {
        expected_version: s.state().unwrap().ledger().version(),
        policy: collateral::Policy {
            revision: PolicyVersion::new(1).unwrap(),
            markets: vec![collateral::MarginRule {
                market: q(0).unit(),
                private_bps: 1000,
                native_bps: 1000,
            }],
            evidence: EvidencePolicy {
                max_issue_age: 1000,
                max_mark_age: 100,
                max_check_age: 1000,
            },
        },
        marks: vec![MarkObservation {
            price: p(price),
            evidence: key(999),
            observed_at: 10,
            valid_until: 100,
            qualified: true,
        }],
    })
}
fn liquidity() -> Vec<Liquidity> {
    [Location::Vault, Location::Venue]
        .into_iter()
        .map(|location| Liquidity {
            location,
            accessible_bps: 10000,
            due: cash(0),
        })
        .collect()
}
fn step(price: u64) -> Step {
    Step {
        after_ms: 1,
        marks: vec![p(price)],
        events: vec![],
        liquidity: liquidity(),
    }
}
fn policy() -> Policy {
    Policy {
        revision: PolicyVersion::new(1).unwrap(),
        markets: vec![MarketRule {
            market: q(0).unit(),
            maximum_leverage: 100000,
            maintenance_bps: 500,
            native_maintenance_bps: 500,
            gross_limit: cash(1_000_000),
            net_limit: cash(1_000_000),
        }],
        buffer: cash(0),
        horizon_ms: 100,
        valid_until: 100,
        paths: vec![Path {
            id: [1; 32],
            steps: vec![step(100)],
        }],
    }
}
fn install(s: &mut Store, p: Policy) -> Committed {
    let expected_version = s.state().unwrap().ledger().version();
    commit(
        s,
        vec![],
        vec![Control::Risk(Action::Install {
            expected_version,
            policy: Box::new(p),
        })],
    )
}
fn bound(id: u8) -> AttemptKey {
    let mut a = attempt(100 + id);
    a.request.account = user(id);
    a
}
fn setup(t: &Temp, a: i128, b: i128, house: i128, qa: i64, qb: i64) -> Store {
    let mut s = t.create();
    let mut events = vec![
        receipt(1, Owner::Customer(user(1)), a),
        receipt(2, Owner::Customer(user(2)), b),
        receipt(3, Owner::House, house),
    ];
    for (id, lots) in [(1, qa), (2, qb)] {
        if lots != 0 {
            events.push(Event {
                key: RecordKey::Attempt(bound(id)),
                policy: config().policy,
                change: Change::BindExecution {
                    market: q(0).unit(),
                    side: if lots > 0 { Side::Buy } else { Side::Sell },
                },
            });
            events.push(event(
                10 + u64::from(id),
                Change::Fill {
                    target: FillTarget::Customer(bound(id)),
                    quantity: q(lots),
                    price: p(100),
                },
            ));
        }
    }
    assert_eq!(
        commit(
            &mut s,
            events,
            vec![
                Control::Order(orders::Action::AdvanceAuthority {
                    account: user(1),
                    epoch: 1
                }),
                Control::Order(orders::Action::AdvanceAuthority {
                    account: user(2),
                    epoch: 1
                })
            ]
        )
        .receipt
        .controls,
        None
    );
    let l = s.state().unwrap().ledger();
    let e = event(
        20,
        Change::Reconcile(NativeCheck {
            expected_version: l.version(),
            cash: Some(l.venue().cash()),
            funding: Some(l.venue().funding()),
            positions: Some(l.venue().positions().to_vec()),
            complete: true,
            resolves: vec![],
        }),
    );
    commit(&mut s, vec![e], vec![]);
    let c = fresh(&s, 100);
    assert_eq!(commit(&mut s, vec![], vec![c]).receipt.controls, None);
    assert_eq!(install(&mut s, policy()).receipt.controls, None);
    s
}
fn report(s: &Store) -> Report {
    s.state().unwrap().risk_report().unwrap()
}
fn selection(s: &Store, n: u8, lev: u64) -> Control {
    let x = Selection {
        request: request(n),
        market: q(0).unit(),
        leverage: lev,
        policy: policy().revision,
        authority_epoch: 1,
        expires_at: 100,
    };
    let _ = s;
    Control::Risk(Action::Select {
        approval: orders::Approval {
            account: user(1),
            intent_hash: x.digest().unwrap(),
            authority_epoch: 1,
        },
        selection: Box::new(x),
    })
}
fn intent(n: u8, lots: i64) -> orders::Intent {
    orders::Intent {
        request: request(n),
        quantity: q(lots),
        minimum: p(100),
        maximum: p(100),
        maximum_fee_per_lot: cash(0),
        reduce_only: false,
        policy: config().policy,
        authority_epoch: 1,
        expires_at: 100,
        time_in_force: orders::TimeInForce::GoodTilCancelled,
    }
}
fn accept(i: orders::Intent) -> Control {
    Control::Order(orders::Action::Accept {
        approval: orders::Approval {
            account: i.request.account,
            intent_hash: i.digest().unwrap(),
            authority_epoch: 1,
        },
        reservations: vec![
            Reservation {
                resource: Resource::Customer(i.request.account),
                amount: cash(1),
            },
            Reservation {
                resource: Resource::Location(Location::Venue),
                amount: cash(1),
            },
        ],
        intent: Box::new(i),
    })
}

#[test]
fn opposing_pending_order_is_classified_before_its_reservation_is_checked() {
    let t = Temp::new();
    let mut s = setup(&t, 100, 100, 100, 0, 0);
    assert_eq!(
        commit(&mut s, vec![], vec![accept(intent(60, 9))])
            .receipt
            .controls,
        None
    );
    let mut opposing = accept(intent(61, -9));
    if let Control::Order(orders::Action::Accept { reservations, .. }) = &mut opposing {
        reservations[0].amount = cash(20);
    }
    assert_eq!(
        commit(&mut s, vec![], vec![opposing]).receipt.controls,
        None
    );
    let r = report(&s);
    assert_eq!(r.current.customers[0].outcome_requirement, cash(90));
    assert_eq!(r.current.customers[0].other_held, cash(0));
    assert_eq!(r.current.customers[0].free, cash(10));
    let before = s.state().unwrap().clone();
    assert_eq!(
        commit(&mut s, vec![], vec![accept(intent(62, 2))])
            .receipt
            .controls,
        Some(ControlError::Capacity)
    );
    assert_eq!(s.state().unwrap().holds(), before.holds());
    assert_eq!(report(&s), r);
    drop(s);
    assert_eq!(report(&t.open()), r);
}

#[test]
fn retired_history_does_not_multiply_risk_work_but_live_work_is_bounded() {
    let t = Temp::new();
    let mut s = setup(&t, 100_000, 100_000, 100_000, 0, 0);
    let mut controls = vec![];
    for n in 1_u64..=350 {
        let mut i = intent(60, 1);
        let mut id = [0; 32];
        id[..8].copy_from_slice(&n.to_be_bytes());
        i.request.request = RequestId::new(id).unwrap();
        controls.push(accept(i.clone()));
        controls.push(Control::Release(i.request));
    }
    assert_eq!(commit(&mut s, vec![], controls).receipt.controls, None);
    assert_eq!(s.state().unwrap().holds().len(), 350);
    assert!(s.state().unwrap().holds().iter().all(|h| !h.active));
    assert!(report(&s).admissible);
    let mut expensive = policy();
    expensive.revision = PolicyVersion::new(2).unwrap();
    expensive.paths = (1..=32)
        .map(|n| Path {
            id: [n; 32],
            steps: vec![step(100); 64],
        })
        .collect();
    assert_eq!(install(&mut s, expensive).receipt.controls, None);
    // Even many scenario prefixes remain cheap when all orders are retired.
    assert!(report(&s).admissible);
    drop(s);
    let mut s = t.open();
    assert!(report(&s).admissible);
    let retained = s.state().unwrap().holds()[0].request;
    let mut duplicate = intent(60, 1);
    duplicate.request = retained;
    assert_eq!(
        commit(&mut s, vec![], vec![accept(duplicate)])
            .receipt
            .controls,
        Some(ControlError::Invalid)
    );
    let mut blocked = false;
    for n in 60..80 {
        let result = commit(&mut s, vec![], vec![accept(intent(n, 1))]);
        if result.receipt.controls == Some(ControlError::Unqualified) {
            blocked = true;
            break;
        }
        assert_eq!(result.receipt.controls, None);
    }
    assert!(
        blocked,
        "active orders and scenario scans must still obey the work budget"
    );
    assert!(
        report(&s).admissible,
        "the over-budget candidate is rolled back"
    );
}

#[test]
fn private_selection_changes_initial_margin_not_position_equity_or_native_setting() {
    let t = Temp::new();
    let mut s = setup(&t, 100, 100, 100, 2, 0);
    let before = s.state().unwrap().ledger().clone();
    let choose = selection(&s, 50, 25000);
    assert_eq!(commit(&mut s, vec![], vec![choose]).receipt.controls, None);
    let r = report(&s);
    assert_eq!(r.current.customers[0].initial, cash(80));
    assert_eq!(r.current.customers[0].maintenance, cash(10));
    assert_eq!(s.state().unwrap().ledger(), &before);
    for lev in [9999, 100001] {
        let c = selection(&s, 51, lev);
        assert_eq!(
            commit(&mut s, vec![], vec![c]).receipt.controls,
            Some(ControlError::Invalid)
        );
    }
    assert_eq!(report(&t.open()), r);
}
#[test]
fn leverage_selection_cannot_reuse_payout_collateral_and_bad_auth_is_atomic() {
    let t = Temp::new();
    let mut s = setup(&t, 100, 100, 100, 2, 0);
    assert_eq!(
        commit(&mut s, vec![], vec![reserve(30, 21)])
            .receipt
            .controls,
        None
    );
    let c = selection(&s, 31, 25000);
    assert_eq!(
        commit(&mut s, vec![], vec![c]).receipt.controls,
        Some(ControlError::Capacity)
    );
    assert_eq!(report(&s).current.customers[0].initial, cash(20));
    let mut c = selection(&s, 32, 50000);
    if let Control::Risk(Action::Select { approval, .. }) = &mut c {
        approval.intent_hash = [0; 32]
    }
    assert_eq!(
        commit(&mut s, vec![], vec![c]).receipt.controls,
        Some(ControlError::Invalid)
    );
}
#[test]
fn cap_downgrade_records_initial_breach_without_liquidating_maintenance_healthy_customer() {
    let t = Temp::new();
    let mut s = setup(&t, 100, 100, 100, 2, 0);
    let mut p = policy();
    p.revision = PolicyVersion::new(2).unwrap();
    p.markets[0].maximum_leverage = 10000;
    assert_eq!(install(&mut s, p.clone()).receipt.controls, None);
    let r = report(&s);
    assert!(r.current.flags.private_initial);
    assert!(!r.current.flags.private_maintenance);
    assert!(!r.admissible);
    assert_eq!(
        s.state()
            .unwrap()
            .ledger()
            .book(Owner::Customer(user(1)))
            .unwrap()
            .positions()[0]
            .quantity(),
        q(2)
    );
    p.markets[0].maximum_leverage = 20000;
    assert_eq!(
        install(&mut s, p).receipt.controls,
        Some(ControlError::Invalid)
    );
}
#[test]
fn independent_buy_sell_outcomes_are_bounded_not_net_zero_and_small_holds_cannot_bypass() {
    let t = Temp::new();
    let mut s = setup(&t, 100, 100, 100, 0, 0);
    assert_eq!(
        commit(
            &mut s,
            vec![],
            vec![accept(intent(30, 8)), accept(intent(31, -8))]
        )
        .receipt
        .controls,
        None
    );
    let r = report(&s);
    assert_eq!(r.current.customers[0].outcome_requirement, cash(80));
    assert_eq!(r.current.concentration[0].native, cash(800));
    assert!(r.admissible);
    assert_eq!(
        commit(&mut s, vec![], vec![accept(intent(32, 3))])
            .receipt
            .controls,
        Some(ControlError::Capacity)
    );
    assert_eq!(s.state().unwrap().orders().len(), 2);
    let pending = report(&s);
    for buy in 0..=8 {
        for sell in 0..=8 {
            let actual = (buy - sell) as i64;
            assert!(
                cash(i128::from(actual.abs()) * 10).atoms()
                    <= pending.current.customers[0].outcome_requirement.atoms()
            );
        }
    }
}
#[test]
fn all_partial_outcomes_and_price_edges_fit_conservative_integer_envelope() {
    let t = Temp::new();
    let mut s = setup(&t, 1000, 1000, 1000, 0, 0);
    let mut a = intent(30, 5);
    a.minimum = p(90);
    a.maximum = p(110);
    a.maximum_fee_per_lot = cash(2);
    let mut b = a.clone();
    b.request = request(31);
    b.quantity = q(-4);
    assert_eq!(
        commit(&mut s, vec![], vec![accept(a), accept(b)])
            .receipt
            .controls,
        None
    );
    let bound = report(&s).current.customers[0].outcome_requirement.atoms();
    let market = config().markets[0];
    let mut cases = 0;
    for buy in 0..=5 {
        for sell in 0..=4 {
            for bp in [90, 100, 110] {
                for sp in [90, 100, 110] {
                    for reverse in [false, true] {
                        let mut pos = Position::flat(q(0).unit());
                        let mut cash_delta = 0;
                        let mut legs = vec![(buy, bp), (-sell, sp)];
                        if reverse {
                            legs.reverse()
                        }
                        for (qty, price) in legs {
                            if qty != 0 {
                                let f = pos.fill(market, q(qty), p(price)).unwrap();
                                pos = f.position;
                                cash_delta += f.realized.atoms() - i128::from(qty.abs()) * 2;
                            }
                        }
                        let equity_delta =
                            cash_delta + pos.unrealized(market, p(100)).unwrap().atoms();
                        let initial = i128::from(pos.quantity().lots().abs()) * 10;
                        assert!(initial - equity_delta <= bound);
                        cases += 1;
                    }
                }
            }
        }
    }
    assert_eq!(cases, 540);
}
#[test]
fn private_reduce_only_can_increase_native_margin_and_breach_net_limit() {
    let t = Temp::new();
    let mut s = setup(&t, 100, 100, 100, 2, -2);
    let mut p = policy();
    p.revision = PolicyVersion::new(2).unwrap();
    p.markets[0].net_limit = cash(100);
    install(&mut s, p);
    let mut i = intent(30, -2);
    i.reduce_only = true;
    assert_eq!(
        commit(&mut s, vec![], vec![accept(i)]).receipt.controls,
        Some(ControlError::Capacity)
    );
    assert_eq!(report(&s).current.concentration[0].native, cash(0));
}
#[test]
fn gross_limit_catches_net_flat_users_and_is_not_private_loss_mutualization() {
    let t = Temp::new();
    let mut s = setup(&t, 100, 100, 100, 2, -2);
    let mut p = policy();
    p.revision = PolicyVersion::new(2).unwrap();
    p.markets[0].gross_limit = cash(300);
    install(&mut s, p);
    let r = report(&s);
    assert_eq!(r.current.concentration[0].gross, cash(400));
    assert_eq!(r.current.concentration[0].native, cash(0));
    assert!(r.current.flags.concentration);
    assert_eq!(r.current.customers[0].initial, cash(20));
    assert_eq!(r.current.customers[1].initial, cash(20));
}
#[test]
fn peak_deficit_before_recovery_sizes_capital_not_just_terminal_price() {
    let t = Temp::new();
    let mut s = setup(&t, 1000, 50, 100, 10, -10);
    let mut p = policy();
    p.revision = PolicyVersion::new(2).unwrap();
    p.paths[0].steps = vec![step(120), step(100)];
    install(&mut s, p);
    let r = report(&s);
    assert_eq!(r.minimum_free, cash(-50));
    assert_eq!(r.target, cash(150));
    assert_eq!(r.paths[0].prefixes.last().unwrap().free_capital, cash(100));
    assert!(r.paths[0].prefixes[1].flags.insolvent);
    assert!(!r.admissible);
    assert_eq!(
        s.state()
            .unwrap()
            .ledger()
            .book(Owner::House)
            .unwrap()
            .cash(),
        cash(100)
    );
}
#[test]
fn repeated_actual_transition_costs_deplete_capital_without_capping_simulated_loss() {
    let t = Temp::new();
    let mut s = setup(&t, 100, 100, 100, 0, 0);
    let mut p = policy();
    p.revision = PolicyVersion::new(2).unwrap();
    let mut first = step(100);
    first.events.push(event(
        1000,
        Change::Economics(EconomicChange::Execution {
            target: FillTarget::House,
            quantity: q(1),
            price: support::p(100),
            fee: cash(60),
            pnl: None,
        }),
    ));
    let mut second = step(100);
    second.after_ms = 2;
    second.events.push(event(
        1001,
        Change::Economics(EconomicChange::Execution {
            target: FillTarget::House,
            quantity: q(-1),
            price: support::p(100),
            fee: cash(60),
            pnl: None,
        }),
    ));
    p.paths[0].steps = vec![first, second];
    install(&mut s, p);
    let r = report(&s);
    assert_eq!(r.minimum_free, cash(-20));
    assert_eq!(r.target, cash(120));
    assert!(!r.admissible);
    assert_eq!(
        s.state()
            .unwrap()
            .ledger()
            .book(Owner::House)
            .unwrap()
            .cash(),
        cash(100)
    );
}
#[test]
fn location_deadline_liquidity_and_capital_are_independent_failures() {
    let t = Temp::new();
    let mut s = setup(&t, 100, 100, 100, 0, 0);
    let mut p = policy();
    p.revision = PolicyVersion::new(2).unwrap();
    p.buffer = cash(101);
    install(&mut s, p.clone());
    let r = report(&s);
    assert!(r.current.flags.capital);
    assert!(!r.current.flags.insolvent);
    assert!(!r.current.flags.illiquid);
    p.revision = PolicyVersion::new(3).unwrap();
    p.buffer = cash(0);
    p.paths[0].steps[0].liquidity[0].due = cash(1);
    install(&mut s, p);
    let r = report(&s);
    assert_eq!(r.minimum_free, cash(100));
    assert!(r.paths[0].prefixes[1].flags.illiquid);
    assert!(!r.admissible);
}
#[test]
fn missing_paths_duplicate_location_future_capital_and_rebates_cannot_qualify() {
    let variants = 0..5;
    for variant in variants {
        let t = Temp::new();
        let mut s = setup(&t, 100, 100, 100, 0, 0);
        let mut p = policy();
        p.revision = PolicyVersion::new(2).unwrap();
        match variant {
            0 => p.paths.clear(),
            1 => p.paths[0].steps[0].liquidity[1].location = Location::Vault,
            2 => p.paths[0].steps[0]
                .events
                .push(receipt(1000, Owner::House, 1000)),
            3 => p.paths[0].steps[0].events.push(event(
                1000,
                Change::Economics(EconomicChange::Execution {
                    target: FillTarget::House,
                    quantity: q(1),
                    price: support::p(100),
                    fee: cash(-1),
                    pnl: None,
                }),
            )),
            _ => p.markets[0].maintenance_bps = 1000,
        }
        assert_eq!(
            install(&mut s, p).receipt.controls,
            Some(ControlError::Invalid)
        );
        assert!(report(&s).admissible);
    }
}
#[test]
fn external_shock_records_even_when_new_exposure_fails_and_stale_marks_never_pass() {
    let t = Temp::new();
    let mut s = setup(&t, 100, 100, 100, 0, 0);
    assert_eq!(
        commit(
            &mut s,
            vec![],
            vec![
                accept(intent(30, 1)),
                Control::Order(orders::Action::Prepare {
                    attempt: attempt(30)
                })
            ]
        )
        .receipt
        .controls,
        None
    );
    let shock = event(
        1000,
        Change::Economics(EconomicChange::Execution {
            target: FillTarget::House,
            quantity: q(1),
            price: p(100),
            fee: cash(150),
            pnl: None,
        }),
    );
    let result = commit(&mut s, vec![shock], vec![Control::Expose(attempt(30))]);
    assert_eq!(
        result.receipt.inputs,
        vec![InputResult::Normalized(Disposition::Applied)]
    );
    assert_eq!(result.receipt.controls, Some(ControlError::Capacity));
    assert_eq!(
        s.state()
            .unwrap()
            .ledger()
            .book(Owner::House)
            .unwrap()
            .cash(),
        cash(-50)
    );
    assert!(report(&s).current.flags.insolvent);
    let mut tx = transaction(s.head(), 99, vec![], vec![Control::Expose(attempt(30))]);
    tx.at = 101;
    assert_eq!(
        s.commit(tx).unwrap().receipt.controls,
        Some(ControlError::Unqualified)
    );
}
#[test]
fn policy_install_requires_exact_financial_cut_and_unknown_stress_events_fail_closed() {
    let t = Temp::new();
    let mut s = setup(&t, 100, 100, 100, 0, 0);
    let mut p = policy();
    p.revision = PolicyVersion::new(2).unwrap();
    assert_eq!(
        commit(
            &mut s,
            vec![],
            vec![Control::Risk(Action::Install {
                expected_version: 0,
                policy: Box::new(p.clone())
            })]
        )
        .receipt
        .controls,
        Some(ControlError::Invalid)
    );
    p.paths[0].steps[0].events.push(event(
        1000,
        Change::Fill {
            target: FillTarget::Customer(attempt(99)),
            quantity: q(1),
            price: support::p(100),
        },
    ));
    assert_eq!(install(&mut s, p).receipt.controls, None);
    assert_eq!(
        s.state().unwrap().risk_report(),
        Err(ControlError::Unqualified)
    );
}

#[test]
fn final_atomic_cut_prevents_exposure_followed_by_policy_downgrade_bypass() {
    let t = Temp::new();
    let mut s = setup(&t, 100, 100, 100, 0, 0);
    let mut tighter = policy();
    tighter.revision = PolicyVersion::new(2).unwrap();
    tighter.markets[0].net_limit = cash(0);
    // Preparing binds an execution and advances the financial version once.
    let expected_version = s.state().unwrap().ledger().version() + 1;
    let out = commit(
        &mut s,
        vec![],
        vec![
            accept(intent(30, 1)),
            Control::Order(orders::Action::Prepare {
                attempt: attempt(30),
            }),
            Control::Expose(attempt(30)),
            Control::Risk(Action::Install {
                expected_version,
                policy: Box::new(tighter),
            }),
        ],
    );
    assert_eq!(out.receipt.controls, Some(ControlError::Capacity));
    assert!(out.exposures.is_empty());
    assert!(s.state().unwrap().orders().is_empty());
    assert!(report(&s).admissible);
}

#[test]
fn payout_and_pending_order_share_margin_without_spending_it_twice() {
    use cinder_kernel::ledger::funds::Destination;
    let t = Temp::new();
    let mut s = setup(&t, 100, 100, 100, 0, 0);
    assert_eq!(
        commit(
            &mut s,
            vec![],
            vec![
                accept(intent(30, 5)),
                Control::Order(orders::Action::Prepare {
                    attempt: attempt(30)
                }),
                Control::Expose(attempt(30))
            ]
        )
        .receipt
        .controls,
        None
    );
    for (amount, expected) in [(51, Some(ControlError::Capacity)), (50, None)] {
        let i = funds::Intent {
            request: request(40),
            source: Location::Venue,
            destination: Destination::Recipient([42; 32]),
            net: cash(amount),
            maximum_fee: cash(1),
            fee_payer: Owner::House,
            allow_partial: true,
            policy: config().policy,
            authority_epoch: 1,
            expires_at: 100,
        };
        let approval = orders::Approval {
            account: user(1),
            intent_hash: i.digest().unwrap(),
            authority_epoch: 1,
        };
        assert_eq!(
            commit(
                &mut s,
                vec![],
                vec![Control::Funds(funds::Action::Accept {
                    intent: Box::new(i),
                    approval
                })]
            )
            .receipt
            .controls,
            expected
        );
    }
    assert_eq!(report(&s).current.customers[0].free, cash(0));
    let fill = event(
        1000,
        Change::Fill {
            target: FillTarget::Customer(attempt(30)),
            quantity: q(1),
            price: p(100),
        },
    );
    assert_eq!(
        commit(&mut s, vec![fill], vec![]).receipt.inputs,
        vec![InputResult::Normalized(Disposition::Applied)]
    );
    assert_eq!(report(&s).current.customers[0].free, cash(0));
    assert_eq!(report(&t.open()), report(&s));
}

#[test]
fn shared_location_demands_are_summed_before_scenario_availability() {
    let t = Temp::new();
    let mut s = setup(&t, 100, 100, 100, 0, 0);
    let mut p = policy();
    p.revision = PolicyVersion::new(2).unwrap();
    p.paths[0].steps[0].liquidity[1].accessible_bps = 5000;
    p.paths[0].steps[0].liquidity[1].due = cash(50);
    install(&mut s, p);
    // 150 accessible, 50 due: two 60-unit source promises cannot both pass.
    assert_eq!(
        commit(
            &mut s,
            vec![],
            vec![Control::Reserve {
                request: request(30),
                reservations: vec![Reservation {
                    resource: Resource::Location(Location::Venue),
                    amount: cash(60)
                }]
            }]
        )
        .receipt
        .controls,
        None
    );
    assert_eq!(
        commit(
            &mut s,
            vec![],
            vec![Control::Reserve {
                request: request(31),
                reservations: vec![Reservation {
                    resource: Resource::Location(Location::Venue),
                    amount: cash(60)
                }]
            }]
        )
        .receipt
        .controls,
        Some(ControlError::Capacity)
    );
    assert_eq!(
        s.state()
            .unwrap()
            .reserved(Resource::Location(Location::Venue))
            .unwrap(),
        cash(60)
    );
}

#[test]
fn hypothetical_execution_conflict_and_expired_policy_never_become_safe_empty_paths() {
    use cinder_kernel::ledger::economics::NativePnl;
    let t = Temp::new();
    let mut s = setup(&t, 100, 100, 100, 0, 0);
    let mut p = policy();
    p.revision = PolicyVersion::new(2).unwrap();
    p.paths[0].steps[0].events.push(event(
        1000,
        Change::Economics(EconomicChange::Execution {
            target: FillTarget::House,
            quantity: q(1),
            price: support::p(100),
            fee: cash(0),
            pnl: Some(NativePnl::Gross(cash(99))),
        }),
    ));
    install(&mut s, p);
    assert_eq!(
        s.state().unwrap().risk_report(),
        Err(ControlError::Unqualified)
    );
    let mut p = policy();
    p.revision = PolicyVersion::new(3).unwrap();
    p.valid_until = 10;
    install(&mut s, p);
    assert!(report(&s).admissible);
    let mut tx = transaction(s.head(), 99, vec![], vec![]);
    tx.at = 11;
    s.commit(tx).unwrap();
    assert_eq!(
        s.state().unwrap().risk_report(),
        Err(ControlError::Unqualified)
    );
}

#[test]
fn grouped_scenario_cannot_hide_a_capital_breach_before_later_recovery() {
    let t = Temp::new();
    let mut s = setup(&t, 100, 100, 100, 0, 0);
    let mut p = policy();
    p.revision = PolicyVersion::new(2).unwrap();
    p.buffer = cash(40);
    p.paths[0].steps[0].events = vec![
        event(
            1000,
            Change::Economics(EconomicChange::Execution {
                target: FillTarget::House,
                quantity: q(1),
                price: support::p(100),
                fee: cash(60),
                pnl: None,
            }),
        ),
        event(
            1001,
            Change::Fill {
                target: FillTarget::House,
                quantity: q(-1),
                price: support::p(200),
            },
        ),
    ];
    install(&mut s, p);
    let r = report(&s);
    assert_eq!(r.paths[0].prefixes.len(), 4);
    assert_eq!(r.minimum_free, cash(30));
    assert_eq!(r.target, cash(110));
    assert_eq!(r.paths[0].prefixes.last().unwrap().free_capital, cash(140));
    assert!(!r.admissible);
}
