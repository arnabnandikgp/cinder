//! Bounded-close ownership over one actual native fill; no venue calibration.
mod support;
use cinder_journal::{collateral, liquidation as lc, model::*, orders, risk, wire};
use cinder_kernel::{
    identity::*,
    ledger::{close::*, protection::ProtectionChange, *},
    position::Market,
};
use support::*;

fn run(s: &mut Store, events: Vec<Event>, controls: Vec<Control>) -> cinder_journal::Committed {
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
fn live(t: &Temp, lots: i64, mark: u64) -> Store {
    use cinder_kernel::ledger::evidence::*;
    let mut s = t.create();
    let mut es = vec![
        receipt(1, Owner::Customer(user(1)), 100),
        receipt(2, Owner::Customer(user(2)), 100),
        receipt(3, Owner::House, 10000),
    ];
    if lots != 0 {
        es.extend([
            Event {
                key: RecordKey::Attempt(attempt(1)),
                policy: config().policy,
                change: Change::BindExecution {
                    market: q(0).unit(),
                    side: if lots > 0 { Side::Buy } else { Side::Sell },
                },
            },
            event(
                4,
                Change::Fill {
                    target: FillTarget::Customer(attempt(1)),
                    quantity: q(lots),
                    price: p(100),
                },
            ),
        ])
    }
    run(
        &mut s,
        es,
        vec![Control::Order(orders::Action::AdvanceAuthority {
            account: user(1),
            epoch: 1,
        })],
    );
    let l = s.state().unwrap().ledger();
    let e = event(
        900,
        Change::Reconcile(NativeCheck {
            expected_version: l.version(),
            cash: Some(l.venue().cash()),
            funding: Some(l.venue().funding()),
            positions: Some(l.venue().positions().to_vec()),
            complete: true,
            resolves: vec![],
        }),
    );
    run(&mut s, vec![e], vec![]);
    let c = collateral::Cut {
        expected_version: s.state().unwrap().ledger().version(),
        policy: collateral::Policy {
            revision: config().policy,
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
            price: p(mark),
            evidence: key(999),
            observed_at: 10,
            valid_until: 100,
            qualified: true,
        }],
    };
    assert_eq!(
        run(&mut s, vec![], vec![Control::Collateral(c)])
            .receipt
            .controls,
        None
    );
    let policy = risk::Policy {
        revision: config().policy,
        markets: vec![risk::MarketRule {
            market: q(0).unit(),
            maximum_leverage: 100000,
            maintenance_bps: 500,
            native_maintenance_bps: 500,
            gross_limit: cash(100000),
            net_limit: cash(100000),
        }],
        buffer: cash(0),
        horizon_ms: 100,
        valid_until: 100,
        paths: vec![risk::Path {
            id: [1; 32],
            steps: vec![risk::Step {
                after_ms: 1,
                marks: vec![p(mark)],
                events: vec![],
                liquidity: [Location::Vault, Location::Venue]
                    .into_iter()
                    .map(|location| risk::Liquidity {
                        location,
                        accessible_bps: 10000,
                        due: cash(0),
                    })
                    .collect(),
            }],
        }],
    };
    let c = Control::Risk(risk::Action::Install {
        expected_version: s.state().unwrap().ledger().version(),
        policy: Box::new(policy),
    });
    assert_eq!(run(&mut s, vec![], vec![c]).receipt.controls, None);
    install(&mut s, limits());
    s
}
fn limits() -> lc::Policy {
    lc::Policy {
        revision: config().policy,
        authority_epoch: 1,
        valid_until: 100,
        limits: vec![lc::Limit {
            market: q(0).unit(),
            maximum_lots: 2,
            minimum: p(70),
            maximum: p(130),
            fee_per_lot: cash(1),
            additional_per_lot: cash(5),
        }],
    }
}
fn install(s: &mut Store, p: lc::Policy) {
    let c = Control::Liquidation(lc::Action::Install {
        expected_version: s.state().unwrap().ledger().version(),
        policy: Box::new(p),
    });
    assert_eq!(run(s, vec![], vec![c]).receipt.controls, None)
}
fn proposal(s: &Store, n: u8, kind: lc::Kind) -> Control {
    let p = lc::Proposal {
        attempt: attempt(n),
        kind,
        market: q(0).unit(),
        expected_version: s.state().unwrap().ledger().version(),
        policy: config().policy,
        authority_epoch: 1,
        expires_at: 100,
    };
    Control::Liquidation(lc::Action::Prepare {
        authenticated_digest: p.digest().unwrap(),
        proposal: Box::new(p),
    })
}
fn actual(n: u64, a: u8, lots: i64, price: u64, fee: i128) -> Event {
    event(
        n,
        Change::Economics(economics::EconomicChange::Execution {
            target: FillTarget::Customer(attempt(a)),
            quantity: q(lots),
            price: p(price),
            fee: cash(fee),
            pnl: None,
        }),
    )
}
fn terminal(s: &mut Store, n: u8, filled: i64, ids: &[u64]) {
    let mut tx = transaction(
        s.head(),
        u8::try_from(s.head().sequence + 1).unwrap(),
        vec![],
        vec![Control::Order(orders::Action::Release {
            request: request(n),
        })],
    );
    tx.order_observations.push(orders::Observation {
        key: key(800 + u64::from(n)),
        attempt: attempt(n),
        status: orders::Status::Terminal(orders::Terminal {
            filled: q(filled),
            executions: ids.iter().map(|n| key(*n)).collect(),
            through: 0,
        }),
        authority_epoch: 1,
        observed_at: 10,
        raw: raw(b"qualified complete close history"),
    });
    assert_eq!(s.commit(tx).unwrap().receipt.controls, None);
}
fn private(s: &mut Store, n: u8) {
    let intent = orders::Intent {
        request: request(n),
        time_in_force: orders::TimeInForce::ImmediateOrCancel,
        quantity: q(-2),
        minimum: p(99),
        maximum: p(101),
        maximum_fee_per_lot: cash(1),
        reduce_only: true,
        policy: config().policy,
        authority_epoch: 1,
        expires_at: 100,
    };
    let c = Control::Order(orders::Action::Accept {
        approval: orders::Approval {
            account: user(1),
            authority_epoch: 1,
            intent_hash: intent.digest().unwrap(),
        },
        intent: Box::new(intent),
        reservations: vec![
            Reservation {
                resource: Resource::Customer(user(1)),
                amount: cash(1),
            },
            Reservation {
                resource: Resource::Location(Location::Venue),
                amount: cash(1),
            },
        ],
    });
    assert_eq!(
        run(
            s,
            vec![],
            vec![
                c,
                Control::Order(orders::Action::Prepare {
                    attempt: attempt(n)
                }),
                Control::Expose(attempt(n))
            ]
        )
        .receipt
        .controls,
        None
    );
}

#[test]
fn liquidation_requires_maintenance_breach_caps_depth_and_rechecks_before_exposure() {
    let t = Temp::new();
    let mut s = live(&t, 5, 100);
    let c = proposal(&s, 20, lc::Kind::Liquidation);
    assert_eq!(
        run(&mut s, vec![], vec![c]).receipt.controls,
        Some(ControlError::Invalid)
    );
    let t = Temp::new();
    let mut s = live(&t, 5, 81);
    let c = proposal(&s, 20, lc::Kind::Liquidation);
    assert_eq!(run(&mut s, vec![], vec![c]).receipt.controls, None);
    assert_eq!(s.state().unwrap().orders()[0].intent.quantity, q(-2));
    // Actual top-up repairs private maintenance before dispatch; do not liquidate
    // a now-healthy customer from a stale prepared instruction.
    run(
        &mut s,
        vec![receipt(1000, Owner::Customer(user(1)), 100)],
        vec![],
    );
    assert_eq!(
        run(&mut s, vec![], vec![Control::Expose(attempt(20))])
            .receipt
            .controls,
        Some(ControlError::Invalid)
    );
}
#[test]
fn liquidation_partial_ack_timeout_and_restart_retain_residual_and_holds() {
    let t = Temp::new();
    let mut s = live(&t, 5, 81);
    let c = proposal(&s, 20, lc::Kind::Liquidation);
    let r = run(&mut s, vec![], vec![c, Control::Expose(attempt(20))]);
    assert_eq!(r.receipt.controls, None);
    assert_eq!(r.exposures.len(), 1);
    run(&mut s, vec![actual(100, 20, -1, 80, 1)], vec![]);
    assert_eq!(
        qty(s.state().unwrap().ledger(), Owner::Customer(user(1))),
        4
    );
    let expected = s.state().unwrap().clone();
    drop(s);
    let mut s = t.open();
    assert_eq!(s.state().unwrap(), &expected);
    let mut tx = transaction(
        s.head(),
        99,
        vec![],
        vec![Control::Order(orders::Action::Release {
            request: request(20),
        })],
    );
    tx.at = 101;
    tx.order_observations.push(orders::Observation {
        key: key(500),
        attempt: attempt(20),
        status: orders::Status::Unknown,
        authority_epoch: 1,
        observed_at: 101,
        raw: raw(b"timeout is not completed execution"),
    });
    assert_eq!(
        s.commit(tx).unwrap().receipt.controls,
        Some(ControlError::Unqualified)
    );
    assert!(s.state().unwrap().holds()[0].active);
}
#[test]
fn protected_private_close_books_late_excess_and_only_funded_house_unwind_is_allowed() {
    let t = Temp::new();
    let mut s = live(&t, 2, 100);
    private(&mut s, 20);
    run(
        &mut s,
        vec![
            Event {
                key: RecordKey::Attempt(attempt(9)),
                policy: config().policy,
                change: Change::BindExecution {
                    market: q(0).unit(),
                    side: Side::Sell,
                },
            },
            actual(99, 9, -1, 100, 0),
        ],
        vec![],
    );
    run(&mut s, vec![actual(100, 20, -2, 99, 2)], vec![]);
    assert_eq!(
        qty(s.state().unwrap().ledger(), Owner::Customer(user(1))),
        0
    );
    assert_eq!(qty(s.state().unwrap().ledger(), Owner::House), -1);
    assert!(s.state().unwrap().closes()[0].contained);
    assert_eq!(
        run(&mut s, vec![], vec![reserve(40, 1)]).receipt.controls,
        Some(ControlError::Unqualified)
    );
    terminal(&mut s, 20, -2, &[100]);
    let c = proposal(&s, 30, lc::Kind::HouseUnwind);
    let r = run(&mut s, vec![], vec![c, Control::Expose(attempt(30))]);
    assert_eq!(r.receipt.controls, None);
    assert_eq!(r.exposures.len(), 1);
    run(&mut s, vec![actual(101, 30, 1, 120, 1)], vec![]);
    assert_eq!(qty(s.state().unwrap().ledger(), Owner::House), 0);
    assert_eq!(
        qty(s.state().unwrap().ledger(), Owner::Customer(user(1))),
        0
    );
    assert_eq!(
        s.state()
            .unwrap()
            .ledger()
            .book(Owner::House)
            .unwrap()
            .cash(),
        cash(9977)
    );
    terminal(&mut s, 30, 1, &[101]);
    let c = proposal(&s, 31, lc::Kind::HouseUnwind);
    assert_eq!(
        run(&mut s, vec![], vec![c]).receipt.controls,
        Some(ControlError::Invalid)
    );
}
#[test]
fn ordinary_risk_cannot_piggyback_on_emergency_admission() {
    let t = Temp::new();
    let mut s = live(&t, 5, 81);
    let c = proposal(&s, 20, lc::Kind::Liquidation);
    assert!(
        run(
            &mut s,
            vec![],
            vec![c, Control::Expose(attempt(20)), reserve(30, 1)]
        )
        .receipt
        .controls
        .is_some()
    );
    assert!(s.state().unwrap().attempts().is_empty());
}

#[test]
fn prepared_unexposed_cleanup_can_be_abandoned_but_unknown_exposure_cannot() {
    let t = Temp::new();
    let mut s = live(&t, 5, 81);
    let c = proposal(&s, 20, lc::Kind::Liquidation);
    assert_eq!(run(&mut s, vec![], vec![c]).receipt.controls, None);
    assert_eq!(
        run(&mut s, vec![], vec![Control::Release(request(20))])
            .receipt
            .controls,
        None
    );
    assert!(s.state().unwrap().orders()[0].abandoned);
    assert!(!s.state().unwrap().holds()[0].active);
    assert!(
        run(&mut s, vec![], vec![Control::Expose(attempt(20))])
            .receipt
            .controls
            .is_some()
    );
    let c = proposal(&s, 21, lc::Kind::Liquidation);
    assert_eq!(
        run(&mut s, vec![], vec![c, Control::Expose(attempt(21))])
            .receipt
            .controls,
        None
    );
    assert_eq!(
        run(&mut s, vec![], vec![Control::Release(request(21))])
            .receipt
            .controls,
        Some(ControlError::Exposed)
    );
    assert!(s.state().unwrap().holds()[1].active);
}

#[test]
fn final_policy_and_authority_cut_cannot_be_changed_after_exposure_in_same_commit() {
    let t = Temp::new();
    let mut s = live(&t, 5, 81);
    let c = proposal(&s, 20, lc::Kind::Liquidation);
    assert_eq!(run(&mut s, vec![], vec![c]).receipt.controls, None);
    let mut p = limits();
    p.revision = cinder_kernel::identity::PolicyVersion::new(2).unwrap();
    p.authority_epoch = 2;
    let install = Control::Liquidation(lc::Action::Install {
        expected_version: s.state().unwrap().ledger().version(),
        policy: Box::new(p),
    });
    let result = run(&mut s, vec![], vec![Control::Expose(attempt(20)), install]);
    assert!(result.receipt.controls.is_some());
    assert!(result.exposures.is_empty());
    assert!(!s.state().unwrap().attempts()[0].possibly_exposed);
}

#[test]
fn house_budget_is_shared_not_a_fresh_allowance_per_cleanup_request() {
    let t = Temp::new();
    let mut s = live(&t, 2, 100);
    assert_eq!(
        run(
            &mut s,
            vec![],
            vec![Control::Reserve {
                request: request(40),
                reservations: vec![Reservation {
                    resource: Resource::House,
                    amount: cash(10000)
                }]
            }]
        )
        .receipt
        .controls,
        None
    );
    let intent = orders::Intent {
        request: request(20),
        time_in_force: orders::TimeInForce::ImmediateOrCancel,
        quantity: q(-2),
        minimum: p(99),
        maximum: p(101),
        maximum_fee_per_lot: cash(1),
        reduce_only: true,
        policy: config().policy,
        authority_epoch: 1,
        expires_at: 100,
    };
    let accept = Control::Order(orders::Action::Accept {
        approval: orders::Approval {
            account: user(1),
            authority_epoch: 1,
            intent_hash: intent.digest().unwrap(),
        },
        intent: Box::new(intent),
        reservations: vec![
            Reservation {
                resource: Resource::Customer(user(1)),
                amount: cash(1),
            },
            Reservation {
                resource: Resource::Location(Location::Venue),
                amount: cash(1),
            },
        ],
    });
    assert_eq!(run(&mut s, vec![], vec![accept]).receipt.controls, None);
    assert_eq!(
        run(
            &mut s,
            vec![],
            vec![Control::Order(orders::Action::Prepare {
                attempt: attempt(20)
            })]
        )
        .receipt
        .controls,
        Some(ControlError::Capacity)
    );
    assert!(s.state().unwrap().closes().is_empty());
}

#[test]
fn outside_bounds_actual_fill_survives_refused_controls_and_allows_scoped_cancel() {
    let t = Temp::new();
    let mut s = live(&t, 5, 81);
    let c = proposal(&s, 20, lc::Kind::Liquidation);
    assert_eq!(
        run(&mut s, vec![], vec![c, Control::Expose(attempt(20))])
            .receipt
            .controls,
        None
    );
    let result = run(
        &mut s,
        vec![actual(100, 20, -3, 30, 1000)],
        vec![reserve(40, 1)],
    );
    assert!(result.receipt.controls.is_some());
    let l = s.state().unwrap().ledger();
    assert_eq!(qty(l, Owner::Customer(user(1))), 5);
    assert_eq!(qty(l, Owner::House), -3);
    assert_eq!(l.book(Owner::House).unwrap().cash(), cash(9000));
    assert!(s.state().unwrap().closes()[0].contained);
    let a = AttemptKey {
        request: request(20),
        attempt: AttemptId::new([21; 32]).unwrap(),
    };
    let result = run(
        &mut s,
        vec![],
        vec![
            Control::Order(orders::Action::PrepareCancel {
                attempt: a,
                authority_epoch: 1,
                expires_at: 100,
            }),
            Control::Expose(a),
        ],
    );
    assert_eq!(result.receipt.controls, None);
    assert_eq!(result.exposures.len(), 1);
}

#[test]
fn expired_depth_policy_refuses_dispatch_without_releasing_order_or_hiding_late_fill() {
    let t = Temp::new();
    let mut s = live(&t, 5, 81);
    let c = proposal(&s, 20, lc::Kind::Liquidation);
    assert_eq!(
        run(&mut s, vec![], vec![c, Control::Expose(attempt(20))])
            .receipt
            .controls,
        None
    );
    let mut tx = transaction(
        s.head(),
        99,
        vec![actual(100, 20, -1, 80, 1)],
        vec![reserve(40, 1)],
    );
    tx.at = 101;
    assert!(s.commit(tx).unwrap().receipt.controls.is_some());
    assert_eq!(
        qty(s.state().unwrap().ledger(), Owner::Customer(user(1))),
        4
    );
    assert!(s.state().unwrap().holds()[0].active);
}

#[test]
fn actual_fill_contradicting_local_abandonment_reencumbers_and_stays_house_owned() {
    let t = Temp::new();
    let mut s = live(&t, 5, 81);
    let c = proposal(&s, 20, lc::Kind::Liquidation);
    assert_eq!(run(&mut s, vec![], vec![c]).receipt.controls, None);
    assert_eq!(
        run(&mut s, vec![], vec![Control::Release(request(20))])
            .receipt
            .controls,
        None
    );
    run(&mut s, vec![actual(100, 20, -1, 80, 1)], vec![]);
    assert!(s.state().unwrap().holds()[0].active);
    assert!(s.state().unwrap().orders()[0].faulted);
    assert_eq!(
        qty(s.state().unwrap().ledger(), Owner::Customer(user(1))),
        5
    );
    assert_eq!(qty(s.state().unwrap().ledger(), Owner::House), -1);
}
fn put(l: &Ledger, e: Event) -> Ledger {
    assert_eq!(
        wire::decode_event(&wire::encode_event(&e).unwrap()).unwrap(),
        e
    );
    let s = l.apply(&e).unwrap();
    s.check_bridge().unwrap();
    s
}
fn bind(l: &Ledger, n: u8, house: bool, lots: i64) -> Ledger {
    put(
        l,
        Event {
            key: RecordKey::Attempt(attempt(n)),
            policy: config().policy,
            change: Change::Close(CloseChange::Bind {
                house,
                quantity: q(lots),
            }),
        },
    )
}
fn fill(l: &Ledger, id: u64, n: u8, lots: i64, price: u64, fee: i128) -> Ledger {
    put(
        l,
        event(
            id,
            Change::Close(CloseChange::Execution {
                attempt: attempt(n),
                quantity: q(lots),
                price: p(price),
                fee: cash(fee),
                pnl: None,
                customer_allowed: true,
            }),
        ),
    )
}
fn seeded(cfg: Config, qty: i64, price: u64) -> Ledger {
    let mut l = Ledger::new(cfg).unwrap();
    for e in [
        receipt(1, Owner::Customer(user(1)), 100),
        receipt(2, Owner::House, 100),
        Event {
            key: RecordKey::Request(request(10)),
            policy: config().policy,
            change: Change::Protection(ProtectionChange::Designate { delta: cash(100) }),
        },
        Event {
            key: RecordKey::Attempt(attempt(1)),
            policy: config().policy,
            change: Change::BindExecution {
                market: q(0).unit(),
                side: if qty > 0 { Side::Buy } else { Side::Sell },
            },
        },
        event(
            3,
            Change::Fill {
                target: FillTarget::Customer(attempt(1)),
                quantity: q(qty),
                price: p(price),
            },
        ),
    ] {
        l = put(&l, e)
    }
    l
}
fn qty(l: &Ledger, o: Owner) -> i64 {
    l.book(o).unwrap().positions()[0].quantity().lots()
}

#[test]
fn late_customer_close_cannot_flip_and_unwind_records_uncapped_loss() {
    let mut l = seeded(config(), 2, 100);
    l = bind(&l, 2, false, -2);
    // Another qualified reduction (e.g. the future P12 forced-event allocator)
    // reaches the causal ledger before the old close execution.
    l = bind(&l, 3, false, -1);
    l = fill(&l, 4, 3, -1, 100, 0);
    l = fill(&l, 5, 2, -2, 99, 2);
    assert_eq!(qty(&l, Owner::Customer(user(1))), 0);
    assert_eq!(qty(&l, Owner::House), -1);
    assert_eq!(l.book(Owner::Customer(user(1))).unwrap().cash(), cash(98));
    assert_eq!(l.protection().reserve, cash(99));
    l = bind(&l, 4, true, 1);
    l = fill(&l, 6, 4, 1, 120, 2);
    assert_eq!(qty(&l, Owner::House), 0);
    assert_eq!(l.protection().reserve, cash(76));
    assert_eq!(l.closes()[0].excess, 1);
    assert_eq!(l.closes()[2].spent, cash(23));
}
#[test]
fn partial_execution_retains_position_and_unexecuted_quantity_is_not_flat() {
    let mut l = bind(&seeded(config(), 5, 100), 2, false, -5);
    l = fill(&l, 4, 2, -2, 90, 1);
    assert_eq!(qty(&l, Owner::Customer(user(1))), 3);
    assert_eq!(l.closes()[0].received, 2);
    assert_eq!(l.venue().positions()[0].quantity(), q(3));
}
#[test]
fn split_native_notional_preserves_fractional_child_residue_and_fee_once() {
    let mut cfg = config();
    cfg.markets[0] = Market::new(q(0).unit(), 3, 2).unwrap();
    let mut l = seeded(cfg, 2, 2);
    l = bind(&l, 2, false, -2);
    l = bind(&l, 3, false, -1);
    l = fill(&l, 4, 3, -1, 2, 0);
    l = fill(&l, 5, 2, -2, 1, 3);
    assert_eq!(qty(&l, Owner::Customer(user(1))), 0);
    assert_eq!(qty(&l, Owner::House), -1);
    assert_eq!(l.book(Owner::Customer(user(1))).unwrap().cash(), cash(97));
    assert_eq!(l.book(Owner::House).unwrap().cash(), cash(98));
    assert_eq!(
        l.book(Owner::House).unwrap().positions()[0].basis().atoms(),
        -2
    );
    assert_eq!(l.venue().cash(), cash(195));
    assert_eq!(l.venue().positions()[0].basis().atoms(), -2);
}
#[test]
fn wrong_direction_or_overexecution_is_house_not_customer_reversal() {
    let mut l = bind(&seeded(config(), 2, 100), 2, false, -2);
    l = fill(&l, 4, 2, 1, 110, 1);
    assert_eq!(qty(&l, Owner::Customer(user(1))), 2);
    assert_eq!(qty(&l, Owner::House), 1);
    l = fill(&l, 5, 2, -4, 90, 4);
    // The wrong-side execution consumed an actual-attempt magnitude; only one
    // of its original two closing lots remains automatically attributable.
    assert_eq!(qty(&l, Owner::Customer(user(1))), 1);
    assert_eq!(qty(&l, Owner::House), -2);
    assert_eq!(l.closes()[0].received, 5);
    assert_eq!(l.closes()[0].excess, 4);
}
#[test]
fn exact_duplicate_and_reported_native_pnl_difference_do_not_duplicate_allocation() {
    let mut l = bind(&seeded(config(), 2, 100), 2, false, -2);
    let e = event(
        4,
        Change::Close(CloseChange::Execution {
            attempt: attempt(2),
            quantity: q(-2),
            price: p(90),
            fee: cash(2),
            pnl: Some(economics::NativePnl::NetOfFee(cash(-23))),
            customer_allowed: true,
        }),
    );
    l = put(&l, e.clone());
    assert_eq!(put(&l, e), l);
    assert_eq!(l.book(Owner::Customer(user(1))).unwrap().cash(), cash(78));
    assert_eq!(l.book(Owner::Suspense).unwrap().cash(), cash(-1));
    assert!(l.issues().iter().any(|i| i.open));
}
#[test]
fn property_both_sides_every_partial_close_allocation_conserves_quantity_cash_basis() {
    for side in [-1, 1] {
        for original in 1..=6 {
            for still in 0..=original {
                for arrived in 1..=8 {
                    let mut l = seeded(config(), side * original, 100);
                    l = bind(&l, 2, false, -side * original);
                    if original > still {
                        l = bind(&l, 3, false, -side * (original - still));
                        l = fill(&l, 4, 3, -side * (original - still), 100, 0)
                    }
                    l = fill(&l, 5, 2, -side * arrived, 103, arrived.into());
                    let allocated = still.min(arrived);
                    assert_eq!(
                        qty(&l, Owner::Customer(user(1))),
                        side * (still - allocated)
                    );
                    assert_eq!(qty(&l, Owner::House), -side * (arrived - allocated));
                    assert_eq!(l.closes()[0].customer_filled, allocated as u64);
                    l.check_bridge().unwrap();
                }
            }
        }
    }
}
