//! Synthetic qualified ports only; no chain/RPC truth is inferred by these tests.
mod support;
use cinder_journal::{escrow::*, funds, model::*, orders, sqlite::*, wire, *};
use cinder_kernel::{
    identity::*,
    ledger::{evidence::*, funds::*, *},
};
use support::{FixtureProtection, Store, Temp, attempt, cash, p, raw, request, user};

fn config() -> Config {
    let mut c = support::config();
    for (n, location) in [(8, Location::Broker), (9, Location::Vault)] {
        let mut s = c.sources[0];
        s.location = location;
        s.scope.namespace = NamespaceId::new([n; 32]).unwrap();
        c.sources.push(s);
    }
    c
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
fn batch(n: u8) -> Batch {
    Batch {
        request: request(n),
        route: [20; 32],
        batch: [n; 32],
        operation: [n; 32],
        owner: [21; 32],
        amount: cash(100),
        epoch: 1,
        expires_at_slot: 500,
        signature: [n; 64],
    }
}
fn create(t: &Temp) -> Store {
    Journal::create(
        SqliteBackend::create(&t.db).unwrap(),
        FixtureProtection,
        config(),
    )
    .unwrap()
}
fn reopen(t: &Temp) -> Store {
    Journal::open(
        SqliteBackend::open(&t.db, Migration::None).unwrap(),
        FixtureProtection,
        config(),
    )
    .unwrap()
}
fn commit(s: &mut Store, n: u8, controls: Vec<Control>) -> Committed {
    let tx = support::transaction(s.head(), n, vec![], controls);
    assert_eq!(
        wire::decode_transaction(&wire::encode_transaction(&tx).unwrap()).unwrap(),
        tx
    );
    s.commit(tx).unwrap()
}
fn escrow(action: Action) -> Control {
    Control::Escrow(action)
}
fn pending(s: &mut Store, n: u8) {
    assert_eq!(
        commit(s, n, vec![escrow(Action::Pending(Box::new(batch(n))))])
            .receipt
            .controls,
        None
    );
}
fn prepare(n: u8) -> Control {
    escrow(Action::Prepare {
        attempt: attempt(n),
        message: raw(b"exact qualified sweep action"),
        epoch: 1,
        expires_at: 100,
    })
}
fn handoff(n: u8) -> Control {
    escrow(Action::Handoff {
        attempt: attempt(n),
        receipt: batch(n).handoff(source(Location::Broker)).unwrap(),
        slot: 200,
        observed_at: 10,
        raw: raw(b"qualified finalized original sweep"),
    })
}
fn swept(s: &mut Store, n: u8) {
    pending(s, n);
    assert_eq!(
        commit(s, n + 1, vec![prepare(n), Control::Expose(attempt(n))])
            .receipt
            .controls,
        None
    );
    assert_eq!(commit(s, n + 2, vec![handoff(n)]).receipt.controls, None);
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
fn pending_is_not_cash_or_an_active_recovery_commitment_and_refund_never_pays_a_claim() {
    let t = Temp::new();
    let mut s = create(&t);
    pending(&mut s, 10);
    assert_eq!(claim(&s), 0);
    assert_eq!(s.state().unwrap().ledger().broker(), cash(0));
    assert!(s.state().unwrap().holds().iter().all(|h| !h.active));
    assert_eq!(
        commit(
            &mut s,
            11,
            vec![
                Control::Funds(funds::Action::Freeze),
                escrow(Action::Refund(request(10)))
            ]
        )
        .receipt
        .controls,
        None
    );
    assert_eq!(claim(&s), 0);
    assert_eq!(s.state().unwrap().ledger().paid(user(1)).unwrap(), cash(0));
    assert_eq!(s.state().unwrap().escrows()[0].status, Status::Refunded);
    assert!(
        commit(&mut s, 12, vec![prepare(10)])
            .receipt
            .controls
            .is_some()
    );
    let expected = s.state().unwrap().clone();
    drop(s);
    assert_eq!(reopen(&t).state().unwrap(), &expected);
}
#[test]
fn handoff_is_exactly_once_and_cannot_release_funding_holds_by_generic_controls() {
    let t = Temp::new();
    let mut s = create(&t);
    swept(&mut s, 10);
    assert_eq!(claim(&s), 100);
    assert_eq!(s.state().unwrap().ledger().broker(), cash(100));
    for resource in [
        Resource::Customer(user(1)),
        Resource::Location(Location::Broker),
    ] {
        assert_eq!(s.state().unwrap().reserved(resource).unwrap(), cash(100));
    }
    assert_eq!(
        s.state()
            .unwrap()
            .available_location(Location::Broker)
            .unwrap(),
        cash(0)
    );
    assert_eq!(commit(&mut s, 13, vec![handoff(10)]).receipt.controls, None);
    assert_eq!(claim(&s), 100);
    assert!(
        commit(&mut s, 14, vec![Control::Release(request(10))])
            .receipt
            .controls
            .is_some()
    );
    assert!(
        commit(&mut s, 15, vec![escrow(Action::Refund(request(10)))])
            .receipt
            .controls
            .is_some()
    );
    assert!(
        commit(&mut s, 16, vec![escrow(Action::Finish(request(10)))])
            .receipt
            .controls
            .is_some()
    );
    commit(
        &mut s,
        17,
        vec![Control::Funds(funds::Action::NativeCreditReady(true))],
    );
    assert!(!s.state().unwrap().native_funding_ready());
    let expected = s.state().unwrap().clone();
    drop(s);
    assert_eq!(reopen(&t).state().unwrap(), &expected);
}
#[test]
fn refund_wins_after_exposure_or_sweep_wins_but_not_both_and_timeout_cannot_cancel() {
    for refund_first in [false, true] {
        let t = Temp::new();
        let mut s = create(&t);
        pending(&mut s, 10);
        assert_eq!(
            commit(&mut s, 11, vec![prepare(10), Control::Expose(attempt(10))])
                .receipt
                .controls,
            None
        );
        assert!(
            commit(&mut s, 12, vec![escrow(Action::Cancel(request(10)))])
                .receipt
                .controls
                .is_some()
        );
        assert!(
            commit(&mut s, 13, vec![Control::Expose(attempt(10))])
                .receipt
                .controls
                .is_some()
        );
        let refund = escrow(Action::Refund(request(10)));
        assert_eq!(
            commit(
                &mut s,
                14,
                vec![if refund_first {
                    refund.clone()
                } else {
                    handoff(10)
                }]
            )
            .receipt
            .controls,
            None
        );
        assert!(
            commit(
                &mut s,
                15,
                vec![if refund_first { handoff(10) } else { refund }]
            )
            .receipt
            .controls
            .is_some()
        );
        assert_eq!(claim(&s), if refund_first { 0 } else { 100 });
        assert!(
            s.state()
                .unwrap()
                .qualified_escrow_delivery(attempt(10))
                .is_err()
        );
    }
}
#[test]
fn failed_control_candidate_never_leaves_unrestricted_customer_credit() {
    let t = Temp::new();
    let mut s = create(&t);
    pending(&mut s, 10);
    commit(&mut s, 11, vec![prepare(10), Control::Expose(attempt(10))]);
    assert!(
        commit(&mut s, 12, vec![handoff(10), Control::Release(request(99))])
            .receipt
            .controls
            .is_some()
    );
    assert_eq!(claim(&s), 0);
    assert_eq!(s.state().unwrap().escrows()[0].status, Status::Exposed);
    assert_eq!(
        commit(
            &mut s,
            13,
            vec![Control::Funds(funds::Action::Freeze), handoff(10)]
        )
        .receipt
        .controls,
        None
    );
    assert_eq!(claim(&s), 100);
    assert_eq!(
        s.state()
            .unwrap()
            .reserved(Resource::Customer(user(1)))
            .unwrap(),
        cash(100)
    );
}
#[test]
fn conflicting_owner_batch_request_amount_or_source_does_not_duplicate_recognition() {
    let t = Temp::new();
    let mut s = create(&t);
    pending(&mut s, 10);
    for (i, changed) in [
        (11, {
            let mut b = batch(10);
            b.amount = cash(101);
            b
        }),
        (12, {
            let mut b = batch(10);
            b.request = request(12);
            b
        }),
        (13, {
            let mut b = batch(10);
            b.owner = [22; 32];
            b
        }),
    ] {
        assert!(
            commit(&mut s, i, vec![escrow(Action::Pending(Box::new(changed)))])
                .receipt
                .controls
                .is_some()
        );
    }
    commit(&mut s, 14, vec![prepare(10), Control::Expose(attempt(10))]);
    let bad = escrow(Action::Handoff {
        attempt: attempt(10),
        receipt: batch(10).handoff(source(Location::Vault)).unwrap(),
        slot: 200,
        observed_at: 10,
        raw: raw(b"wrong route"),
    });
    assert!(commit(&mut s, 15, vec![bad]).receipt.controls.is_some());
    assert_eq!(claim(&s), 0);
}
fn install_collateral(s: &mut Store) {
    let ledger = s.state().unwrap().ledger();
    let version = ledger.version();
    let event = Event {
        key: RecordKey::Economic(key(80, Location::Venue)),
        policy: config().policy,
        change: Change::Reconcile(NativeCheck {
            expected_version: version,
            cash: Some(ledger.venue().cash()),
            funding: Some(ledger.venue().funding()),
            positions: Some(ledger.venue().positions().to_vec()),
            complete: true,
            resolves: vec![],
        }),
    };
    let mut tx = support::transaction(
        s.head(),
        80,
        vec![],
        vec![Control::Collateral(cinder_journal::collateral::Cut {
            expected_version: version + 1,
            policy: cinder_journal::collateral::Policy {
                revision: config().policy,
                markets: vec![cinder_journal::collateral::MarginRule {
                    market: p(100).unit(),
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
                evidence: key(81, Location::Venue),
                observed_at: 10,
                valid_until: 100,
                qualified: true,
            }],
        })],
    );
    tx.inputs = vec![Input {
        source: source(Location::Venue),
        source_cut: Some(10),
        authority_epoch: 1,
        observed_at: 10,
        raw: raw(b"qualified native check"),
        event: Some(event),
    }];
    assert_eq!(s.commit(tx).unwrap().receipt.controls, None);
}
fn fund(s: &mut Store) {
    install_collateral(s);
    commit(
        s,
        81,
        vec![Control::Order(orders::Action::AdvanceAuthority {
            account: user(1),
            epoch: 1,
        })],
    );
    let intent = funds::Intent {
        request: request(50),
        source: Location::Broker,
        destination: Destination::Location(Location::Venue),
        net: cash(100),
        maximum_fee: cash(0),
        fee_payer: Owner::House,
        allow_partial: false,
        policy: config().policy,
        authority_epoch: 1,
        expires_at: 100,
    };
    let approval = orders::Approval {
        account: user(1),
        authority_epoch: 1,
        intent_hash: intent.digest().unwrap(),
    };
    assert_eq!(
        commit(
            s,
            82,
            vec![
                escrow(Action::Fund {
                    request: request(10),
                    intent: Box::new(intent),
                    approval
                }),
                Control::Funds(funds::Action::Prepare {
                    attempt: attempt(50),
                    net: cash(100)
                }),
                Control::Expose(attempt(50))
            ]
        )
        .receipt
        .controls,
        None
    );
}
fn leg(s: &mut Store, n: u8, location: Location, leg: Leg, amount: i128) {
    let mut tx = support::transaction(s.head(), n, vec![], vec![]);
    tx.inputs = vec![Input {
        source: source(location),
        source_cut: Some(10),
        authority_epoch: 1,
        observed_at: 10,
        raw: raw(b"synthetic qualified movement"),
        event: Some(Event {
            key: RecordKey::Economic(key(n, location)),
            policy: config().policy,
            change: Change::Funds(FundsChange::Observe {
                attempt: attempt(50),
                leg,
                amount: cash(amount),
                fee: cash(0),
            }),
        }),
    }];
    assert_eq!(
        s.commit(tx).unwrap().receipt.inputs,
        [InputResult::Normalized(Disposition::Applied)]
    );
}
#[test]
fn partial_or_unqualified_native_credit_keeps_restrictions_full_strong_credit_releases_once() {
    let t = Temp::new();
    let mut s = create(&t);
    swept(&mut s, 10);
    fund(&mut s);
    assert_eq!(claim(&s), 100);
    leg(&mut s, 90, Location::Broker, Leg::Debit, 100);
    leg(
        &mut s,
        91,
        Location::Venue,
        Leg::Arrive(Destination::Location(Location::Venue)),
        40,
    );
    assert!(
        commit(&mut s, 92, vec![escrow(Action::Finish(request(10)))])
            .receipt
            .controls
            .is_some()
    );
    assert_eq!(
        s.state()
            .unwrap()
            .reserved(Resource::Customer(user(1)))
            .unwrap(),
        cash(100)
    );
    leg(
        &mut s,
        93,
        Location::Venue,
        Leg::Arrive(Destination::Location(Location::Venue)),
        60,
    );
    assert!(
        commit(&mut s, 94, vec![escrow(Action::Finish(request(10)))])
            .receipt
            .controls
            .is_some()
    );
    let mut tx = support::transaction(
        s.head(),
        95,
        vec![],
        vec![
            Control::Funds(funds::Action::Finalize(request(50))),
            escrow(Action::Finish(request(10))),
            Control::Funds(funds::Action::NativeCreditReady(true)),
        ],
    );
    tx.funds_observations = vec![funds::Observation {
        key: key(95, Location::Venue),
        terminal: funds::Terminal {
            attempt: attempt(50),
            debit: cash(100),
            settled: cash(100),
            receipts: vec![
                key(90, Location::Broker),
                key(91, Location::Venue),
                key(93, Location::Venue),
            ],
            coverage: config()
                .sources
                .into_iter()
                .map(|s| funds::Coverage {
                    source: s.scope,
                    through: 10,
                })
                .collect(),
            no_later_execution: [1; 32],
        },
        authority_epoch: 1,
        observed_at: 10,
        raw: raw(b"synthetic strong full history and capability fence"),
    }];
    assert_eq!(s.commit(tx).unwrap().receipt.controls, None);
    assert_eq!(s.state().unwrap().escrows()[0].status, Status::Available);
    assert_eq!(
        s.state()
            .unwrap()
            .reserved(Resource::Customer(user(1)))
            .unwrap(),
        cash(0)
    );
    assert_eq!(claim(&s), 100);
    assert_eq!(s.state().unwrap().ledger().broker(), cash(0));
    assert_eq!(s.state().unwrap().ledger().venue().cash(), cash(100));
    assert_eq!(s.state().unwrap().ledger().in_transit().unwrap(), cash(0));
    assert!(s.state().unwrap().native_funding_ready());
    assert_eq!(
        commit(
            &mut s,
            96,
            vec![handoff(10), escrow(Action::Finish(request(10)))]
        )
        .receipt
        .controls,
        None
    );
    assert_eq!(claim(&s), 100);
    let expected = s.state().unwrap().clone();
    drop(s);
    assert_eq!(reopen(&t).state().unwrap(), &expected);
}

fn begin_recovery(s: &mut Store, n: u8) {
    let version = s.state().unwrap().ledger().version();
    assert_eq!(
        commit(
            s,
            n,
            vec![Control::Recovery(cinder_journal::recovery::Action::Begin {
                expected_version: version,
                authority_epoch: 7,
                valid_until: 100,
                fence: [9; 32]
            })]
        )
        .receipt
        .controls,
        None
    );
}
fn recall() -> Control {
    let i = funds::Intent {
        request: request(60),
        source: Location::Broker,
        destination: Destination::Location(Location::Vault),
        net: cash(100),
        maximum_fee: cash(0),
        fee_payer: Owner::House,
        allow_partial: false,
        policy: config().policy,
        authority_epoch: 7,
        expires_at: 100,
    };
    let approval = orders::Approval {
        account: user(1),
        authority_epoch: 7,
        intent_hash: i.digest().unwrap(),
    };
    escrow(Action::Recall {
        request: request(10),
        intent: Box::new(i),
        approval,
    })
}
#[test]
fn unallocated_or_strongly_failed_native_ingress_can_return_for_recovery_without_recredit_or_payment()
 {
    for failed_native in [false, true] {
        let t = Temp::new();
        let mut s = create(&t);
        swept(&mut s, 10);
        if failed_native {
            fund(&mut s);
            let mut tx = support::transaction(
                s.head(),
                100,
                vec![],
                vec![Control::Funds(funds::Action::Finalize(request(50)))],
            );
            tx.funds_observations = vec![funds::Observation {
                key: key(100, Location::Venue),
                terminal: funds::Terminal {
                    attempt: attempt(50),
                    debit: cash(0),
                    settled: cash(0),
                    receipts: vec![],
                    coverage: vec![],
                    no_later_execution: [1; 32],
                },
                authority_epoch: 1,
                observed_at: 10,
                raw: raw(b"synthetic qualified finalized zero-effect original transaction"),
            }];
            assert_eq!(s.commit(tx).unwrap().receipt.controls, None);
        }
        assert!(
            commit(&mut s, 101, vec![recall()])
                .receipt
                .controls
                .is_some()
        );
        begin_recovery(&mut s, 102);
        assert_eq!(
            commit(
                &mut s,
                103,
                vec![
                    recall(),
                    Control::Funds(funds::Action::Prepare {
                        attempt: attempt(60),
                        net: cash(100)
                    }),
                    Control::Expose(attempt(60))
                ]
            )
            .receipt
            .controls,
            None
        );
        assert!(
            commit(&mut s, 104, vec![escrow(Action::FinishReturn(request(10)))])
                .receipt
                .controls
                .is_some()
        );
        for (n, location, kind) in [
            (120, Location::Broker, Leg::Debit),
            (
                121,
                Location::Vault,
                Leg::Arrive(Destination::Location(Location::Vault)),
            ),
        ] {
            let mut tx = support::transaction(s.head(), n, vec![], vec![]);
            tx.inputs = vec![Input {
                source: source(location),
                source_cut: Some(10),
                authority_epoch: 1,
                observed_at: 10,
                raw: raw(b"synthetic qualified actual recovery return"),
                event: Some(Event {
                    key: RecordKey::Economic(key(n, location)),
                    policy: config().policy,
                    change: Change::Funds(FundsChange::Observe {
                        attempt: attempt(60),
                        leg: kind,
                        amount: cash(100),
                        fee: cash(0),
                    }),
                }),
            }];
            assert_eq!(
                s.commit(tx).unwrap().receipt.inputs,
                [InputResult::Normalized(Disposition::Applied)]
            );
        }
        assert_eq!(
            s.state()
                .unwrap()
                .reserved(Resource::Customer(user(1)))
                .unwrap(),
            cash(100)
        );
        let mut tx = support::transaction(
            s.head(),
            122,
            vec![],
            vec![
                Control::Funds(funds::Action::Finalize(request(60))),
                escrow(Action::FinishReturn(request(10))),
            ],
        );
        tx.funds_observations = vec![funds::Observation {
            key: key(122, Location::Vault),
            terminal: funds::Terminal {
                attempt: attempt(60),
                debit: cash(100),
                settled: cash(100),
                receipts: vec![key(120, Location::Broker), key(121, Location::Vault)],
                coverage: config()
                    .sources
                    .into_iter()
                    .map(|s| funds::Coverage {
                        source: s.scope,
                        through: 10,
                    })
                    .collect(),
                no_later_execution: [2; 32],
            },
            authority_epoch: 7,
            observed_at: 10,
            raw: raw(b"synthetic qualified full original return and capability fence"),
        }];
        assert_eq!(s.commit(tx).unwrap().receipt.controls, None);
        assert_eq!(s.state().unwrap().escrows()[0].status, Status::Returned);
        assert_eq!(
            s.state()
                .unwrap()
                .reserved(Resource::Customer(user(1)))
                .unwrap(),
            cash(0)
        );
        assert_eq!(claim(&s), 100);
        assert_eq!(s.state().unwrap().ledger().vault(), cash(100));
        assert_eq!(s.state().unwrap().ledger().broker(), cash(0));
        assert_eq!(s.state().unwrap().ledger().paid(user(1)).unwrap(), cash(0));
        assert!(
            commit(&mut s, 123, vec![escrow(Action::Refund(request(10)))])
                .receipt
                .controls
                .is_some()
        );
        // An unrelated unswept owner's inventory adds no estate claim/active hold.
        let mut b = batch(30);
        b.request.account = user(2);
        assert_eq!(
            commit(&mut s, 124, vec![escrow(Action::Pending(Box::new(b)))])
                .receipt
                .controls,
            None
        );
        let l = s.state().unwrap().ledger();
        let version = l.version();
        let check = Event {
            key: RecordKey::Economic(key(125, Location::Venue)),
            policy: config().policy,
            change: Change::Reconcile(NativeCheck {
                expected_version: version,
                cash: Some(l.venue().cash()),
                funding: Some(l.venue().funding()),
                positions: Some(l.venue().positions().to_vec()),
                complete: true,
                resolves: vec![],
            }),
        };
        let mut tx = support::transaction(s.head(), 125, vec![], vec![]);
        tx.inputs = vec![Input {
            source: source(Location::Venue),
            source_cut: Some(10),
            authority_epoch: 7,
            observed_at: 10,
            raw: raw(b"synthetic final settled native cut"),
            event: Some(check),
        }];
        s.commit(tx).unwrap();
        let head = s.head();
        assert!(s.recovery_cut(head, key(125, Location::Venue)).is_ok());
        let expected = s.state().unwrap().clone();
        drop(s);
        assert_eq!(reopen(&t).state().unwrap(), &expected);
    }
}
#[test]
fn unknown_or_partially_credited_native_ingress_cannot_be_recalled_as_unused_collateral() {
    for partial in [false, true] {
        let t = Temp::new();
        let mut s = create(&t);
        swept(&mut s, 10);
        fund(&mut s);
        if partial {
            leg(&mut s, 90, Location::Broker, Leg::Debit, 100);
            leg(
                &mut s,
                91,
                Location::Venue,
                Leg::Arrive(Destination::Location(Location::Venue)),
                40,
            );
        }
        begin_recovery(&mut s, 100);
        assert!(
            commit(&mut s, 101, vec![recall()])
                .receipt
                .controls
                .is_some()
        );
        assert_eq!(
            s.state()
                .unwrap()
                .reserved(Resource::Customer(user(1)))
                .unwrap(),
            cash(100)
        );
        assert_eq!(claim(&s), 100);
    }
}
