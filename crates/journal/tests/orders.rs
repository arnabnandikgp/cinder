//! Joined offline order lifecycle, not a native adapter qualification.
mod support;
use cinder_journal::{model::*, orders::*, wire, *};
use cinder_kernel::{
    identity::*,
    ledger::{economics::EconomicChange, evidence::Disposition, *},
};
use support::*;

fn intent(n: u8, lots: i64) -> Intent {
    Intent {
        time_in_force: TimeInForce::GoodTilCancelled,
        request: request(n),
        quantity: q(lots),
        minimum: p(90),
        maximum: p(110),
        maximum_fee_per_lot: cash(1),
        reduce_only: false,
        policy: config().policy,
        authority_epoch: 1,
        expires_at: 100,
    }
}
fn accept(i: Intent, amount: i128) -> Control {
    let approval = Approval {
        account: i.request.account,
        intent_hash: i.digest().unwrap(),
        authority_epoch: i.authority_epoch,
    };
    Control::Order(Action::Accept {
        reservations: vec![
            Reservation {
                resource: Resource::Customer(i.request.account),
                amount: cash(amount),
            },
            Reservation {
                resource: Resource::Location(Location::Venue),
                amount: cash(amount),
            },
        ],
        intent: Box::new(i),
        approval,
    })
}
fn commit(s: &mut Store, n: u8, events: Vec<Event>, controls: Vec<Control>) -> Committed {
    s.commit(transaction(s.head(), n, events, controls))
        .unwrap()
}
fn setup(temp: &Temp) -> Store {
    let mut s = temp.create();
    let result = commit(
        &mut s,
        1,
        vec![
            receipt(1, Owner::Customer(user(1)), 1000),
            receipt(2, Owner::Customer(user(2)), 1000),
        ],
        vec![
            Control::Order(Action::AdvanceAuthority {
                account: user(1),
                epoch: 1,
            }),
            Control::Order(Action::AdvanceAuthority {
                account: user(2),
                epoch: 1,
            }),
        ],
    );
    assert_eq!(result.receipt.controls, None);
    s
}
fn place(s: &mut Store, n: u8, lots: i64, exposed: bool) {
    let mut controls = vec![
        accept(intent(n, lots), 100),
        Control::Order(Action::Prepare {
            attempt: attempt(n),
        }),
    ];
    if exposed {
        controls.push(Control::Expose(attempt(n)));
    }
    let result = commit(s, n, vec![], controls);
    assert_eq!(result.receipt.controls, None);
    assert_eq!(result.exposures.len(), usize::from(exposed));
}
fn fill(n: u64, a: AttemptKey, lots: i64, price: u64, fee: i128) -> Event {
    event(
        n,
        Change::Economics(EconomicChange::Execution {
            target: FillTarget::Customer(a),
            quantity: q(lots),
            price: p(price),
            fee: cash(fee),
            pnl: None,
        }),
    )
}
fn status(n: u64, a: AttemptKey, status: Status) -> Observation {
    Observation {
        key: key(n),
        attempt: a,
        status,
        authority_epoch: 1,
        observed_at: 10,
        raw: raw(b"sanitized-order-response"),
    }
}
fn observe(
    s: &mut Store,
    n: u8,
    observations: Vec<Observation>,
    controls: Vec<Control>,
) -> Committed {
    let mut tx = transaction(s.head(), n, vec![], controls);
    tx.order_observations = observations;
    s.commit(tx).unwrap()
}
fn terminal(filled: i64, ids: &[u64]) -> Status {
    Status::Terminal(Terminal {
        filled: q(filled),
        executions: ids.iter().copied().map(key).collect(),
        through: 10,
    })
}
fn release(n: u8) -> Control {
    Control::Order(Action::Release {
        request: request(n),
    })
}

#[test]
fn incomplete_native_funding_still_blocks_ordinary_order_exposure() {
    let t = Temp::new();
    let mut s = setup(&t);
    place(&mut s, 3, 2, false);
    assert_eq!(
        commit(
            &mut s,
            4,
            vec![],
            vec![Control::Funds(
                cinder_journal::funds::Action::NativeCreditReady(false)
            )]
        )
        .receipt
        .controls,
        None
    );
    let result = commit(&mut s, 5, vec![], vec![Control::Expose(attempt(3))]);
    assert_eq!(result.receipt.controls, Some(ControlError::Unqualified));
    assert!(result.exposures.is_empty());
    assert!(!s.state().unwrap().attempts()[0].possibly_exposed);
}

#[test]
fn restoration_receipt_for_ordinary_order_is_retained_and_rejected_without_panic() {
    for exposed in [false, true] {
        for price in [100, 111] {
            let t = Temp::new();
            let mut s = setup(&t);
            place(&mut s, 2, 4, exposed);
            let before = s.state().unwrap().ledger().venue().clone();
            let e = event(
                80,
                Change::Restoration(
                    cinder_kernel::ledger::restoration::RestorationChange::Receipt {
                        attempt: attempt(2),
                        quantity: q(1),
                        price: p(price),
                        fee: cash(0),
                        pnl: None,
                        executed_at: 10,
                    },
                ),
            );
            let tx = transaction(s.head(), 80, vec![e], vec![]);
            let id = tx.id;
            let retained = tx.inputs.clone();
            let result = s.commit(tx).unwrap();
            assert_eq!(
                result.receipt.inputs,
                vec![InputResult::Normalized(Disposition::Rejected(
                    LedgerError::Attribution
                ))]
            );
            assert_eq!(s.transaction(id).unwrap().inputs, retained);
            assert_eq!(s.state().unwrap().ledger().venue(), &before);
            assert!(s.state().unwrap().orders()[0].faulted);
            assert!(s.state().unwrap().holds()[0].active);
            assert_eq!(t.open().state().unwrap(), s.state().unwrap());
        }
    }
}

#[test]
fn authorized_intent_precedes_one_shot_exposure_and_replays_exactly() {
    let temp = Temp::new();
    let mut s = setup(&temp);
    let tx = transaction(
        s.head(),
        2,
        vec![],
        vec![
            accept(intent(2, 4), 100),
            Control::Order(Action::Prepare {
                attempt: attempt(2),
            }),
            Control::Expose(attempt(2)),
        ],
    );
    assert_eq!(
        wire::decode_transaction(&wire::encode_transaction(&tx).unwrap()).unwrap(),
        tx
    );
    let result = s.commit(tx.clone()).unwrap();
    assert_eq!(result.receipt.controls, None);
    assert_eq!(result.exposures.len(), 1);
    let (_, message) = result.exposures.into_iter().next().unwrap().into_message();
    assert_eq!(message, s.state().unwrap().attempts()[0].message);
    assert!(s.commit(tx.clone()).unwrap().exposures.is_empty());
    let state = s.state().unwrap().clone();
    drop(s);
    let mut s = temp.open();
    assert_eq!(&state, s.state().unwrap());
    assert!(s.commit(tx).unwrap().duplicate);
    assert_eq!(
        commit(&mut s, 3, vec![], vec![accept(intent(2, 5), 100)])
            .receipt
            .controls,
        Some(ControlError::Invalid)
    );
    assert_eq!(s.state().unwrap().orders()[0].intent.quantity, q(4));
    assert_eq!(
        commit(&mut s, 4, vec![], vec![Control::Expose(attempt(2))])
            .receipt
            .controls,
        Some(ControlError::Exposed)
    );
    assert_eq!(
        commit(&mut s, 5, vec![], vec![prepare(2)]).receipt.controls,
        Some(ControlError::Invalid)
    );
}

#[test]
fn bad_auth_or_reservation_rolls_back_all_control_side_effects() {
    for variant in 0..7 {
        let temp = Temp::new();
        let mut s = setup(&temp);
        let mut action = accept(intent(2, 4), 100);
        if let Control::Order(Action::Accept {
            intent,
            approval,
            reservations,
        }) = &mut action
        {
            match variant {
                0 => approval.account = user(2),
                1 => approval.intent_hash[0] ^= 1,
                2 => approval.authority_epoch = 2,
                3 => intent.expires_at = 9,
                4 => reservations[0].amount = cash(1001),
                5 => {
                    intent.quantity = q(0);
                    approval.intent_hash = intent.digest().unwrap();
                }
                _ => {
                    intent.authority_epoch = 2;
                    approval.authority_epoch = 2;
                    approval.intent_hash = intent.digest().unwrap();
                }
            }
        }
        assert!(
            commit(
                &mut s,
                2,
                vec![],
                vec![
                    action,
                    Control::Order(Action::Prepare {
                        attempt: attempt(2)
                    })
                ]
            )
            .receipt
            .controls
            .is_some()
        );
        assert!(s.state().unwrap().orders().is_empty());
        assert!(s.state().unwrap().holds().is_empty());
        assert!(s.state().unwrap().attempts().is_empty());
        assert_eq!(
            s.state()
                .unwrap()
                .ledger()
                .book(Owner::Customer(user(1)))
                .unwrap()
                .cash(),
            cash(1000)
        );
    }
}

#[test]
fn ack_cancel_ack_unknown_and_expiry_never_release_or_post_a_fill() {
    let temp = Temp::new();
    let mut s = setup(&temp);
    place(&mut s, 2, 4, true);
    for (n, st) in [
        (3, Status::Acknowledged),
        (4, Status::CancelAcknowledged),
        (5, Status::Unknown),
    ] {
        let result = observe(
            &mut s,
            n,
            vec![status(u64::from(n), attempt(2), st)],
            vec![release(2)],
        );
        assert_eq!(result.receipt.controls, Some(ControlError::Unqualified));
        assert_eq!(
            s.state()
                .unwrap()
                .reserved(Resource::Customer(user(1)))
                .unwrap(),
            cash(100)
        );
        assert_eq!(s.state().unwrap().orders()[0].filled, q(0));
    }
    let mut tx = transaction(s.head(), 6, vec![], vec![release(2)]);
    tx.at = 1000;
    assert_eq!(
        s.commit(tx).unwrap().receipt.controls,
        Some(ControlError::Unqualified)
    );
    let state = s.state().unwrap().clone();
    drop(s);
    assert_eq!(&state, temp.open().state().unwrap());
}

#[test]
fn terminal_before_partial_fill_requires_exact_history_not_ack_or_quantity_alone() {
    let temp = Temp::new();
    let mut s = setup(&temp);
    place(&mut s, 2, 4, true);
    let proof = status(50, attempt(2), terminal(3, &[20, 21]));
    assert_eq!(
        observe(&mut s, 3, vec![proof.clone()], vec![release(2)])
            .receipt
            .controls,
        Some(ControlError::Unqualified)
    );
    commit(&mut s, 4, vec![fill(20, attempt(2), 1, 100, 1)], vec![]);
    assert_eq!(s.state().unwrap().orders()[0].remaining(), 3);
    assert_eq!(
        commit(&mut s, 5, vec![], vec![release(2)]).receipt.controls,
        Some(ControlError::Unqualified)
    );
    let result = commit(
        &mut s,
        6,
        vec![fill(21, attempt(2), 2, 101, 2)],
        vec![release(2)],
    );
    assert_eq!(result.receipt.controls, None);
    assert!(s.state().unwrap().orders()[0].complete());
    assert_eq!(s.state().unwrap().orders()[0].remaining(), 1);
    assert_eq!(
        s.state()
            .unwrap()
            .reserved(Resource::Customer(user(1)))
            .unwrap(),
        cash(0)
    );
    let b = s
        .state()
        .unwrap()
        .ledger()
        .book(Owner::Customer(user(1)))
        .unwrap();
    assert_eq!(b.cash(), cash(997));
    assert_eq!(b.positions()[0].quantity(), q(3));
    assert_eq!(b.positions()[0].basis().atoms(), 302);
    let mut duplicate = proof;
    duplicate.observed_at = 9;
    duplicate.raw = raw(b"same native fact different transport");
    assert_eq!(
        observe(&mut s, 7, vec![duplicate], vec![])
            .receipt
            .order_observations,
        vec![true]
    );
    let result = commit(&mut s, 8, vec![fill(20, attempt(2), 1, 100, 1)], vec![]);
    assert_eq!(
        result.receipt.inputs,
        vec![InputResult::Normalized(Disposition::Duplicate)]
    );
    let state = s.state().unwrap().clone();
    drop(s);
    assert_eq!(&state, temp.open().state().unwrap());
}

#[test]
fn adverse_actual_fill_is_booked_to_native_and_suspense_despite_rejected_controls() {
    for variant in 0..4 {
        let temp = Temp::new();
        let mut s = setup(&temp);
        place(&mut s, 2, 4, true);
        let (lots, price, fee) = match variant {
            0 => (5, 100, 0),
            1 => (1, 111, 0),
            2 => (1, 100, 2),
            _ => (-1, 100, 0),
        };
        let event = fill(20, attempt(2), lots, price, fee);
        let result = commit(&mut s, 3, vec![event.clone()], vec![release(2)]);
        assert_eq!(
            result.receipt.inputs,
            vec![InputResult::Normalized(Disposition::Applied)]
        );
        assert_eq!(result.receipt.controls, Some(ControlError::Unqualified));
        let state = s.state().unwrap();
        assert!(state.orders()[0].faulted);
        assert_eq!(state.ledger().venue().positions()[0].quantity(), q(lots));
        assert_eq!(
            state
                .ledger()
                .book(Owner::Customer(user(1)))
                .unwrap()
                .positions()[0]
                .quantity(),
            q(0)
        );
        assert_eq!(
            state.ledger().book(Owner::Suspense).unwrap().positions()[0].quantity(),
            q(lots)
        );
        assert_eq!(
            state.ledger().book(Owner::Suspense).unwrap().cash(),
            cash(-fee)
        );
        assert_eq!(
            commit(&mut s, 4, vec![event], vec![]).receipt.inputs,
            vec![InputResult::Normalized(Disposition::Duplicate)]
        );
        assert_eq!(s.state().unwrap().orders()[0].executions.len(), 1);
        let state = s.state().unwrap().clone();
        drop(s);
        assert_eq!(&state, temp.open().state().unwrap());
    }
}

#[test]
fn late_contradiction_reencumbers_released_hold_and_preserves_real_exposure() {
    let temp = Temp::new();
    let mut s = setup(&temp);
    place(&mut s, 2, 4, true);
    assert_eq!(
        observe(
            &mut s,
            3,
            vec![status(50, attempt(2), terminal(0, &[]))],
            vec![release(2)]
        )
        .receipt
        .controls,
        None
    );
    assert!(!s.state().unwrap().holds()[0].active);
    commit(&mut s, 4, vec![fill(20, attempt(2), 1, 100, 0)], vec![]);
    let state = s.state().unwrap();
    assert!(state.orders()[0].faulted);
    assert!(state.holds()[0].active);
    assert_eq!(state.ledger().venue().positions()[0].quantity(), q(1));
    assert_eq!(
        state
            .ledger()
            .book(Owner::Customer(user(1)))
            .unwrap()
            .positions()[0]
            .quantity(),
        q(1)
    );
    assert_eq!(
        commit(&mut s, 5, vec![], vec![release(2)]).receipt.controls,
        Some(ControlError::Unqualified)
    );
}

#[test]
fn cancel_is_scoped_durable_and_available_for_cleanup_after_containment() {
    let temp = Temp::new();
    let mut s = setup(&temp);
    place(&mut s, 2, 4, true);
    commit(&mut s, 3, vec![fill(20, attempt(2), 1, 111, 0)], vec![]);
    let cancel = AttemptKey {
        request: request(2),
        attempt: AttemptId::new([99; 32]).unwrap(),
    };
    let controls = vec![
        Control::Order(Action::PrepareCancel {
            attempt: cancel,
            authority_epoch: 1,
            expires_at: 100,
        }),
        Control::Expose(cancel),
    ];
    let tx = transaction(s.head(), 4, vec![], controls);
    assert_eq!(
        wire::decode_transaction(&wire::encode_transaction(&tx).unwrap()).unwrap(),
        tx
    );
    let result = s.commit(tx.clone()).unwrap();
    assert_eq!(result.receipt.controls, None);
    assert_eq!(result.exposures.len(), 1);
    assert_eq!(s.state().unwrap().attempts()[1].kind, AttemptKind::Cancel);
    assert!(s.commit(tx).unwrap().exposures.is_empty());
    let state = s.state().unwrap().clone();
    drop(s);
    assert_eq!(&state, temp.open().state().unwrap());
}

#[test]
fn revoked_epoch_blocks_unsent_place_but_does_not_unreserve_escaped_attempts() {
    let temp = Temp::new();
    let mut s = setup(&temp);
    place(&mut s, 2, 4, false);
    assert_eq!(
        commit(
            &mut s,
            3,
            vec![],
            vec![Control::Order(Action::AdvanceAuthority {
                account: user(1),
                epoch: 2
            })]
        )
        .receipt
        .controls,
        None
    );
    assert_eq!(
        commit(&mut s, 4, vec![], vec![Control::Expose(attempt(2))])
            .receipt
            .controls,
        Some(ControlError::Invalid)
    );
    assert!(s.state().unwrap().holds()[0].active);
    assert_eq!(
        commit(
            &mut s,
            5,
            vec![],
            vec![Control::Order(Action::AdvanceAuthority {
                account: user(1),
                epoch: 1
            })]
        )
        .receipt
        .controls,
        Some(ControlError::Invalid)
    );
}

#[test]
fn missing_causal_cut_and_conflicting_status_never_qualify_terminal_release() {
    let temp = Temp::new();
    let mut s = setup(&temp);
    place(&mut s, 2, 4, true);
    let mut tx = transaction(s.head(), 3, vec![fill(20, attempt(2), 1, 100, 0)], vec![]);
    tx.inputs[0].source_cut = None;
    s.commit(tx).unwrap();
    assert_eq!(
        observe(
            &mut s,
            4,
            vec![status(50, attempt(2), terminal(1, &[20]))],
            vec![release(2)]
        )
        .receipt
        .controls,
        Some(ControlError::Unqualified)
    );
    let result = observe(
        &mut s,
        5,
        vec![status(50, attempt(2), terminal(0, &[]))],
        vec![],
    );
    assert_eq!(result.receipt.order_observations, vec![false]);
    assert!(s.state().unwrap().orders()[0].faulted);
    assert!(s.state().unwrap().holds()[0].active);
}

#[test]
fn independent_pending_buy_sell_outcomes_are_not_netted_commitments() {
    for buy in [false, true] {
        for sell in [false, true] {
            let temp = Temp::new();
            let mut s = setup(&temp);
            place(&mut s, 2, 4, true);
            place(&mut s, 3, -4, true);
            assert_eq!(
                s.state()
                    .unwrap()
                    .reserved(Resource::Customer(user(1)))
                    .unwrap(),
                cash(200)
            );
            let mut events = vec![];
            if buy {
                events.push(fill(20, attempt(2), 4, 100, 0));
            }
            if sell {
                events.push(fill(21, attempt(3), -4, 100, 0));
            }
            commit(&mut s, 4, events, vec![]);
            assert_eq!(
                s.state().unwrap().ledger().venue().positions()[0].quantity(),
                q(i64::from(buy) * 4 - i64::from(sell) * 4)
            );
            // Filled or not, neither pending operation releases without a full witness.
            assert_eq!(
                s.state()
                    .unwrap()
                    .reserved(Resource::Customer(user(1)))
                    .unwrap(),
                cash(200)
            );
        }
    }
}

#[test]
fn property_all_24_partial_terminal_delivery_orders_converge_after_durable_reloads() {
    let mut schedules = vec![];
    for a in 0..4 {
        for b in 0..4 {
            for c in 0..4 {
                for d in 0..4 {
                    if a != b && a != c && a != d && b != c && b != d && c != d {
                        schedules.push([a, b, c, d]);
                    }
                }
            }
        }
    }
    assert_eq!(schedules.len(), 24);
    for schedule in schedules {
        let temp = Temp::new();
        let mut s = setup(&temp);
        place(&mut s, 2, 4, true);
        for (step, item) in schedule.into_iter().enumerate() {
            let n = u8::try_from(3 + 2 * step).unwrap();
            if item == 3 {
                let obs = status(50, attempt(2), terminal(3, &[20, 21, 22]));
                assert_eq!(
                    observe(&mut s, n, vec![obs.clone()], vec![])
                        .receipt
                        .order_observations,
                    vec![true]
                );
                observe(&mut s, n + 1, vec![obs], vec![]);
            } else {
                let e = fill(20 + item, attempt(2), 1, 90 + 10 * item, 1);
                assert_eq!(
                    commit(&mut s, n, vec![e.clone()], vec![]).receipt.inputs,
                    vec![InputResult::Normalized(Disposition::Applied)]
                );
                assert_eq!(
                    commit(&mut s, n + 1, vec![e], vec![]).receipt.inputs,
                    vec![InputResult::Normalized(Disposition::Duplicate)]
                );
            }
            drop(s);
            s = temp.open();
        }
        assert_eq!(
            commit(&mut s, 20, vec![], vec![release(2)])
                .receipt
                .controls,
            None
        );
        let state = s.state().unwrap();
        let b = state.ledger().book(Owner::Customer(user(1))).unwrap();
        assert_eq!(b.positions()[0].quantity(), q(3));
        assert_eq!(b.positions()[0].basis().atoms(), 300);
        assert_eq!(b.cash(), cash(997));
        assert!(state.orders()[0].complete());
        assert!(!state.holds()[0].active);
    }
}

#[test]
fn unknown_durable_order_commit_cannot_deliver_or_resign_after_restart() {
    use cinder_journal::sqlite::{Migration, SqliteBackend};
    struct LostReply(SqliteBackend);
    impl Backend for LostReply {
        fn load(&mut self) -> Result<Vec<Frame>, Error> {
            self.0.load()
        }
        fn append(&mut self, expected: Option<Head>, frame: &Frame) -> Result<(), Error> {
            self.0.append(expected, frame)?;
            Err(Error::Storage)
        }
    }
    let temp = Temp::new();
    let s = setup(&temp);
    let head = s.head();
    drop(s);
    let tx = transaction(
        head,
        2,
        vec![],
        vec![
            accept(intent(2, 4), 100),
            Control::Order(Action::Prepare {
                attempt: attempt(2),
            }),
            Control::Expose(attempt(2)),
        ],
    );
    let backend = LostReply(SqliteBackend::open(&temp.db, Migration::None).unwrap());
    let mut s = Journal::open(backend, FixtureProtection, config()).unwrap();
    assert_eq!(s.commit(tx.clone()).unwrap_err(), Error::Storage);
    assert_eq!(s.state().unwrap_err(), Error::Poisoned);
    drop(s);
    let mut s = temp.open();
    let result = s.commit(tx).unwrap();
    assert!(result.duplicate);
    assert!(result.exposures.is_empty());
    assert!(s.state().unwrap().attempts()[0].possibly_exposed);
    assert!(s.state().unwrap().holds()[0].active);
    assert_eq!(
        commit(&mut s, 3, vec![], vec![Control::Expose(attempt(2))])
            .receipt
            .controls,
        Some(ControlError::Exposed)
    );
}

#[test]
fn conflicting_originals_cannot_hide_behind_the_same_suspense_classification() {
    let temp = Temp::new();
    let mut s = setup(&temp);
    place(&mut s, 2, 4, true);
    let e = fill(20, attempt(2), 1, 111, 0);
    commit(&mut s, 3, vec![e], vec![]);
    let conflict = fill(20, attempt(2), 2, 111, 0);
    let result = commit(&mut s, 4, vec![conflict], vec![]);
    assert!(matches!(
        result.receipt.inputs[0],
        InputResult::Normalized(Disposition::Rejected(LedgerError::Conflict))
    ));
    assert_eq!(
        s.state().unwrap().ledger().venue().positions()[0].quantity(),
        q(1)
    );
    assert!(s.state().unwrap().orders()[0].faulted);
    assert!(s.state().unwrap().unresolved_raw() > 0);
}
