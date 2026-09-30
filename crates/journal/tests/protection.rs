//! Repeated protection episodes over production ledger primitives, not live coverage.
mod support;
use cinder_journal::{collateral, model::*, protection as controller, risk, wire};
use cinder_kernel::{
    identity::*,
    ledger::{funds::*, protection::*, *},
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
fn coverage(limit: i128) -> controller::Policy {
    controller::Policy {
        revision: PolicyVersion::new(1).unwrap(),
        authority_epoch: 1,
        valid_until: 100,
        lifetime_limit: cash(limit),
        customer_limit: cash(limit),
        coverage: [1; 32],
    }
}
fn install(s: &mut Store, p: controller::Policy) {
    let c = Control::Protection(controller::Action::Install {
        expected_version: s.state().unwrap().ledger().version(),
        policy: Box::new(p),
    });
    assert_eq!(run(s, vec![], vec![c]).receipt.controls, None);
}
fn decision(s: &Store, n: u8, change: ProtectionChange) -> Control {
    let state = s.state().unwrap();
    let p = state.protection_policy().unwrap();
    let d = controller::Decision {
        request: request(n),
        expected_version: state.ledger().version(),
        policy: p.revision,
        authority_epoch: p.authority_epoch,
        expires_at: 100,
        change,
    };
    Control::Protection(controller::Action::Apply {
        authenticated_digest: d.digest().unwrap(),
        decision: Box::new(d),
    })
}
fn control(s: &mut Store, n: u8, change: ProtectionChange) -> cinder_journal::Committed {
    let c = decision(s, n, change);
    run(s, vec![], vec![c])
}
fn store(t: &Temp, limit: i128) -> Store {
    let mut s = t.create();
    run(
        &mut s,
        vec![
            receipt(1, Owner::Customer(user(1)), 100),
            receipt(2, Owner::Customer(user(2)), 100),
            receipt(3, Owner::House, 100),
        ],
        vec![],
    );
    install(&mut s, coverage(limit));
    assert_eq!(
        control(&mut s, 10, ProtectionChange::Designate { delta: cash(100) })
            .receipt
            .controls,
        None
    );
    s
}
fn actual_loss(s: &mut Store, id: u8, n: u64) {
    let mut buy = attempt(id * 10);
    buy.request.account = user(id);
    let mut sell = attempt(id * 10 + 1);
    sell.request.account = user(id);
    let mut es = vec![];
    for (a, side, qty, price) in [(buy, Side::Buy, 2, 100), (sell, Side::Sell, -2, 40)] {
        es.push(Event {
            key: RecordKey::Attempt(a),
            policy: config().policy,
            change: Change::BindExecution {
                market: q(0).unit(),
                side,
            },
        });
        es.push(event(
            n + u64::from(qty < 0),
            Change::Fill {
                target: FillTarget::Customer(a),
                quantity: q(qty),
                price: p(price),
            },
        ));
    }
    run(s, es, vec![]);
}
fn earmark_absorb(
    s: &mut Store,
    n: u8,
    claim: RequestKey,
    amount: i128,
) -> cinder_journal::Committed {
    assert_eq!(
        control(
            s,
            n,
            ProtectionChange::Commit {
                claim,
                amount: cash(amount)
            }
        )
        .receipt
        .controls,
        None
    );
    control(
        s,
        n + 1,
        ProtectionChange::Absorb {
            allocations: vec![Allocation {
                claim,
                amount: cash(amount),
            }],
        },
    )
}
fn join_risk(s: &mut Store, buffer: i128) {
    use cinder_kernel::ledger::evidence::*;
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
    run(s, vec![e], vec![]);
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
            price: p(100),
            evidence: key(999),
            observed_at: 10,
            valid_until: 100,
            qualified: true,
        }],
    };
    assert_eq!(
        run(s, vec![], vec![Control::Collateral(c)])
            .receipt
            .controls,
        None
    );
    let p = risk::Policy {
        revision: config().policy,
        markets: vec![risk::MarketRule {
            market: q(0).unit(),
            maximum_leverage: 100000,
            maintenance_bps: 500,
            native_maintenance_bps: 500,
            gross_limit: cash(10000),
            net_limit: cash(10000),
        }],
        buffer: cash(buffer),
        horizon_ms: 100,
        valid_until: 100,
        paths: vec![risk::Path {
            id: [1; 32],
            steps: vec![risk::Step {
                after_ms: 1,
                marks: vec![p(100)],
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
        policy: Box::new(p),
    });
    assert_eq!(run(s, vec![], vec![c]).receipt.controls, None);
}

#[test]
fn controller_decisions_and_repeated_episodes_replay_identically() {
    let t = Temp::new();
    let mut s = store(&t, 100);
    actual_loss(&mut s, 1, 100);
    run(&mut s, vec![recognize(1, 30, 20, Kind::Deficit)], vec![]);
    assert_eq!(
        earmark_absorb(&mut s, 31, request(30), 20).receipt.controls,
        None
    );
    run(
        &mut s,
        vec![receipt(200, Owner::Customer(user(1)), 120)],
        vec![],
    );
    actual_loss(&mut s, 2, 300);
    run(&mut s, vec![recognize(2, 40, 20, Kind::Deficit)], vec![]);
    assert_eq!(
        earmark_absorb(&mut s, 41, request_for(2, 40), 20)
            .receipt
            .controls,
        None
    );
    let expected = s.state().unwrap().clone();
    let head = s.head();
    drop(s);
    let s = t.open();
    assert_eq!(s.head(), head);
    assert_eq!(s.state().unwrap(), &expected);
    assert_eq!(expected.ledger().protection().absorbed_total, cash(40));
    assert_eq!(expected.ledger().protection().reserve, cash(80));
    assert_eq!(account(expected.ledger(), 1), 100);
}

#[test]
fn global_extraction_cap_survives_recovery_new_accounts_and_policy_revision() {
    let t = Temp::new();
    let mut s = store(&t, 20);
    actual_loss(&mut s, 1, 100);
    run(&mut s, vec![recognize(1, 30, 20, Kind::Deficit)], vec![]);
    assert_eq!(
        earmark_absorb(&mut s, 31, request(30), 20).receipt.controls,
        None
    );
    run(
        &mut s,
        vec![receipt(200, Owner::Customer(user(1)), 20)],
        vec![],
    );
    let mut p = coverage(20);
    p.revision = PolicyVersion::new(2).unwrap();
    install(&mut s, p);
    actual_loss(&mut s, 2, 300);
    run(&mut s, vec![recognize(2, 40, 20, Kind::Deficit)], vec![]);
    assert_eq!(
        earmark_absorb(&mut s, 41, request_for(2, 40), 20)
            .receipt
            .controls,
        Some(ControlError::Capacity)
    );
    assert_eq!(
        s.state().unwrap().ledger().protection().absorbed_total,
        cash(20)
    );
    assert_eq!(account(s.state().unwrap().ledger(), 2), -20);
    // A new operation ID cannot resurrect the consumed global budget.
    assert_eq!(
        control(
            &mut s,
            50,
            ProtectionChange::Absorb {
                allocations: vec![Allocation {
                    claim: request_for(2, 40),
                    amount: cash(1)
                }]
            }
        )
        .receipt
        .controls,
        Some(ControlError::Capacity)
    );
}

#[test]
fn zero_coverage_disables_absorption_without_deleting_loss() {
    let t = Temp::new();
    let mut s = store(&t, 0);
    actual_loss(&mut s, 1, 100);
    run(&mut s, vec![recognize(1, 30, 20, Kind::Deficit)], vec![]);
    assert_eq!(
        earmark_absorb(&mut s, 31, request(30), 20).receipt.controls,
        Some(ControlError::Capacity)
    );
    assert_eq!(
        s.state().unwrap().ledger().protection().claims[0]
            .remaining()
            .unwrap(),
        cash(20)
    );
}

#[test]
fn owed_remediation_survives_refused_controls_and_depleted_reserve() {
    let t = Temp::new();
    let mut s = store(&t, 0);
    let c = decision(&s, 31, ProtectionChange::Designate { delta: cash(-1) });
    let result = run(
        &mut s,
        vec![recognize(1, 30, 150, Kind::Remediation)],
        vec![c],
    );
    assert_eq!(result.receipt.controls, Some(ControlError::Invalid));
    let l = s.state().unwrap().ledger();
    assert_eq!(account(l, 1), 250);
    assert_eq!(l.protection().reserve, cash(-50));
    assert_eq!(l.diagnostics(&[p(100)]).unwrap().shortfall, cash(50));
    let before = l.clone();
    drop(s);
    assert_eq!(t.open().state().unwrap().ledger(), &before);
}

#[test]
fn external_input_cannot_bypass_coverage_controller() {
    let t = Temp::new();
    let mut s = store(&t, 0);
    actual_loss(&mut s, 1, 100);
    run(&mut s, vec![recognize(1, 30, 20, Kind::Deficit)], vec![]);
    let result = run(
        &mut s,
        vec![
            commit_claim(31, request(30), 20),
            absorb(32, &[(request(30), 20)]),
        ],
        vec![],
    );
    assert_eq!(
        result.receipt.inputs,
        vec![InputResult::EnvelopeRejected, InputResult::EnvelopeRejected]
    );
    assert_eq!(
        s.state().unwrap().ledger().protection().absorbed_total,
        cash(0)
    );
}

#[test]
fn admin_digest_epoch_expiry_and_financial_cut_are_bound() {
    let t = Temp::new();
    let mut s = store(&t, 100);
    for i in 0..4 {
        let mut c = decision(&s, 30 + i, ProtectionChange::Designate { delta: cash(1) });
        if let Control::Protection(controller::Action::Apply {
            decision,
            authenticated_digest,
        }) = &mut c
        {
            match i {
                0 => *authenticated_digest = [0; 32],
                1 => decision.authority_epoch += 1,
                2 => decision.expected_version += 1,
                _ => decision.expires_at = 9,
            };
            if i != 0 {
                *authenticated_digest = decision.digest().unwrap()
            }
        }
        assert!(run(&mut s, vec![], vec![c]).receipt.controls.is_some());
    }
    assert_eq!(s.state().unwrap().ledger().protection().reserve, cash(100));
}

#[test]
fn designated_free_capital_and_claim_commitment_join_risk_and_restrict_releases() {
    let t = Temp::new();
    let mut s = store(&t, 100);
    join_risk(&mut s, 50);
    assert_eq!(
        s.state()
            .unwrap()
            .risk_report()
            .unwrap()
            .current
            .free_capital,
        cash(100)
    );
    assert_eq!(
        control(&mut s, 30, ProtectionChange::Designate { delta: cash(-51) })
            .receipt
            .controls,
        Some(ControlError::Capacity)
    );
    assert_eq!(
        control(&mut s, 31, ProtectionChange::Designate { delta: cash(-50) })
            .receipt
            .controls,
        None
    );
    assert_eq!(
        s.state()
            .unwrap()
            .risk_report()
            .unwrap()
            .current
            .free_capital,
        cash(50)
    );
    actual_loss(&mut s, 1, 100);
    run(&mut s, vec![recognize(1, 40, 20, Kind::Deficit)], vec![]);
    assert_eq!(
        run(&mut s, vec![], vec![reserve(50, 1)]).receipt.controls,
        Some(ControlError::Unqualified)
    );
    assert!(
        control(&mut s, 51, ProtectionChange::Designate { delta: cash(-1) })
            .receipt
            .controls
            .is_some()
    );
    assert_eq!(
        control(
            &mut s,
            52,
            ProtectionChange::Commit {
                claim: request(40),
                amount: cash(20)
            }
        )
        .receipt
        .controls,
        None
    );
    assert_eq!(
        s.state()
            .unwrap()
            .risk_report()
            .unwrap()
            .current
            .free_capital,
        cash(30)
    );
    assert_eq!(
        control(
            &mut s,
            53,
            ProtectionChange::Absorb {
                allocations: vec![Allocation {
                    claim: request(40),
                    amount: cash(20)
                }]
            }
        )
        .receipt
        .controls,
        None
    );
    assert_eq!(
        s.state()
            .unwrap()
            .risk_report()
            .unwrap()
            .current
            .free_capital,
        cash(30)
    );
    assert!(!s.state().unwrap().risk_report().unwrap().admissible);
}

#[test]
fn unresolved_customer_commitments_prevent_absorbing_a_transient_flat_deficit() {
    let t = Temp::new();
    let mut s = store(&t, 100);
    join_risk(&mut s, 0);
    assert_eq!(
        run(&mut s, vec![], vec![reserve(20, 1)]).receipt.controls,
        None
    );
    actual_loss(&mut s, 1, 100);
    run(&mut s, vec![recognize(1, 30, 20, Kind::Deficit)], vec![]);
    assert_eq!(
        control(
            &mut s,
            31,
            ProtectionChange::Commit {
                claim: request(30),
                amount: cash(20)
            }
        )
        .receipt
        .controls,
        Some(ControlError::Unqualified)
    );
    assert_eq!(
        run(&mut s, vec![], vec![Control::Release(request(20))])
            .receipt
            .controls,
        None
    );
    assert_eq!(
        control(
            &mut s,
            32,
            ProtectionChange::Commit {
                claim: request(30),
                amount: cash(20)
            }
        )
        .receipt
        .controls,
        None
    );
}

#[test]
fn opposing_accounts_can_extract_house_capital_despite_perfect_aggregate_bridge() {
    let t = Temp::new();
    let mut s = store(&t, 20);
    let mut events = vec![];
    for (id, open, close) in [(1, 2, -2), (2, -2, 2)] {
        for (j, quantity, price) in [(0, open, 100), (1, close, 40)] {
            let mut a = attempt(id * 10 + j);
            a.request.account = user(id);
            events.push(Event {
                key: RecordKey::Attempt(a),
                policy: config().policy,
                change: Change::BindExecution {
                    market: q(0).unit(),
                    side: if quantity > 0 { Side::Buy } else { Side::Sell },
                },
            });
            events.push(event(
                100 + u64::from(id * 10 + j),
                Change::Fill {
                    target: FillTarget::Customer(a),
                    quantity: q(quantity),
                    price: p(price),
                },
            ));
        }
    }
    run(&mut s, events, vec![]);
    let l = s.state().unwrap().ledger();
    l.check_bridge().unwrap();
    assert_eq!(account(l, 1), -20);
    assert_eq!(account(l, 2), 220);
    assert_eq!(l.venue().cash(), cash(300));
    run(&mut s, vec![recognize(1, 30, 20, Kind::Deficit)], vec![]);
    assert_eq!(
        earmark_absorb(&mut s, 31, request(30), 20).receipt.controls,
        None
    );
    let l = s.state().unwrap().ledger();
    l.check_bridge().unwrap();
    assert_eq!(account(l, 1) + account(l, 2), 220); // originally combined 200
    assert_eq!(l.book(Owner::House).unwrap().cash(), cash(80));
    assert_eq!(l.protection().absorbed_total, cash(20));
    // The loss is real even though external net position ends flat. The global
    // lifetime cap is a loss bound, NOT a claim that this attack is impossible.
}

fn put(s: &Ledger, e: Event) -> Ledger {
    assert_eq!(
        wire::decode_event(&wire::encode_event(&e).unwrap()).unwrap(),
        e
    );
    let next = s.apply(&e).unwrap();
    next.check_bridge().unwrap();
    next
}
fn request_for(id: u8, n: u8) -> RequestKey {
    let mut k = request(n);
    k.account = user(id);
    k
}
fn change(id: u8, n: u8, c: ProtectionChange) -> Event {
    Event {
        key: RecordKey::Request(request_for(id, n)),
        policy: config().policy,
        change: Change::Protection(c),
    }
}
fn recognize(id: u8, n: u8, amount: i128, kind: Kind) -> Event {
    change(
        id,
        n,
        ProtectionChange::Recognize {
            kind,
            cause: [n; 32],
            amount: cash(amount),
        },
    )
}
fn commit_claim(n: u8, claim: RequestKey, amount: i128) -> Event {
    change(
        1,
        n,
        ProtectionChange::Commit {
            claim,
            amount: cash(amount),
        },
    )
}
fn absorb(n: u8, rows: &[(RequestKey, i128)]) -> Event {
    change(
        1,
        n,
        ProtectionChange::Absorb {
            allocations: rows
                .iter()
                .map(|(claim, amount)| Allocation {
                    claim: *claim,
                    amount: cash(*amount),
                })
                .collect(),
        },
    )
}
fn setup(capital: i128) -> Ledger {
    let mut s = Ledger::new(config()).unwrap();
    for e in [
        receipt(1, Owner::Customer(user(1)), 100),
        receipt(2, Owner::Customer(user(2)), 100),
        receipt(3, Owner::House, capital),
    ] {
        s = put(&s, e);
    }
    for (n, side) in [(1, Side::Buy), (2, Side::Sell)] {
        s = put(
            &s,
            Event {
                key: RecordKey::Attempt(attempt(n)),
                policy: config().policy,
                change: Change::BindExecution {
                    market: q(0).unit(),
                    side,
                },
            },
        );
    }
    put(
        &s,
        change(
            1,
            10,
            ProtectionChange::Designate {
                delta: cash(capital),
            },
        ),
    )
}
fn loss(s: &Ledger, n: u64, quantity: i64, close: u64) -> Ledger {
    let s = put(
        s,
        event(
            n,
            Change::Fill {
                target: FillTarget::Customer(attempt(1)),
                quantity: q(quantity),
                price: p(100),
            },
        ),
    );
    put(
        &s,
        event(
            n + 1,
            Change::Fill {
                target: FillTarget::Customer(attempt(2)),
                quantity: q(-quantity),
                price: p(close),
            },
        ),
    )
}
fn backing(s: &Ledger) -> i128 {
    s.diagnostics(&[p(100)]).unwrap().backing_margin.atoms()
}
fn free(s: &Ledger) -> i128 {
    s.protection()
        .reserve
        .checked_sub(s.protection().committed().unwrap())
        .unwrap()
        .atoms()
        .min(backing(s))
}
fn account(s: &Ledger, id: u8) -> i128 {
    s.book(Owner::Customer(user(id))).unwrap().cash().atoms()
}

#[test]
fn v06_recognition_absorption_payment_and_recovery_do_not_charge_twice() {
    let mut s = loss(&setup(100), 20, 2, 40);
    assert_eq!(account(&s, 1), -20);
    assert_eq!(backing(&s), 80);
    s = put(&s, recognize(1, 30, 20, Kind::Deficit));
    assert_eq!(s.protection().unfunded().unwrap(), cash(20));
    s = put(&s, commit_claim(31, request(30), 20));
    assert_eq!(free(&s), 80);
    s = put(&s, absorb(32, &[(request(30), 20)]));
    assert_eq!((account(&s, 1), backing(&s), free(&s)), (0, 80, 80));
    assert_eq!(s.protection().collection(user(1)).unwrap(), cash(20));
    let mut a = attempt(40);
    a.request.account = user(2);
    s = put(
        &s,
        Event {
            key: RecordKey::Attempt(a),
            policy: config().policy,
            change: Change::Funds(FundsChange::Authorize(Mandate {
                attempt: a,
                source: Location::Venue,
                destination: Destination::Recipient([42; 32]),
                net: cash(100),
                maximum_fee: cash(0),
                fee_payer: Owner::House,
            })),
        },
    );
    for (n, leg) in [
        (40, Leg::Debit),
        (41, Leg::Arrive(Destination::Recipient([42; 32]))),
    ] {
        s = put(
            &s,
            event(
                n,
                Change::Funds(FundsChange::Observe {
                    attempt: a,
                    leg,
                    amount: cash(100),
                    fee: cash(0),
                }),
            ),
        );
    }
    assert_eq!(s.paid(user(2)).unwrap(), cash(100));
    assert_eq!(s.protection().reserve, cash(80));
    assert_eq!(free(&s), 80);
    s = put(&s, receipt(50, Owner::Customer(user(1)), 25));
    assert_eq!(
        (account(&s, 1), s.protection().reserve.atoms(), free(&s)),
        (5, 100, 100)
    );
    assert_eq!(s.protection().collection(user(1)).unwrap(), cash(0));
    assert_eq!(s.protection().absorbed_total, cash(20));
}
#[test]
fn underfunded_claims_and_partial_absorption_preserve_unpaid_remainder() {
    let mut s = loss(&setup(20), 20, 3, 50);
    assert_eq!(account(&s, 1), -50);
    s = put(&s, recognize(1, 30, 50, Kind::Deficit));
    assert!(s.apply(&commit_claim(31, request(30), 21)).is_err());
    s = put(&s, commit_claim(31, request(30), 20));
    s = put(&s, absorb(32, &[(request(30), 20)]));
    assert_eq!((account(&s, 1), backing(&s), free(&s)), (-30, -30, -30));
    assert_eq!(s.protection().claims[0].remaining().unwrap(), cash(30));
    s = put(&s, receipt(50, Owner::Customer(user(1)), 60));
    assert_eq!(account(&s, 1), 10);
    assert_eq!(s.protection().reserve, cash(20));
    assert_eq!(s.protection().collection(user(1)).unwrap(), cash(0));
    assert_eq!(s.protection().unfunded().unwrap(), cash(0));
}
#[test]
fn recovery_before_absorption_reduces_earmarks_without_another_loss() {
    let mut s = loss(&setup(100), 20, 2, 40);
    s = put(&s, recognize(1, 30, 20, Kind::Deficit));
    s = put(&s, commit_claim(31, request(30), 20));
    s = put(&s, receipt(50, Owner::Customer(user(1)), 20));
    assert_eq!(s.protection().committed().unwrap(), cash(0));
    assert_eq!(s.protection().reserve, cash(100));
    assert!(s.apply(&absorb(32, &[(request(30), 1)])).is_err());
    assert_eq!(s.protection().absorbed_total, cash(0));
}
#[test]
fn repeated_deficit_episodes_keep_distinct_collection_and_replenishment_history() {
    let mut s = loss(&setup(100), 20, 2, 40);
    s = put(&s, recognize(1, 30, 20, Kind::Deficit));
    s = put(&s, commit_claim(31, request(30), 20));
    s = put(&s, absorb(32, &[(request(30), 20)]));
    s = loss(&s, 60, 1, 90);
    assert_eq!(account(&s, 1), -10);
    s = put(&s, recognize(1, 33, 10, Kind::Deficit));
    s = put(&s, commit_claim(34, request(33), 10));
    s = put(&s, absorb(35, &[(request(33), 10)]));
    assert_eq!(s.protection().collection(user(1)).unwrap(), cash(30));
    assert_eq!(s.protection().reserve, cash(70));
    s = put(&s, receipt(70, Owner::Customer(user(1)), 25));
    assert_eq!(account(&s, 1), 0);
    assert_eq!(s.protection().reserve, cash(95));
    assert_eq!(s.protection().claims[0].replenished, cash(20));
    assert_eq!(s.protection().claims[1].replenished, cash(5));
    s = put(&s, receipt(71, Owner::Customer(user(1)), 10));
    assert_eq!(account(&s, 1), 5);
    assert_eq!(s.protection().reserve, cash(100));
    assert_eq!(s.protection().absorbed_total, cash(30));
}
#[test]
fn remediation_is_new_obligation_but_offsets_overlapping_unabsorbed_debt() {
    let mut s = loss(&setup(100), 20, 2, 40);
    s = put(&s, recognize(1, 30, 20, Kind::Deficit));
    s = put(&s, commit_claim(31, request(30), 20));
    s = put(&s, recognize(1, 33, 5, Kind::Remediation));
    assert_eq!(
        (account(&s, 1), s.protection().reserve.atoms(), free(&s)),
        (-15, 95, 80)
    );
    assert_eq!(s.protection().claims[0].offset, cash(5));
    assert_eq!(s.protection().committed().unwrap(), cash(15));
    s = put(&s, absorb(34, &[(request(30), 15)]));
    assert_eq!(s.protection().collection(user(1)).unwrap(), cash(15));
    assert_eq!(free(&s), 80);
    let actual = recognize(2, 35, 200, Kind::Remediation);
    s = put(&s, actual.clone());
    assert_eq!(account(&s, 2), 300);
    assert_eq!(backing(&s), -120);
    assert_eq!(s.protection().reserve, cash(-120));
    assert_eq!(put(&s, actual), s);
}
#[test]
fn recognition_cannot_duplicate_same_cause_or_relabel_old_remaining_deficit() {
    let s = loss(&setup(100), 20, 2, 40);
    let s = put(&s, recognize(1, 30, 20, Kind::Deficit));
    assert!(s.apply(&recognize(1, 31, 1, Kind::Deficit)).is_err());
    let mut duplicate = recognize(1, 32, 20, Kind::Deficit);
    if let Change::Protection(ProtectionChange::Recognize { cause, .. }) = &mut duplicate.change {
        *cause = [30; 32];
    }
    assert!(s.apply(&duplicate).is_err());
    assert!(
        s.apply(&change(
            1,
            33,
            ProtectionChange::Designate { delta: cash(-1) }
        ))
        .is_err()
    );
}
#[test]
fn actual_receipt_in_reopened_debt_episode_is_retained_but_not_invented_repayment() {
    let mut s = loss(&setup(100), 20, 2, 40);
    s = put(&s, recognize(1, 30, 20, Kind::Deficit));
    s = put(
        &s,
        event(
            60,
            Change::Fill {
                target: FillTarget::Customer(attempt(1)),
                quantity: q(1),
                price: p(100),
            },
        ),
    );
    assert!(s.unresolved_protection().unwrap());
    let before = s.venue().cash();
    s = put(&s, receipt(61, Owner::Customer(user(1)), 30));
    assert_eq!(s.venue().cash(), before.checked_add(cash(30)).unwrap());
    assert_eq!(account(&s, 1), 10);
    assert_eq!(s.protection().claims[0].recovered, cash(0));
    assert_eq!(s.protection().receipt_faults.len(), 1);
    assert!(s.apply(&commit_claim(33, request(30), 20)).is_err());
}
