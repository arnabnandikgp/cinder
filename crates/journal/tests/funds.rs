//! Qualified synthetic funding rails; real custody transport remains disabled.
mod support;
use cinder_journal::{collateral, funds::*, model::*, orders, sqlite::*, wire, *};
use cinder_kernel::{
    identity::*,
    ledger::{
        evidence::{Disposition, EvidencePolicy, MarkObservation, NativeCheck},
        funds::*,
        *,
    },
};
use support::{FixtureProtection, Store, Temp, attempt, cash, p, q, raw, request, user};

fn config() -> Config {
    let mut c = support::config();
    let mut vault = c.sources[0];
    vault.scope.namespace = NamespaceId::new([8; 32]).unwrap();
    vault.location = Location::Vault;
    c.sources.push(vault);
    c
}
fn key(n: u64, location: Location) -> EventKey {
    EventKey {
        scope: config()
            .sources
            .into_iter()
            .find(|s| s.location == location)
            .unwrap()
            .scope,
        event: EconomicEventId::new(&n.to_be_bytes()).unwrap(),
        leg: 0,
    }
}
fn event(n: u64, location: Location, change: Change) -> Event {
    Event {
        key: RecordKey::Economic(key(n, location)),
        policy: config().policy,
        change,
    }
}
fn commit(s: &mut Store, n: u8, events: Vec<Event>, controls: Vec<Control>) -> Committed {
    let mut tx = support::transaction(s.head(), n, events, controls);
    for input in &mut tx.inputs {
        if let Some(Event {
            key: RecordKey::Economic(k),
            ..
        }) = &input.event
        {
            input.source = k.scope;
        }
    }
    assert_eq!(
        wire::decode_transaction(&wire::encode_transaction(&tx).unwrap()).unwrap(),
        tx
    );
    s.commit(tx).unwrap()
}
fn open(t: &Temp) -> Store {
    Journal::open(
        SqliteBackend::open(&t.db, Migration::None).unwrap(),
        FixtureProtection,
        config(),
    )
    .unwrap()
}
fn setup(t: &Temp, position: bool) -> Store {
    let mut s = Journal::create(
        SqliteBackend::create(&t.db).unwrap(),
        FixtureProtection,
        config(),
    )
    .unwrap();
    let mut events = vec![
        event(
            1,
            Location::Vault,
            Change::Receipt {
                owner: Owner::Customer(user(1)),
                location: Location::Vault,
                amount: cash(100),
            },
        ),
        event(
            2,
            if position {
                Location::Venue
            } else {
                Location::Vault
            },
            Change::Receipt {
                owner: Owner::Customer(user(2)),
                location: if position {
                    Location::Venue
                } else {
                    Location::Vault
                },
                amount: cash(100),
            },
        ),
        event(
            3,
            Location::Vault,
            Change::Receipt {
                owner: Owner::House,
                location: Location::Vault,
                amount: cash(100),
            },
        ),
    ];
    if position {
        events.push(Event {
            key: RecordKey::Attempt(attempt(99)),
            policy: config().policy,
            change: Change::BindExecution {
                market: q(0).unit(),
                side: Side::Buy,
            },
        });
        events.push(event(
            4,
            Location::Venue,
            Change::Fill {
                target: FillTarget::Customer(attempt(99)),
                quantity: q(1),
                price: p(100),
            },
        ));
    }
    let result = commit(
        &mut s,
        1,
        events,
        vec![
            Control::Order(orders::Action::AdvanceAuthority {
                account: user(1),
                epoch: 1,
            }),
            Control::Order(orders::Action::AdvanceAuthority {
                account: user(2),
                epoch: 1,
            }),
        ],
    );
    assert_eq!(result.receipt.controls, None);
    assert!(
        result
            .receipt
            .inputs
            .iter()
            .all(|r| *r == InputResult::Normalized(Disposition::Applied))
    );
    let l = s.state().unwrap().ledger();
    let version = l.version();
    let check = event(
        5,
        Location::Venue,
        Change::Reconcile(NativeCheck {
            expected_version: version,
            cash: Some(l.venue().cash()),
            funding: Some(l.venue().funding()),
            positions: Some(l.venue().positions().to_vec()),
            complete: true,
            resolves: vec![],
        }),
    );
    let c = collateral::Cut {
        expected_version: version + 1,
        policy: collateral::Policy {
            revision: PolicyVersion::new(1).unwrap(),
            markets: vec![collateral::MarginRule {
                market: q(0).unit(),
                private_bps: 1000,
                native_bps: 1000,
            }],
            evidence: EvidencePolicy {
                max_issue_age: 100,
                max_mark_age: 100,
                max_check_age: 100,
            },
        },
        marks: vec![MarkObservation {
            price: p(100),
            evidence: key(6, Location::Venue),
            observed_at: 10,
            valid_until: 100,
            qualified: true,
        }],
    };
    assert_eq!(
        commit(&mut s, 2, vec![check], vec![Control::Collateral(c)])
            .receipt
            .controls,
        None
    );
    s
}
fn intent(n: u8, payout: bool) -> Intent {
    Intent {
        request: request(n),
        source: Location::Vault,
        destination: if payout {
            Destination::Recipient([42; 32])
        } else {
            Destination::Location(Location::Venue)
        },
        net: cash(80),
        maximum_fee: cash(5),
        fee_payer: Owner::House,
        allow_partial: true,
        policy: config().policy,
        authority_epoch: 1,
        expires_at: 100,
    }
}
fn accept(i: Intent) -> Control {
    let approval = orders::Approval {
        account: i.request.account,
        authority_epoch: i.authority_epoch,
        intent_hash: i.digest().unwrap(),
    };
    Control::Funds(Action::Accept {
        intent: Box::new(i),
        approval,
    })
}
fn prepare(n: u8, amount: i128) -> Control {
    Control::Funds(Action::Prepare {
        attempt: attempt(n),
        net: cash(amount),
    })
}
fn start(s: &mut Store, payout: bool) {
    assert_eq!(
        commit(
            s,
            3,
            vec![],
            vec![
                accept(intent(3, payout)),
                prepare(3, 80),
                Control::Expose(attempt(3))
            ]
        )
        .receipt
        .controls,
        None
    );
}
fn leg(n: u64, l: Leg, amount: i128, fee: i128) -> Event {
    let source = if let Leg::Arrive(Destination::Location(location)) = l {
        location
    } else {
        Location::Vault
    };
    event(
        n,
        source,
        Change::Funds(FundsChange::Observe {
            attempt: attempt(3),
            leg: l,
            amount: cash(amount),
            fee: cash(fee),
        }),
    )
}
fn finish(
    s: &mut Store,
    n: u8,
    amount: i128,
    receipts: Vec<EventKey>,
    finalize: bool,
) -> Committed {
    let mut tx = support::transaction(
        s.head(),
        n,
        vec![],
        if finalize {
            vec![Control::Funds(Action::Finalize(request(3)))]
        } else {
            vec![]
        },
    );
    tx.funds_observations = vec![Observation {
        key: key(50, Location::Vault),
        terminal: Terminal {
            attempt: attempt(3),
            debit: cash(amount),
            settled: cash(amount),
            receipts,
            coverage: config()
                .sources
                .into_iter()
                .map(|s| Coverage {
                    source: s.scope,
                    through: 10,
                })
                .collect(),
            no_later_execution: [1; 32],
        },
        authority_epoch: 1,
        observed_at: 10,
        raw: raw(b"synthetic qualified closed capability and complete history"),
    }];
    assert_eq!(
        wire::decode_transaction(&wire::encode_transaction(&tx).unwrap()).unwrap(),
        tx
    );
    s.commit(tx).unwrap()
}
fn claim(s: &Store) -> i128 {
    s.state()
        .unwrap()
        .ledger()
        .book(Owner::Customer(user(1)))
        .unwrap()
        .cash()
        .atoms()
}

#[test]
fn faulted_unexposed_payout_cannot_release_its_hold_by_cancellation() {
    for same_transaction in [false, true] {
        let t = Temp::new();
        let mut s = setup(&t, false);
        assert_eq!(
            commit(
                &mut s,
                3,
                vec![],
                vec![accept(intent(3, true)), prepare(3, 80)]
            )
            .receipt
            .controls,
            None
        );
        let receipt = leg(10, Leg::Debit, 40, 0);
        if !same_transaction {
            assert_eq!(
                commit(&mut s, 4, vec![receipt.clone()], vec![])
                    .receipt
                    .inputs,
                [InputResult::Normalized(Disposition::Applied)]
            );
        }
        let cancelled = commit(
            &mut s,
            5,
            if same_transaction {
                vec![receipt]
            } else {
                vec![]
            },
            vec![Control::Funds(Action::CancelUnexposed(request(3)))],
        );
        assert_eq!(cancelled.receipt.controls, Some(ControlError::Unqualified));
        assert!(s.state().unwrap().funds()[0].faulted);
        assert!(!s.state().unwrap().funds()[0].terminal);
        assert!(
            s.state()
                .unwrap()
                .holds()
                .iter()
                .find(|h| h.request == request(3))
                .unwrap()
                .active
        );
        assert_eq!(s.state().unwrap().ledger().in_transit().unwrap(), cash(40));
        let tx = s
            .transaction(CommitId::new([5; 32]).unwrap())
            .unwrap()
            .clone();
        assert!(s.commit(tx).unwrap().duplicate);
        let state = s.state().unwrap().clone();
        drop(s);
        let mut reopened = open(&t);
        assert_eq!(reopened.state().unwrap(), &state);
        commit(&mut reopened, 6, vec![], vec![]);
        assert!(
            reopened
                .state()
                .unwrap()
                .holds()
                .iter()
                .find(|h| h.request == request(3))
                .unwrap()
                .active
        );
    }
    let t = Temp::new();
    let mut clean = setup(&t, false);
    commit(
        &mut clean,
        3,
        vec![],
        vec![accept(intent(3, true)), prepare(3, 80)],
    );
    assert_eq!(
        commit(
            &mut clean,
            4,
            vec![],
            vec![Control::Funds(Action::CancelUnexposed(request(3)))]
        )
        .receipt
        .controls,
        None
    );
    assert!(clean.state().unwrap().funds()[0].terminal);
    assert!(
        !clean
            .state()
            .unwrap()
            .holds()
            .iter()
            .find(|h| h.request == request(3))
            .unwrap()
            .active
    );
}

#[test]
fn partial_transfer_moves_existing_assets_once_and_never_mints_another_customer_credit() {
    let t = Temp::new();
    let mut s = setup(&t, false);
    start(&mut s, false);
    assert_eq!(
        s.state()
            .unwrap()
            .reserved(Resource::Location(Location::Vault))
            .unwrap(),
        cash(85)
    );
    assert_eq!(s.state().unwrap().ledger().vault(), cash(300));
    commit(&mut s, 4, vec![leg(10, Leg::Debit, 40, 0)], vec![]);
    assert_eq!(s.state().unwrap().ledger().in_transit().unwrap(), cash(40));
    assert_eq!(
        s.state()
            .unwrap()
            .reserved(Resource::Location(Location::Vault))
            .unwrap(),
        cash(45)
    );
    commit(
        &mut s,
        5,
        vec![leg(
            11,
            Leg::Arrive(Destination::Location(Location::Venue)),
            40,
            2,
        )],
        vec![],
    );
    assert_eq!(claim(&s), 100);
    assert_eq!(s.state().unwrap().ledger().venue().cash(), cash(38));
    assert_eq!(
        s.state()
            .unwrap()
            .ledger()
            .book(Owner::House)
            .unwrap()
            .cash(),
        cash(98)
    );
    commit(
        &mut s,
        6,
        vec![
            leg(12, Leg::Debit, 42, 0),
            leg(
                13,
                Leg::Arrive(Destination::Location(Location::Venue)),
                42,
                0,
            ),
        ],
        vec![],
    );
    assert_eq!(
        finish(
            &mut s,
            7,
            82,
            vec![
                key(10, Location::Vault),
                key(11, Location::Venue),
                key(12, Location::Vault),
                key(13, Location::Venue)
            ],
            true
        )
        .receipt
        .controls,
        None
    );
    let state = s.state().unwrap().clone();
    assert_eq!(state.ledger().vault(), cash(218));
    assert_eq!(state.ledger().venue().cash(), cash(80));
    assert_eq!(claim(&s), 100);
    assert_eq!(state.ledger().in_transit().unwrap(), cash(0));
    drop(s);
    assert_eq!(&state, open(&t).state().unwrap());
}

#[test]
fn arrival_before_debit_is_contra_backing_and_blocks_risk_until_reconciled() {
    let t = Temp::new();
    let mut s = setup(&t, false);
    start(&mut s, false);
    commit(
        &mut s,
        4,
        vec![leg(
            11,
            Leg::Arrive(Destination::Location(Location::Venue)),
            82,
            2,
        )],
        vec![],
    );
    let l = s.state().unwrap().ledger();
    assert_eq!(l.unpaired().unwrap(), cash(82));
    assert_eq!(l.diagnostics(&[p(100)]).unwrap().net_assets, cash(298));
    assert!(
        commit(&mut s, 5, vec![], vec![accept(intent(5, true))])
            .receipt
            .controls
            .is_some()
    );
    commit(&mut s, 6, vec![leg(10, Leg::Debit, 82, 0)], vec![]);
    let l = s.state().unwrap().ledger();
    assert_eq!(l.unpaired().unwrap(), cash(0));
    assert_eq!(l.diagnostics(&[p(100)]).unwrap().net_assets, cash(298));
    l.check_bridge().unwrap();
}

#[test]
fn beneficiary_receipt_not_source_debit_discharges_claim_and_paid_counter() {
    let t = Temp::new();
    let mut s = setup(&t, false);
    start(&mut s, true);
    commit(&mut s, 4, vec![leg(10, Leg::Debit, 40, 0)], vec![]);
    assert_eq!(claim(&s), 100);
    assert_eq!(s.state().unwrap().ledger().paid(user(1)).unwrap(), cash(0));
    let e = leg(11, Leg::Arrive(Destination::Recipient([42; 32])), 40, 2);
    commit(&mut s, 5, vec![e.clone()], vec![]);
    assert_eq!(claim(&s), 62);
    assert_eq!(s.state().unwrap().ledger().paid(user(1)).unwrap(), cash(38));
    assert_eq!(
        s.state()
            .unwrap()
            .reserved(Resource::Customer(user(1)))
            .unwrap(),
        cash(42)
    );
    assert_eq!(
        commit(&mut s, 6, vec![e], vec![]).receipt.inputs,
        vec![InputResult::Normalized(Disposition::Duplicate)]
    );
    assert_eq!(claim(&s), 62);
    assert_eq!(
        finish(
            &mut s,
            7,
            40,
            vec![key(10, Location::Vault), key(11, Location::Vault)],
            true
        )
        .receipt
        .controls,
        None
    );
    assert_eq!(
        s.state()
            .unwrap()
            .reserved(Resource::Customer(user(1)))
            .unwrap(),
        cash(0)
    );
    assert_eq!(claim(&s), 62);
}

#[test]
fn returns_impairment_overfees_and_wrong_recipient_preserve_customer_ownership() {
    for variant in 0..4 {
        let t = Temp::new();
        let mut s = setup(&t, false);
        start(&mut s, true);
        let (which, gross, fee) = match variant {
            0 => (Leg::Return, 80, 2),
            1 => (Leg::Impair, 80, 0),
            2 => (Leg::Arrive(Destination::Recipient([42; 32])), 85, 10),
            _ => (Leg::Arrive(Destination::Recipient([99; 32])), 80, 0),
        };
        commit(
            &mut s,
            4,
            vec![leg(10, Leg::Debit, gross, 0), leg(11, which, gross, fee)],
            vec![],
        );
        let l = s.state().unwrap().ledger();
        l.check_bridge().unwrap();
        assert_eq!(claim(&s), if variant == 2 { 25 } else { 100 });
        assert_eq!(
            l.book(Owner::House).unwrap().cash(),
            cash(match variant {
                0 => 98,
                1 => 20,
                2 => 90,
                _ => 20,
            })
        );
        assert_eq!(
            l.paid(user(1)).unwrap(),
            cash(if variant == 2 { 75 } else { 0 })
        );
        if variant >= 2 {
            assert!(s.state().unwrap().funds()[0].faulted);
        }
        let state = s.state().unwrap().clone();
        drop(s);
        assert_eq!(&state, open(&t).state().unwrap());
    }
}

#[test]
fn overpayment_is_house_loss_not_collectible_customer_debt() {
    let t = Temp::new();
    let mut s = setup(&t, false);
    start(&mut s, true);
    commit(
        &mut s,
        4,
        vec![
            leg(10, Leg::Debit, 90, 0),
            leg(11, Leg::Arrive(Destination::Recipient([42; 32])), 90, 0),
        ],
        vec![],
    );
    assert_eq!(claim(&s), 20);
    assert_eq!(s.state().unwrap().ledger().paid(user(1)).unwrap(), cash(80));
    assert_eq!(
        s.state()
            .unwrap()
            .ledger()
            .book(Owner::House)
            .unwrap()
            .cash(),
        cash(90)
    );
    assert!(s.state().unwrap().funds()[0].faulted);
    assert!(
        commit(&mut s, 5, vec![], vec![accept(intent(5, true))])
            .receipt
            .controls
            .is_some()
    );
}

#[test]
fn customer_fee_cap_is_explicit_and_overruns_are_not_silently_passed_through() {
    let t = Temp::new();
    let mut s = setup(&t, false);
    let mut i = intent(3, true);
    i.fee_payer = Owner::Customer(user(1));
    assert_eq!(
        commit(
            &mut s,
            3,
            vec![],
            vec![accept(i), prepare(3, 80), Control::Expose(attempt(3))]
        )
        .receipt
        .controls,
        None
    );
    assert_eq!(
        s.state()
            .unwrap()
            .reserved(Resource::Customer(user(1)))
            .unwrap(),
        cash(85)
    );
    commit(
        &mut s,
        4,
        vec![
            leg(10, Leg::Debit, 85, 0),
            leg(11, Leg::Arrive(Destination::Recipient([42; 32])), 85, 10),
        ],
        vec![],
    );
    assert_eq!(claim(&s), 20);
    assert_eq!(s.state().unwrap().ledger().paid(user(1)).unwrap(), cash(75));
    assert_eq!(
        s.state()
            .unwrap()
            .ledger()
            .book(Owner::House)
            .unwrap()
            .cash(),
        cash(95)
    );
    assert!(s.state().unwrap().funds()[0].faulted);
}

#[test]
fn frozen_state_and_unknown_outcome_cannot_erase_late_beneficiary_receipts() {
    let t = Temp::new();
    let mut s = setup(&t, false);
    start(&mut s, true);
    commit(&mut s, 4, vec![], vec![Control::Funds(Action::Freeze)]);
    assert!(
        commit(
            &mut s,
            5,
            vec![],
            vec![Control::Funds(Action::CancelUnexposed(request(3)))]
        )
        .receipt
        .controls
        .is_some()
    );
    assert!(
        commit(&mut s, 6, vec![], vec![accept(intent(6, true))])
            .receipt
            .controls
            .is_some()
    );
    let result = commit(
        &mut s,
        7,
        vec![
            leg(10, Leg::Debit, 80, 0),
            leg(11, Leg::Arrive(Destination::Recipient([42; 32])), 80, 0),
        ],
        vec![accept(intent(7, true))],
    );
    assert_eq!(
        result.receipt.inputs,
        vec![InputResult::Normalized(Disposition::Applied); 2]
    );
    assert_eq!(result.receipt.controls, Some(ControlError::Unqualified));
    assert_eq!(claim(&s), 20);
    assert!(s.state().unwrap().frozen());
    let state = s.state().unwrap().clone();
    drop(s);
    assert_eq!(&state, open(&t).state().unwrap());
}

#[test]
fn fifo_source_exclusivity_and_partial_dispatch_consent_are_durable_controls() {
    let t = Temp::new();
    let mut s = setup(&t, false);
    let mut first = intent(3, true);
    first.allow_partial = false;
    let mut second = intent(4, true);
    second.request.account = user(2);
    assert_eq!(
        commit(&mut s, 3, vec![], vec![accept(first), accept(second)])
            .receipt
            .controls,
        None
    );
    let second_attempt = AttemptKey {
        request: RequestKey {
            account: user(2),
            ..request(4)
        },
        ..attempt(4)
    };
    assert_eq!(
        commit(
            &mut s,
            4,
            vec![],
            vec![Control::Funds(Action::Prepare {
                attempt: second_attempt,
                net: cash(80)
            })]
        )
        .receipt
        .controls,
        Some(ControlError::Unqualified)
    );
    assert_eq!(
        commit(&mut s, 5, vec![], vec![prepare(3, 40)])
            .receipt
            .controls,
        Some(ControlError::Invalid)
    );
    assert_eq!(
        commit(
            &mut s,
            6,
            vec![],
            vec![prepare(3, 80), Control::Expose(attempt(3))]
        )
        .receipt
        .controls,
        None
    );
    assert!(
        commit(
            &mut s,
            7,
            vec![],
            vec![Control::Funds(Action::Prepare {
                attempt: second_attempt,
                net: cash(80)
            })]
        )
        .receipt
        .controls
        .is_some()
    );
    assert_eq!(finish(&mut s, 8, 0, vec![], true).receipt.controls, None);
    assert_eq!(
        commit(
            &mut s,
            9,
            vec![],
            vec![
                Control::Funds(Action::Prepare {
                    attempt: second_attempt,
                    net: cash(40)
                }),
                Control::Expose(second_attempt)
            ]
        )
        .receipt
        .controls,
        None
    );
    let state = s.state().unwrap().clone();
    drop(s);
    assert_eq!(&state, open(&t).state().unwrap());
}

#[test]
fn open_position_withdrawal_uses_marked_initial_margin_and_shared_holds() {
    let t = Temp::new();
    let mut s = setup(&t, true);
    let mut too_much = intent(3, true);
    too_much.net = cash(91);
    too_much.maximum_fee = cash(0);
    assert_eq!(
        commit(&mut s, 3, vec![], vec![accept(too_much)])
            .receipt
            .controls,
        Some(ControlError::Capacity)
    );
    let mut allowed = intent(4, true);
    allowed.net = cash(90);
    allowed.maximum_fee = cash(0);
    assert_eq!(
        commit(&mut s, 4, vec![], vec![accept(allowed)])
            .receipt
            .controls,
        None
    );
    let mut extra = intent(5, true);
    extra.net = cash(1);
    extra.maximum_fee = cash(0);
    assert_eq!(
        commit(&mut s, 5, vec![], vec![accept(extra)])
            .receipt
            .controls,
        Some(ControlError::Capacity)
    );
    assert_eq!(
        s.state()
            .unwrap()
            .ledger()
            .book(Owner::Customer(user(1)))
            .unwrap()
            .positions()[0]
            .quantity(),
        q(1)
    );
    assert_eq!(
        commit(
            &mut s,
            6,
            vec![],
            vec![prepare(4, 90), Control::Expose(attempt(4))]
        )
        .receipt
        .controls,
        None
    );
}

#[test]
fn terminal_before_history_waits_and_false_certificate_cannot_hide_later_payment() {
    let t = Temp::new();
    let mut s = setup(&t, false);
    start(&mut s, true);
    assert_eq!(
        finish(
            &mut s,
            4,
            80,
            vec![key(10, Location::Vault), key(11, Location::Vault)],
            true
        )
        .receipt
        .controls,
        Some(ControlError::Unqualified)
    );
    commit(
        &mut s,
        5,
        vec![leg(
            11,
            Leg::Arrive(Destination::Recipient([42; 32])),
            80,
            0,
        )],
        vec![],
    );
    assert!(s.state().unwrap().holds()[0].active);
    assert_eq!(
        commit(
            &mut s,
            6,
            vec![leg(10, Leg::Debit, 80, 0)],
            vec![Control::Funds(Action::Finalize(request(3)))]
        )
        .receipt
        .controls,
        None
    );
    commit(
        &mut s,
        7,
        vec![leg(12, Leg::Arrive(Destination::Recipient([42; 32])), 1, 0)],
        vec![],
    );
    assert!(s.state().unwrap().funds()[0].faulted);
    assert!(s.state().unwrap().holds()[0].active);
    assert_eq!(claim(&s), 20);
    assert_eq!(
        s.state()
            .unwrap()
            .ledger()
            .book(Owner::House)
            .unwrap()
            .cash(),
        cash(99)
    );
}

#[test]
fn property_all_24_transfer_receipt_orders_preserve_every_prefix_bridge() {
    let mut count = 0;
    for a in 0..4 {
        for b in 0..4 {
            for c in 0..4 {
                for d in 0..4 {
                    if a == b || a == c || a == d || b == c || b == d || c == d {
                        continue;
                    }
                    count += 1;
                    let t = Temp::new();
                    let mut s = setup(&t, false);
                    start(&mut s, false);
                    let events = [
                        leg(10, Leg::Debit, 40, 0),
                        leg(
                            11,
                            Leg::Arrive(Destination::Location(Location::Venue)),
                            40,
                            2,
                        ),
                        leg(12, Leg::Debit, 42, 0),
                        leg(
                            13,
                            Leg::Arrive(Destination::Location(Location::Venue)),
                            42,
                            0,
                        ),
                    ];
                    for (step, index) in [a, b, c, d].into_iter().enumerate() {
                        let n = u8::try_from(4 + step * 2).unwrap();
                        let e = events[index].clone();
                        assert_eq!(
                            commit(&mut s, n, vec![e.clone()], vec![]).receipt.inputs,
                            vec![InputResult::Normalized(Disposition::Applied)]
                        );
                        s.state().unwrap().ledger().check_bridge().unwrap();
                        assert_eq!(
                            commit(&mut s, n + 1, vec![e], vec![]).receipt.inputs,
                            vec![InputResult::Normalized(Disposition::Duplicate)]
                        );
                        drop(s);
                        s = open(&t);
                    }
                    assert_eq!(
                        finish(
                            &mut s,
                            20,
                            82,
                            vec![
                                key(10, Location::Vault),
                                key(11, Location::Venue),
                                key(12, Location::Vault),
                                key(13, Location::Venue)
                            ],
                            true
                        )
                        .receipt
                        .controls,
                        None
                    );
                    let l = s.state().unwrap().ledger();
                    assert_eq!(l.vault(), cash(218));
                    assert_eq!(l.venue().cash(), cash(80));
                    assert_eq!(l.in_transit().unwrap(), cash(0));
                    assert_eq!(l.unpaired().unwrap(), cash(0));
                    assert_eq!(claim(&s), 100);
                }
            }
        }
    }
    assert_eq!(count, 24);
}

#[test]
fn fill_between_prepare_and_exposure_rechecks_collateral_without_erasing_loss() {
    let t = Temp::new();
    let mut s = setup(&t, true);
    assert_eq!(
        commit(
            &mut s,
            3,
            vec![],
            vec![accept(intent(3, true)), prepare(3, 80)]
        )
        .receipt
        .controls,
        None
    );
    let bind = Event {
        key: RecordKey::Attempt(attempt(88)),
        policy: config().policy,
        change: Change::BindExecution {
            market: q(0).unit(),
            side: Side::Sell,
        },
    };
    let loss = event(
        20,
        Location::Venue,
        Change::Fill {
            target: FillTarget::Customer(attempt(88)),
            quantity: q(-1),
            price: p(30),
        },
    );
    let result = commit(
        &mut s,
        4,
        vec![bind, loss],
        vec![Control::Expose(attempt(3))],
    );
    assert_eq!(
        result.receipt.inputs,
        vec![InputResult::Normalized(Disposition::Applied); 2]
    );
    assert_eq!(result.receipt.controls, Some(ControlError::Capacity));
    assert!(result.exposures.is_empty());
    assert_eq!(claim(&s), 30);
    assert!(!s.state().unwrap().attempts()[0].possibly_exposed);
    assert_eq!(
        commit(
            &mut s,
            5,
            vec![],
            vec![Control::Funds(Action::CancelUnexposed(request(3)))]
        )
        .receipt
        .controls,
        None
    );
    assert_eq!(claim(&s), 30);
    assert_eq!(s.state().unwrap().ledger().paid(user(1)).unwrap(), cash(0));
}

#[test]
fn queued_claim_can_be_backed_but_lack_source_liquidity_and_stale_prices_cannot_authorize_it() {
    let t = Temp::new();
    let mut s = setup(&t, false);
    // Entirely vault-funded fixture; ask for a venue payout with no native cash.
    let mut i = intent(3, true);
    i.source = Location::Venue;
    assert_eq!(
        commit(&mut s, 3, vec![], vec![accept(i)]).receipt.controls,
        None
    );
    assert_eq!(
        commit(
            &mut s,
            4,
            vec![],
            vec![prepare(3, 80), Control::Expose(attempt(3))]
        )
        .receipt
        .controls,
        Some(ControlError::Capacity)
    );
    assert!(s.state().unwrap().attempts().is_empty());
    assert_eq!(
        s.state()
            .unwrap()
            .ledger()
            .diagnostics(&[p(100)])
            .unwrap()
            .shortfall,
        cash(0)
    );
    let mut tx = support::transaction(s.head(), 5, vec![], vec![accept(intent(5, true))]);
    tx.at = 101;
    assert_eq!(
        s.commit(tx).unwrap().receipt.controls,
        Some(ControlError::Unqualified)
    );
    assert_eq!(claim(&s), 100);
}

#[test]
fn changed_intent_bad_auth_and_wrong_receipt_scope_are_retained_without_payment() {
    for variant in 0..5 {
        let t = Temp::new();
        let mut s = setup(&t, false);
        let mut action = accept(intent(3, true));
        if let Control::Funds(Action::Accept { intent, approval }) = &mut action {
            match variant {
                0 => approval.account = user(2),
                1 => approval.intent_hash[0] ^= 1,
                2 => approval.authority_epoch = 2,
                3 => {
                    intent.net = cash(-1);
                    approval.intent_hash = intent.digest().unwrap();
                }
                _ => {
                    intent.expires_at = 10;
                    approval.intent_hash = intent.digest().unwrap();
                }
            }
        }
        assert!(
            commit(&mut s, 3, vec![], vec![action])
                .receipt
                .controls
                .is_some()
        );
        assert!(s.state().unwrap().funds().is_empty());
        assert!(s.state().unwrap().holds().is_empty());
    }
    let t = Temp::new();
    let mut s = setup(&t, false);
    start(&mut s, true);
    let mut changed = intent(3, true);
    changed.net = cash(81);
    assert_eq!(
        commit(&mut s, 4, vec![], vec![accept(changed)])
            .receipt
            .controls,
        Some(ControlError::Invalid)
    );
    let mut e = leg(10, Leg::Debit, 80, 0);
    e.key = RecordKey::Economic(key(10, Location::Venue));
    assert!(matches!(
        commit(&mut s, 5, vec![e], vec![]).receipt.inputs[0],
        InputResult::Normalized(Disposition::Rejected(LedgerError::Attribution))
    ));
    assert_eq!(s.state().unwrap().ledger().vault(), cash(300));
    assert_eq!(claim(&s), 100);
}

#[test]
fn unknown_funds_commit_recovers_exactly_once_without_another_exposure() {
    struct LostReply(SqliteBackend);
    impl Backend for LostReply {
        fn load(&mut self) -> Result<Vec<Frame>, Error> {
            self.0.load()
        }
        fn append(&mut self, expected: Option<Head>, f: &Frame) -> Result<(), Error> {
            self.0.append(expected, f)?;
            Err(Error::Storage)
        }
    }
    let t = Temp::new();
    let s = setup(&t, false);
    let head = s.head();
    drop(s);
    let mut s = Journal::open(
        LostReply(SqliteBackend::open(&t.db, Migration::None).unwrap()),
        FixtureProtection,
        config(),
    )
    .unwrap();
    let tx = support::transaction(
        head,
        3,
        vec![],
        vec![
            accept(intent(3, true)),
            prepare(3, 80),
            Control::Expose(attempt(3)),
        ],
    );
    assert_eq!(s.commit(tx.clone()).unwrap_err(), Error::Storage);
    assert_eq!(s.state().unwrap_err(), Error::Poisoned);
    drop(s);
    let mut s = open(&t);
    let recovered = s.commit(tx).unwrap();
    assert!(recovered.duplicate);
    assert!(recovered.exposures.is_empty());
    assert!(s.state().unwrap().attempts()[0].possibly_exposed);
    assert!(s.state().unwrap().holds()[0].active);
    assert_eq!(claim(&s), 100);
}

#[test]
fn missing_no_later_proof_or_cross_source_coverage_cannot_release_commitments() {
    for variant in 0..3 {
        let t = Temp::new();
        let mut s = setup(&t, false);
        start(&mut s, true);
        commit(
            &mut s,
            4,
            vec![
                leg(10, Leg::Debit, 80, 0),
                leg(11, Leg::Arrive(Destination::Recipient([42; 32])), 80, 0),
            ],
            vec![],
        );
        let mut tx = support::transaction(
            s.head(),
            5,
            vec![],
            vec![Control::Funds(Action::Finalize(request(3)))],
        );
        let mut terminal = Terminal {
            attempt: attempt(3),
            debit: cash(80),
            settled: cash(80),
            receipts: vec![key(10, Location::Vault), key(11, Location::Vault)],
            coverage: vec![Coverage {
                source: key(1, Location::Vault).scope,
                through: 10,
            }],
            no_later_execution: [1; 32],
        };
        match variant {
            0 => terminal.no_later_execution = [0; 32],
            1 => terminal.coverage[0].source = key(1, Location::Venue).scope,
            _ => terminal.receipts.push(key(10, Location::Vault)),
        }
        tx.funds_observations = vec![Observation {
            key: key(50, Location::Vault),
            terminal,
            authority_epoch: 1,
            observed_at: 10,
            raw: raw(b"bad synthetic certificate"),
        }];
        let result = s.commit(tx).unwrap();
        assert_eq!(result.receipt.funds_observations, vec![false]);
        assert_eq!(result.receipt.controls, Some(ControlError::Unqualified));
        assert!(s.state().unwrap().holds()[0].active);
        assert_eq!(claim(&s), 20);
    }
}

#[test]
fn completed_partial_without_consent_is_recorded_but_not_reported_as_full_success() {
    let t = Temp::new();
    let mut s = setup(&t, false);
    let mut i = intent(3, true);
    i.allow_partial = false;
    assert_eq!(
        commit(
            &mut s,
            3,
            vec![],
            vec![accept(i), prepare(3, 80), Control::Expose(attempt(3))]
        )
        .receipt
        .controls,
        None
    );
    commit(
        &mut s,
        4,
        vec![
            leg(10, Leg::Debit, 40, 0),
            leg(11, Leg::Arrive(Destination::Recipient([42; 32])), 40, 2),
        ],
        vec![],
    );
    assert_eq!(claim(&s), 62);
    let result = finish(
        &mut s,
        5,
        40,
        vec![key(10, Location::Vault), key(11, Location::Vault)],
        true,
    );
    assert_eq!(result.receipt.controls, Some(ControlError::Unqualified));
    assert!(s.state().unwrap().funds()[0].faulted);
    assert!(!s.state().unwrap().funds()[0].terminal);
    assert_eq!(s.state().unwrap().ledger().paid(user(1)).unwrap(), cash(38));
}
