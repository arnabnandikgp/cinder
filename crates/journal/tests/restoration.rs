//! Original-basis restoration over real production ledger/controller paths.
mod support;
use cinder_journal::{collateral, liquidation, model::*, orders, restoration as rc, risk, wire};
use cinder_kernel::{
    identity::*,
    ledger::{evidence::*, restoration::RestorationChange as R, *},
};
use support::*;

fn apply(l: &Ledger, key: RecordKey, c: R) -> Ledger {
    let e = Event {
        key,
        policy: config().policy,
        change: Change::Restoration(c),
    };
    assert_eq!(
        wire::decode_event(&wire::encode_event(&e).unwrap()).unwrap(),
        e
    );
    let out = l.apply(&e).unwrap();
    out.check_bridge().unwrap();
    out
}
fn initial(side: i64) -> Ledger {
    let mut l = Ledger::new(config()).unwrap();
    for e in [
        receipt(1, Owner::Customer(user(1)), 50000),
        receipt(2, Owner::Customer(user(2)), 50000),
        receipt(3, Owner::House, 100000),
    ] {
        l = l.apply(&e).unwrap();
    }
    for (n, qty) in [(1, 6), (2, 4)] {
        let mut a = attempt(n);
        a.request.account = user(n);
        l = l
            .apply(&Event {
                key: RecordKey::Attempt(a),
                policy: config().policy,
                change: Change::BindExecution {
                    market: q(0).unit(),
                    side: if side > 0 { Side::Buy } else { Side::Sell },
                },
            })
            .unwrap();
        l = l
            .apply(&event(
                10 + u64::from(n),
                Change::Fill {
                    target: FillTarget::Customer(a),
                    quantity: q(qty * side),
                    price: p(10000),
                },
            ))
            .unwrap();
    }
    l
}
fn declared(side: i64) -> Ledger {
    let l = initial(side);
    let cut = l.version();
    let l = apply(
        &l,
        RecordKey::Economic(key(20)),
        R::Observe {
            cut,
            quantity: q(-10 * side),
            price: p(10000),
            fee: cash(7),
            pnl: None,
        },
    );
    apply(
        &l,
        RecordKey::Request(request(21)),
        R::Declare { incident: key(20) },
    )
}
fn bound(side: i64) -> Ledger {
    apply(
        &declared(side),
        RecordKey::Attempt(attempt(22)),
        R::Bind {
            incident: key(20),
            target: 10,
        },
    )
}
fn fill(l: &Ledger, n: u64, count: i64, price: u64, eligible: bool) -> Ledger {
    apply(
        l,
        RecordKey::Economic(key(n)),
        R::Execution {
            attempt: attempt(22),
            quantity: q(count),
            price: p(price),
            fee: cash(i128::from(count.unsigned_abs()) * 2),
            pnl: None,
            eligible,
        },
    )
}
#[test]
fn rf1_both_sides_refund_basis_cash_chunking_and_every_prefix() {
    for side in [1, -1] {
        let start = bound(side);
        let px = if side > 0 { 10100 } else { 9900 };
        let full = fill(&start, 30, 10 * side, px, true);
        let mut chunks = start;
        for n in 1..=10 {
            chunks = fill(&chunks, 40 + n, side, px, true);
            assert_eq!(
                chunks.venue().positions()[0].quantity().lots(),
                n as i64 * side
            );
            assert_eq!(chunks.reductions()[0].consumed, n);
        }
        for owner in [
            Owner::Customer(user(1)),
            Owner::Customer(user(2)),
            Owner::House,
            Owner::Suspense,
        ] {
            assert_eq!(full.book(owner), chunks.book(owner));
        }
        for (id, n) in [(1, 6), (2, 4)] {
            let b = full.book(Owner::Customer(user(id))).unwrap();
            assert_eq!(b.cash(), cash(50000));
            assert_eq!(b.positions()[0].quantity(), q(n * side));
            assert_eq!(
                b.positions()[0].basis().atoms(),
                i128::from(n * side) * 10000
            );
        }
        assert_eq!(full.book(Owner::House).unwrap().cash(), cash(98973));
        assert_eq!(full.reductions()[0].spent, cash(1027));
    }
}
#[test]
fn private_cut_change_unqualified_native_adl_and_house_exposure_stay_observed() {
    let l = initial(1);
    let observed = apply(
        &l,
        RecordKey::Economic(key(20)),
        R::Observe {
            cut: 0,
            quantity: q(-10),
            price: p(10000),
            fee: cash(7),
            pnl: None,
        },
    );
    assert_eq!(observed.venue().positions()[0].quantity(), q(0));
    assert_eq!(observed.unresolved_attribution(), 1);
    assert!(
        observed
            .apply(&Event {
                key: RecordKey::Request(request(21)),
                policy: config().policy,
                change: Change::Restoration(R::Declare { incident: key(20) })
            })
            .is_err()
    );
    let cut = l.version();
    let observed = apply(
        &l,
        RecordKey::Economic(key(20)),
        R::Observe {
            cut,
            quantity: q(-10),
            price: p(10000),
            fee: cash(7),
            pnl: None,
        },
    );
    let changed = observed
        .apply(&receipt(23, Owner::Customer(user(1)), 1))
        .unwrap();
    assert!(
        changed
            .apply(&Event {
                key: RecordKey::Request(request(21)),
                policy: config().policy,
                change: Change::Restoration(R::Declare { incident: key(20) })
            })
            .is_err()
    );
}
#[test]
fn voided_allocations_are_house_owned_without_reweighting_other_customer() {
    let l = bound(1);
    let l = apply(
        &l,
        RecordKey::Request(request(23)),
        R::Void {
            market: q(0).unit(),
        },
    );
    let full = fill(&l, 30, 12, 10100, true);
    assert_eq!(
        full.book(Owner::Customer(user(1))).unwrap().positions()[0].quantity(),
        q(0)
    );
    assert_eq!(
        full.book(Owner::Customer(user(2))).unwrap().positions()[0].quantity(),
        q(4)
    );
    assert_eq!(
        full.book(Owner::House).unwrap().positions()[0].quantity(),
        q(8)
    );
    assert_eq!(full.reductions()[0].consumed, 10);
    assert_eq!(full.reductions()[0].excess, 8);
}
#[test]
fn wrong_side_and_ineligible_replacement_preserve_real_house_economics() {
    for (qty, eligible) in [(-3, true), (3, false)] {
        let next = fill(&bound(1), 30, qty, 10100, eligible);
        assert_eq!(next.reductions()[0].consumed, 0);
        assert_eq!(next.reductions()[0].excess, 3);
        assert_eq!(
            next.book(Owner::House).unwrap().positions()[0].quantity(),
            q(qty)
        );
    }
}
fn run(s: &mut Store, es: Vec<Event>, controls: Vec<Control>) -> cinder_journal::Committed {
    let mut tx = transaction(
        s.head(),
        u8::try_from(s.head().sequence + 1).unwrap(),
        es,
        controls,
    );
    for i in &mut tx.inputs {
        i.observed_at = tx.at;
    }
    assert_eq!(
        wire::decode_transaction(&wire::encode_transaction(&tx).unwrap()).unwrap(),
        tx
    );
    s.commit(tx).unwrap()
}
fn prepared(t: &Temp) -> Store {
    prepared_with(t, true)
}
fn prepared_with(t: &Temp, expose: bool) -> Store {
    prepared_void(t, expose, None)
}
fn void_control() -> Control {
    Control::Restoration(rc::Action::Void {
        request: request(23),
        market: q(0).unit(),
        authority_epoch: 1,
        authenticated_digest: rc::void_digest(request(23), q(0).unit(), 1).unwrap(),
    })
}
fn prepared_void(t: &Temp, expose: bool, void_after_prepare: Option<bool>) -> Store {
    let mut s = t.create();
    let l = initial(1);
    run(
        &mut s,
        l.events().to_vec(),
        vec![Control::Order(orders::Action::AdvanceAuthority {
            account: user(1),
            epoch: 1,
        })],
    );
    let cut = s.state().unwrap().ledger().version();
    let observed = event(
        20,
        Change::Restoration(R::Observe {
            cut,
            quantity: q(-10),
            price: p(10000),
            fee: cash(7),
            pnl: None,
        }),
    );
    let result = run(
        &mut s,
        vec![observed],
        vec![Control::Restoration(rc::Action::Declare {
            request: request(21),
            expected_version: cut + 1,
            incident: key(20),
        })],
    );
    assert_eq!(result.receipt.controls, None);
    let l = s.state().unwrap().ledger();
    let reconcile = event(
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
    run(&mut s, vec![reconcile], vec![]);
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
                max_mark_age: 1000,
                max_check_age: 1000,
            },
        },
        marks: vec![MarkObservation {
            price: p(10000),
            evidence: key(999),
            observed_at: 10,
            valid_until: 1000,
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
            gross_limit: cash(1000000),
            net_limit: cash(1000000),
        }],
        buffer: cash(0),
        horizon_ms: 100,
        valid_until: 1000,
        paths: vec![risk::Path {
            id: [1; 32],
            steps: vec![risk::Step {
                after_ms: 1,
                marks: vec![p(10000)],
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
    let c = Control::Liquidation(liquidation::Action::Install {
        expected_version: s.state().unwrap().ledger().version(),
        policy: Box::new(liquidation::Policy {
            revision: config().policy,
            authority_epoch: 1,
            valid_until: 1000,
            limits: vec![liquidation::Limit {
                market: q(0).unit(),
                maximum_lots: 10,
                minimum: p(9900),
                maximum: p(10100),
                fee_per_lot: cash(2),
                additional_per_lot: cash(0),
            }],
        }),
    });
    assert_eq!(run(&mut s, vec![], vec![c]).receipt.controls, None);
    let c = Control::Restoration(rc::Action::Install {
        expected_version: s.state().unwrap().ledger().version(),
        revision: config().policy,
        lifetime_limit: cash(50000),
    });
    assert_eq!(run(&mut s, vec![], vec![c]).receipt.controls, None);
    let p = rc::Proposal {
        limit_revision: config().policy,
        cap_revision: config().policy,
        attempt: attempt(22),
        incident: key(20),
        target: 10,
        expected_version: s.state().unwrap().ledger().version(),
        authority_epoch: 1,
        expires_at: 100,
    };
    if void_after_prepare == Some(false) {
        assert_eq!(
            run(&mut s, vec![], vec![void_control()]).receipt.controls,
            None
        );
    }
    let p = rc::Proposal {
        expected_version: s.state().unwrap().ledger().version(),
        ..p
    };
    let c = Control::Restoration(rc::Action::Prepare {
        authenticated_digest: p.digest().unwrap(),
        proposal: Box::new(p),
    });
    let mut controls = vec![c];
    if void_after_prepare == Some(true) {
        controls.push(void_control());
    }
    if expose {
        controls.push(Control::Expose(attempt(22)));
    }
    assert_eq!(run(&mut s, vec![], controls).receipt.controls, None);
    s
}
fn receipt_restore(id: u64, qty: i64, price: u64, time: u64) -> Event {
    event(
        id,
        Change::Restoration(R::Receipt {
            attempt: attempt(22),
            quantity: q(qty),
            price: p(price),
            fee: cash(i128::from(qty.unsigned_abs()) * 2),
            pnl: None,
            executed_at: time,
        }),
    )
}

#[test]
fn one_void_before_prepare_or_expose_does_not_block_other_fixed_awards() {
    for after_prepare in [false, true] {
        let t = Temp::new();
        let mut s = prepared_void(&t, true, Some(after_prepare));
        assert_eq!(
            run(&mut s, vec![receipt_restore(30, 10, 10100, 10)], vec![])
                .receipt
                .inputs,
            vec![InputResult::Normalized(Disposition::Applied)]
        );
        let l = s.state().unwrap().ledger();
        assert_eq!(
            l.book(Owner::Customer(user(1))).unwrap().positions()[0].quantity(),
            q(0)
        );
        assert_eq!(
            l.book(Owner::Customer(user(2))).unwrap().positions()[0].quantity(),
            q(4)
        );
        assert_eq!(
            l.book(Owner::House).unwrap().positions()[0].quantity(),
            q(6)
        );
        assert_eq!(l.reductions()[0].consumed, 10);
        assert_eq!(l.reductions()[0].rows[1].amount, 4);
        assert!(s.state().unwrap().restorations()[0].contained);
        assert_eq!(t.open().state().unwrap(), s.state().unwrap());
    }
}

#[test]
fn fully_void_incident_cannot_authorize_a_house_only_replacement() {
    let mut l = declared(1);
    for id in [1, 2] {
        let mut r = request(40 + id);
        r.account = user(id);
        l = apply(
            &l,
            RecordKey::Request(r),
            R::Void {
                market: q(0).unit(),
            },
        );
    }
    assert_eq!(
        l.apply(&Event {
            key: RecordKey::Attempt(attempt(22)),
            policy: config().policy,
            change: Change::Restoration(R::Bind {
                incident: key(20),
                target: 10
            }),
        }),
        Err(LedgerError::Attribution)
    );
    let t = Temp::new();
    let mut s = prepared_void(&t, false, Some(false));
    let mut r = request(24);
    r.account = user(2);
    assert_eq!(
        run(
            &mut s,
            vec![],
            vec![
                Control::Order(orders::Action::AdvanceAuthority {
                    account: user(2),
                    epoch: 1
                }),
                Control::Restoration(rc::Action::Void {
                    request: r,
                    market: q(0).unit(),
                    authority_epoch: 1,
                    authenticated_digest: rc::void_digest(r, q(0).unit(), 1).unwrap(),
                }),
            ]
        )
        .receipt
        .controls,
        None
    );
    let result = run(&mut s, vec![], vec![Control::Expose(attempt(22))]);
    assert_eq!(result.receipt.controls, Some(ControlError::Unqualified));
    assert!(result.exposures.is_empty());
}

#[test]
fn house_close_label_does_not_void_customer_awards_but_private_close_does() {
    use cinder_kernel::ledger::close::CloseChange;
    for house in [false, true] {
        let mut l = bound(1);
        // Establish real inventory for a bounded house unwind / private close.
        l = fill(&l, 30, 2, 10100, true);
        if house {
            l = l
                .apply(&event(
                    31,
                    Change::Fill {
                        target: FillTarget::House,
                        quantity: q(1),
                        price: p(10000),
                    },
                ))
                .unwrap();
        }
        l = l
            .apply(&Event {
                key: RecordKey::Attempt(attempt(40)),
                policy: config().policy,
                change: Change::Close(CloseChange::Bind {
                    house,
                    quantity: q(-1),
                }),
            })
            .unwrap();
        l = l
            .apply(&event(
                41,
                Change::Close(CloseChange::Execution {
                    attempt: attempt(40),
                    quantity: q(-1),
                    price: p(10000),
                    fee: cash(0),
                    pnl: None,
                    customer_allowed: true,
                }),
            ))
            .unwrap();
        assert_eq!(l.reductions()[0].rows[0].void, !house);
        assert!(!l.reductions()[0].rows[1].void);
        l.check_bridge().unwrap();
        let full = fill(&l, 42, 8, 10100, true);
        assert_eq!(
            full.reductions()[0].rows[0].restored,
            if house { 6 } else { 1 }
        );
        assert_eq!(full.reductions()[0].rows[1].restored, 4);
    }
}
#[test]
fn durable_funded_replacement_follows_frozen_quota_and_replays() {
    let t = Temp::new();
    let mut s = prepared(&t);
    let result = run(&mut s, vec![receipt_restore(30, 3, 10100, 10)], vec![]);
    assert_eq!(
        result.receipt.inputs,
        vec![InputResult::Normalized(Disposition::Applied)]
    );
    assert_eq!(s.state().unwrap().ledger().reductions()[0].consumed, 3);
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
    assert_eq!(
        s.state()
            .unwrap()
            .ledger()
            .book(Owner::Customer(user(2)))
            .unwrap()
            .positions()[0]
            .quantity(),
        q(1)
    );
    let replay = t.open();
    assert_eq!(s.state().unwrap(), replay.state().unwrap());
    drop(replay);
    assert_eq!(
        run(&mut s, vec![receipt_restore(30, 3, 10100, 10)], vec![])
            .receipt
            .inputs,
        vec![InputResult::Normalized(Disposition::Duplicate)]
    );
    assert_eq!(s.state().unwrap().ledger().reductions()[0].consumed, 3);
    assert!(
        s.state()
            .unwrap()
            .holds()
            .iter()
            .any(|h| h.request == request(22) && h.active)
    );
}
#[test]
fn changed_intent_late_receipt_and_fee_limit_violation_never_reweight_customers() {
    let t = Temp::new();
    let mut s = prepared(&t);
    let c = Control::Restoration(rc::Action::Void {
        request: request(23),
        market: q(0).unit(),
        authority_epoch: 1,
        authenticated_digest: rc::void_digest(request(23), q(0).unit(), 1).unwrap(),
    });
    assert_eq!(run(&mut s, vec![], vec![c]).receipt.controls, None);
    run(&mut s, vec![receipt_restore(30, 10, 10100, 10)], vec![]);
    let l = s.state().unwrap().ledger();
    assert_eq!(
        l.book(Owner::Customer(user(1))).unwrap().positions()[0].quantity(),
        q(0)
    );
    assert_eq!(
        l.book(Owner::Customer(user(2))).unwrap().positions()[0].quantity(),
        q(4)
    );
    assert!(s.state().unwrap().restorations()[0].contained);
    for (time, price) in [(101, 10100), (10, 10101)] {
        let t = Temp::new();
        let mut s = prepared(&t);
        let mut tx = transaction(
            s.head(),
            80,
            vec![receipt_restore(30, 2, price, time)],
            vec![],
        );
        tx.at = 110;
        for i in &mut tx.inputs {
            i.observed_at = 110;
        }
        s.commit(tx).unwrap();
        assert_eq!(s.state().unwrap().ledger().reductions()[0].consumed, 0);
        assert_eq!(
            s.state()
                .unwrap()
                .ledger()
                .book(Owner::House)
                .unwrap()
                .positions()[0]
                .quantity(),
            q(2)
        );
    }
}

fn terminal(
    s: &mut Store,
    n: u64,
    filled: i64,
    ids: &[u64],
    release: bool,
) -> cinder_journal::Committed {
    let controls = if release {
        vec![Control::Order(orders::Action::Release {
            request: request(22),
        })]
    } else {
        vec![]
    };
    let mut tx = transaction(
        s.head(),
        u8::try_from(s.head().sequence + 1).unwrap(),
        vec![],
        controls,
    );
    tx.order_observations.push(orders::Observation {
        key: key(n),
        attempt: attempt(22),
        authority_epoch: 1,
        observed_at: 10,
        raw: raw(b"synthetic qualified complete restoration history"),
        status: orders::Status::Terminal(orders::Terminal {
            filled: q(filled),
            executions: ids.iter().map(|id| key(*id)).collect(),
            through: 0,
        }),
    });
    s.commit(tx).unwrap()
}
#[test]
fn no_fill_partial_and_full_terminal_release_only_exact_history() {
    for n in [0, 3, 10] {
        let t = Temp::new();
        let mut s = prepared(&t);
        // Terminal can precede the referenced fills, but cannot release yet.
        let ids = if n == 0 { vec![] } else { vec![30] };
        let first = terminal(&mut s, 800, n, &ids, true);
        assert_eq!(first.receipt.controls.is_none(), n == 0);
        if n > 0 {
            run(&mut s, vec![receipt_restore(30, n, 10100, 10)], vec![]);
            assert_eq!(
                run(
                    &mut s,
                    vec![],
                    vec![Control::Order(orders::Action::Release {
                        request: request(22)
                    })]
                )
                .receipt
                .controls,
                None
            );
        }
        assert!(
            !s.state()
                .unwrap()
                .holds()
                .iter()
                .find(|h| h.request == request(22))
                .unwrap()
                .active
        );
        assert_eq!(
            s.state().unwrap().ledger().venue().positions()[0].quantity(),
            q(n)
        );
        let replay = t.open();
        assert_eq!(s.state().unwrap(), replay.state().unwrap());
    }
}
#[test]
fn terminal_contradiction_reencumbers_and_scoped_cancel_remains_available() {
    let t = Temp::new();
    let mut s = prepared(&t);
    assert_eq!(terminal(&mut s, 800, 0, &[], true).receipt.controls, None);
    run(&mut s, vec![receipt_restore(30, 2, 10100, 10)], vec![]);
    assert_eq!(s.state().unwrap().ledger().reductions()[0].consumed, 0);
    assert!(
        s.state()
            .unwrap()
            .holds()
            .iter()
            .find(|h| h.request == request(22))
            .unwrap()
            .active
    );
    let a = AttemptKey {
        attempt: AttemptId::new([99; 32]).unwrap(),
        ..attempt(22)
    };
    assert_eq!(
        run(
            &mut s,
            vec![],
            vec![
                Control::Order(orders::Action::PrepareCancel {
                    attempt: a,
                    authority_epoch: 1,
                    expires_at: 100
                }),
                Control::Expose(a)
            ]
        )
        .receipt
        .controls,
        None
    );
}
#[test]
fn post_submission_capital_shock_does_not_revoke_owed_restoration() {
    use cinder_kernel::ledger::economics::EconomicChange as E;
    let t = Temp::new();
    let mut s = prepared(&t);
    let shock = event(
        28,
        Change::Economics(E::Execution {
            target: FillTarget::House,
            quantity: q(1),
            price: p(10000),
            fee: cash(200000),
            pnl: None,
        }),
    );
    run(&mut s, vec![shock], vec![]);
    assert!(!s.state().unwrap().risk_report().unwrap().admissible);
    run(&mut s, vec![receipt_restore(30, 10, 10100, 10)], vec![]);
    assert_eq!(
        s.state()
            .unwrap()
            .ledger()
            .book(Owner::Customer(user(1)))
            .unwrap()
            .cash(),
        cash(50000)
    );
    assert_eq!(
        s.state()
            .unwrap()
            .ledger()
            .book(Owner::Customer(user(2)))
            .unwrap()
            .positions()[0]
            .quantity(),
        q(4)
    );
    assert_eq!(s.state().unwrap().ledger().reductions()[0].consumed, 10);
    assert!(
        s.state()
            .unwrap()
            .ledger()
            .book(Owner::House)
            .unwrap()
            .cash()
            .atoms()
            < 0
    );
}
#[test]
fn funding_uses_actual_missing_and_restored_quantities_without_gap_compensation() {
    use cinder_kernel::{
        ledger::economics::{EconomicChange as E, FundingRate},
        math::Rounding,
    };
    let mut l = bound(1);
    for (step, quantity) in [(0, 0), (1, 3), (2, 10)] {
        if step == 1 {
            l = fill(&l, 50, 3, 10100, true);
        }
        if step == 2 {
            l = fill(&l, 51, 7, 10100, true);
        }
        let id = 100 + step * 10;
        l = l
            .apply(&event(
                id,
                Change::Economics(E::FundingBoundary {
                    market: q(0).unit(),
                    expected_version: l.version(),
                }),
            ))
            .unwrap();
        l = l
            .apply(&event(
                id + 1,
                Change::Economics(E::FundingInputs {
                    boundary: key(id),
                    rate: Some(FundingRate {
                        numerator: cash(-1),
                        denominator: 1,
                        rounding: Rounding::Exact,
                    }),
                    native: Some(cash(-quantity)),
                }),
            ))
            .unwrap();
        l = l
            .apply(&event(
                id + 2,
                Change::Economics(E::FundingSettlement {
                    boundary: key(id),
                    native: cash(-quantity),
                }),
            ))
            .unwrap();
    }
    assert_eq!(
        l.book(Owner::Customer(user(1))).unwrap().cash(),
        cash(50000 - 2 - 6)
    );
    assert_eq!(
        l.book(Owner::Customer(user(2))).unwrap().cash(),
        cash(50000 - 1 - 4)
    );
    assert_eq!(l.book(Owner::House).unwrap().cash(), cash(98973));
    l.check_bridge().unwrap();
}
#[test]
fn raw_allocation_manifest_bypass_is_rejected_but_original_evidence_survives() {
    let t = Temp::new();
    let mut s = prepared(&t);
    let forged = event(
        30,
        Change::Restoration(R::Execution {
            attempt: attempt(22),
            quantity: q(10),
            price: p(10100),
            fee: cash(20),
            pnl: None,
            eligible: true,
        }),
    );
    assert_eq!(
        run(&mut s, vec![forged], vec![]).receipt.inputs,
        vec![InputResult::EnvelopeRejected]
    );
    assert_eq!(s.state().unwrap().ledger().reductions()[0].received, 0);
    assert_eq!(s.state().unwrap().unresolved_raw(), 1);
}
#[test]
fn signed_basis_and_fee_residues_survive_all_512_chunkings_per_direction() {
    for side in [1, -1] {
        for mask in 0..512 {
            let mut l = bound(side);
            let mut pending = 0;
            for i in 0..10 {
                pending += 1;
                if i == 9 || mask & (1 << i) != 0 {
                    l = fill(
                        &l,
                        100 + i,
                        side * pending,
                        if side > 0 { 10100 } else { 9900 },
                        true,
                    );
                    pending = 0;
                }
            }
            let full = fill(
                &bound(side),
                90,
                side * 10,
                if side > 0 { 10100 } else { 9900 },
                true,
            );
            for owner in [
                Owner::Customer(user(1)),
                Owner::Customer(user(2)),
                Owner::House,
            ] {
                assert_eq!(l.book(owner), full.book(owner));
            }
        }
    }
}

#[test]
fn nondivisible_removed_basis_returns_exactly_and_split_native_value_keeps_residue() {
    for side in [1, -1] {
        let mut es = initial(side).events().to_vec();
        for e in &mut es {
            if e.key == RecordKey::Economic(key(11))
                && let Change::Fill { quantity, .. } = &mut e.change
            {
                *quantity = q(5 * side);
            }
        }
        es.push(event(
            13,
            Change::Fill {
                target: FillTarget::Customer(attempt(1)),
                quantity: q(side),
                price: p(10001),
            },
        ));
        let mut original = Ledger::new(config()).unwrap();
        for e in es {
            original = original.apply(&e).unwrap();
        }
        let cut = original.version();
        let l = apply(
            &original,
            RecordKey::Economic(key(20)),
            R::Observe {
                cut,
                quantity: q(-7 * side),
                price: p(10000),
                fee: cash(7),
                pnl: None,
            },
        );
        let l = apply(
            &l,
            RecordKey::Request(request(21)),
            R::Declare { incident: key(20) },
        );
        let l = apply(
            &l,
            RecordKey::Attempt(attempt(22)),
            R::Bind {
                incident: key(20),
                target: 7,
            },
        );
        let full = fill(&l, 30, 7 * side, if side > 0 { 10100 } else { 9900 }, true);
        let mut chunks = l;
        for i in 0..7 {
            chunks = fill(
                &chunks,
                40 + i,
                side,
                if side > 0 { 10100 } else { 9900 },
                true,
            );
        }
        for id in [1, 2] {
            assert_eq!(
                original.book(Owner::Customer(user(id))),
                full.book(Owner::Customer(user(id)))
            );
            assert_eq!(
                full.book(Owner::Customer(user(id))),
                chunks.book(Owner::Customer(user(id)))
            );
        }
        assert_eq!(full.book(Owner::House), chunks.book(Owner::House));
    }
    let mut c = config();
    c.markets[0] = cinder_kernel::position::Market::new(q(0).unit(), 3, 2).unwrap();
    let mut original = Ledger::new(c).unwrap();
    for e in initial(1).events() {
        original = original.apply(e).unwrap();
    }
    let cut = original.version();
    let l = apply(
        &original,
        RecordKey::Economic(key(20)),
        R::Observe {
            cut,
            quantity: q(-2),
            price: p(10000),
            fee: cash(7),
            pnl: None,
        },
    );
    let l = apply(
        &l,
        RecordKey::Request(request(21)),
        R::Declare { incident: key(20) },
    );
    let l = apply(
        &l,
        RecordKey::Attempt(attempt(22)),
        R::Bind {
            incident: key(20),
            target: 2,
        },
    );
    let full = fill(&l, 30, 2, 10101, true);
    for id in [1, 2] {
        assert_eq!(
            full.book(Owner::Customer(user(id))),
            original.book(Owner::Customer(user(id)))
        );
    }
    assert_eq!(full.book(Owner::House).unwrap().cash(), cash(100000 - 314));
}

#[test]
fn cap_policy_changes_do_not_rewrite_existing_owed_receipt_or_reset_spent() {
    let t = Temp::new();
    let mut s = prepared(&t);
    let c = Control::Restoration(rc::Action::Install {
        expected_version: s.state().unwrap().ledger().version(),
        revision: PolicyVersion::new(2).unwrap(),
        lifetime_limit: cash(0),
    });
    assert_eq!(run(&mut s, vec![], vec![c]).receipt.controls, None);
    run(&mut s, vec![receipt_restore(30, 10, 10100, 10)], vec![]);
    assert_eq!(
        s.state().unwrap().ledger().reductions()[0].spent,
        cash(1027)
    );
    assert_eq!(
        s.state()
            .unwrap()
            .ledger()
            .book(Owner::Customer(user(1)))
            .unwrap()
            .cash(),
        cash(50000)
    );
    assert_eq!(
        terminal(&mut s, 800, 10, &[30], true).receipt.controls,
        None
    );
    let c = Control::Restoration(rc::Action::Install {
        expected_version: s.state().unwrap().ledger().version(),
        revision: PolicyVersion::new(3).unwrap(),
        lifetime_limit: cash(50000),
    });
    assert_eq!(run(&mut s, vec![], vec![c]).receipt.controls, None);
    assert_eq!(
        s.state().unwrap().ledger().reductions()[0].spent,
        cash(1027)
    );
}

#[test]
fn final_cut_revocation_rollback_and_pre_exposure_refusal_are_atomic() {
    let t = Temp::new();
    let mut s = prepared_with(&t, false);
    let cut = s.state().unwrap().ledger().version();
    let revoke = Control::Restoration(rc::Action::Install {
        expected_version: cut,
        revision: PolicyVersion::new(2).unwrap(),
        lifetime_limit: cash(0),
    });
    assert!(
        run(
            &mut s,
            vec![],
            vec![Control::Expose(attempt(22)), revoke.clone()]
        )
        .receipt
        .controls
        .is_some()
    );
    assert!(
        !s.state()
            .unwrap()
            .attempts()
            .iter()
            .find(|a| a.key == attempt(22))
            .unwrap()
            .possibly_exposed
    );
    assert_eq!(run(&mut s, vec![], vec![revoke]).receipt.controls, None);
    assert!(
        run(&mut s, vec![], vec![Control::Expose(attempt(22))])
            .receipt
            .controls
            .is_some()
    );
    assert_eq!(
        run(&mut s, vec![], vec![Control::Release(request(22))])
            .receipt
            .controls,
        None
    );
    run(&mut s, vec![receipt_restore(30, 2, 10100, 10)], vec![]);
    assert_eq!(s.state().unwrap().ledger().reductions()[0].consumed, 0);
    assert!(
        s.state()
            .unwrap()
            .holds()
            .iter()
            .find(|h| h.request == request(22))
            .unwrap()
            .active
    );
}
#[test]
fn source_execution_time_not_receive_time_and_fee_bounds_control_eligibility() {
    for (exec, received, fee, expected) in [
        (10, 110, 4, 2),
        (9, 10, 4, 0),
        (11, 10, 4, 0),
        (10, 10, 5, 0),
    ] {
        let t = Temp::new();
        let mut s = prepared(&t);
        let mut e = receipt_restore(30, 2, 10100, exec);
        if let Change::Restoration(R::Receipt { fee: f, .. }) = &mut e.change {
            *f = cash(fee);
        }
        let mut tx = transaction(s.head(), 80, vec![e], vec![]);
        tx.at = received;
        for i in &mut tx.inputs {
            i.observed_at = received;
        }
        s.commit(tx).unwrap();
        assert_eq!(
            s.state().unwrap().ledger().reductions()[0].consumed,
            expected
        );
    }
}

#[test]
fn prospective_restoration_executes_same_postings_without_mutating_live_book() {
    let t = Temp::new();
    let mut s = prepared(&t);
    let future = event(
        700,
        Change::Restoration(R::Execution {
            attempt: attempt(22),
            quantity: q(3),
            price: p(10100),
            fee: cash(6),
            pnl: None,
            eligible: true,
        }),
    );
    let policy = risk::Policy {
        revision: PolicyVersion::new(2).unwrap(),
        markets: vec![risk::MarketRule {
            market: q(0).unit(),
            maximum_leverage: 100000,
            maintenance_bps: 500,
            native_maintenance_bps: 500,
            gross_limit: cash(1000000),
            net_limit: cash(1000000),
        }],
        buffer: cash(0),
        horizon_ms: 100,
        valid_until: 1000,
        paths: vec![risk::Path {
            id: [2; 32],
            steps: vec![risk::Step {
                after_ms: 1,
                marks: vec![p(10000)],
                events: vec![future],
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
    let report = s.state().unwrap().risk_report().unwrap();
    assert_eq!(
        report.paths[0].prefixes.last().unwrap().customers[0].initial,
        cash(2000)
    );
    assert!(report.minimum_free.atoms() < report.current.free_capital.atoms());
    assert_eq!(s.state().unwrap().ledger().reductions()[0].consumed, 0);
    assert_eq!(
        s.state()
            .unwrap()
            .ledger()
            .book(Owner::Customer(user(1)))
            .unwrap()
            .positions()[0]
            .quantity(),
        q(0)
    );
}
