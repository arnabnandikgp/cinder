//! Synthetic qualified internal ports only; not a native credit/chain proof.
mod support;
use cinder_journal::{
    controller::Controller,
    escrow, funds,
    model::*,
    orders,
    sqlite::{Migration, SqliteBackend},
    units, *,
};
use cinder_kernel::{
    identity::*,
    ledger::{evidence::*, funds::Destination, *},
};
use support::{FixtureProtection, Temp, raw};
type Store = units::Journal<SqliteBackend, FixtureProtection>;
fn uid(n: u8) -> UnitId {
    UnitId::new([n; 32]).unwrap()
}
fn account() -> AccountId {
    onboarding::account(support::config().domain, [70; 32]).unwrap()
}
fn req(n: u8) -> RequestKey {
    RequestKey {
        domain: support::config().domain,
        account: account(),
        request: RequestId::new([n; 32]).unwrap(),
    }
}
fn key(n: u8) -> AttemptKey {
    AttemptKey {
        request: req(n),
        attempt: AttemptId::new([n; 32]).unwrap(),
    }
}
fn cfg() -> units::Configuration {
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
                for (tag, location) in [(8, Location::Broker), (9, Location::Vault)] {
                    let mut source = c.sources[0];
                    source.scope.namespace = NamespaceId::new([tag; 32]).unwrap();
                    source.location = location;
                    c.sources.push(source);
                }
                units::Descriptor {
                    id: uid(n),
                    environment: [90; 32],
                    native_account: [n + 3; 32],
                    funding_wallet: [n + 4; 32],
                    custody_config: [n + 5; 32],
                    payout_vault: [n + 6; 32],
                    key_policy: [n + 7; 32],
                    ledger: c,
                }
            })
            .collect(),
    )
    .unwrap()
}
fn fixture(t: &Temp) -> Store {
    let mut j = Store::create(
        SqliteBackend::create(&t.db).unwrap(),
        FixtureProtection,
        cfg(),
    )
    .unwrap();
    j.commit(units::Transaction {
        id: CommitId::new([1; 32]).unwrap(),
        expected: j.head(),
        at: 1,
        change: units::Change::Enroll {
            wallet: [70; 32],
            recipients: vec![
                units::Recipient {
                    unit: uid(40),
                    tokens: [71; 32],
                },
                units::Recipient {
                    unit: uid(50),
                    tokens: [72; 32],
                },
            ],
        },
    })
    .unwrap();
    j
}
fn commit(j: &mut Store, unit: u8, n: u8, controls: Vec<Control>) -> Committed {
    let tx = Transaction {
        id: CommitId::new([n; 32]).unwrap(),
        expected: j.head(),
        at: u64::from(n),
        evidence: vec![],
        inputs: vec![],
        order_observations: vec![],
        funds_observations: vec![],
        controls,
    };
    j.select(uid(unit)).unwrap().commit(tx).unwrap()
}
fn batch() -> escrow::Batch {
    escrow::Batch {
        request: req(20),
        route: [80; 32],
        batch: [81; 32],
        operation: [82; 32],
        owner: [70; 32],
        amount: support::cash(100),
        epoch: 1,
        expires_at_slot: 1000,
        signature: [83; 64],
    }
}
fn pending() -> Control {
    Control::Escrow(escrow::Action::Pending(Box::new(batch())))
}
fn prepare() -> Control {
    Control::Escrow(escrow::Action::Prepare {
        attempt: key(20),
        message: raw(b"qualified original sweep"),
        epoch: 1,
        expires_at: 1000,
    })
}
fn handoff() -> Control {
    let source = cfg().units()[0]
        .ledger
        .sources
        .iter()
        .find(|s| s.location == Location::Broker)
        .unwrap()
        .scope;
    Control::Escrow(escrow::Action::Handoff {
        attempt: key(20),
        receipt: batch().handoff(source).unwrap(),
        slot: 200,
        observed_at: 4,
        raw: raw(b"finalized original batch sweep"),
    })
}
#[test]
fn exact_batch_sweep_and_refund_are_exclusive_and_cannot_move_to_another_unit() {
    for refund in [false, true] {
        let t = Temp::new();
        let mut j = fixture(&t);
        assert_eq!(
            commit(&mut j, 40, 2, vec![pending()]).receipt.controls,
            None
        );
        assert!(
            commit(&mut j, 50, 3, vec![pending()])
                .receipt
                .controls
                .is_some()
        );
        assert_eq!(
            commit(&mut j, 40, 4, vec![prepare(), Control::Expose(key(20))])
                .receipt
                .controls,
            None
        );
        if refund {
            assert_eq!(
                commit(
                    &mut j,
                    40,
                    5,
                    vec![Control::Escrow(escrow::Action::Refund(req(20)))]
                )
                .receipt
                .controls,
                None
            );
            assert!(
                commit(&mut j, 40, 6, vec![handoff()])
                    .receipt
                    .controls
                    .is_some()
            );
        } else {
            assert_eq!(
                commit(&mut j, 40, 5, vec![handoff()]).receipt.controls,
                None
            );
            assert_eq!(
                commit(&mut j, 40, 6, vec![handoff()]).receipt.controls,
                None
            );
            assert!(
                commit(
                    &mut j,
                    40,
                    7,
                    vec![Control::Escrow(escrow::Action::Refund(req(20)))]
                )
                .receipt
                .controls
                .is_some()
            );
        }
        let state = j.state().unwrap().unit(uid(40)).unwrap();
        assert_eq!(
            state
                .ledger()
                .book(Owner::Customer(account()))
                .unwrap()
                .cash()
                .atoms(),
            if refund { 0 } else { 100 }
        );
        assert_eq!(
            j.state()
                .unwrap()
                .unit(uid(50))
                .unwrap()
                .ledger()
                .book(Owner::Customer(account()))
                .unwrap()
                .cash()
                .atoms(),
            0
        );
        assert!(!state.native_funding_ready());
        let expected = j.state().unwrap().clone();
        drop(j);
        let reopened = Store::open(
            SqliteBackend::open(&t.db, Migration::None).unwrap(),
            FixtureProtection,
            cfg(),
        )
        .unwrap();
        assert_eq!(reopened.state().unwrap(), &expected);
    }
}
#[test]
fn escrow_allocation_keeps_full_original_hold_and_missing_native_credit_cannot_release_it() {
    let t = Temp::new();
    let mut j = fixture(&t);
    for (n, c) in [
        (2, pending()),
        (3, prepare()),
        (4, Control::Expose(key(20))),
        (5, handoff()),
    ] {
        assert_eq!(commit(&mut j, 40, n, vec![c]).receipt.controls, None);
    }
    let intent = funds::Intent {
        request: req(30),
        source: Location::Broker,
        destination: Destination::Location(Location::Venue),
        net: support::cash(100),
        maximum_fee: support::cash(0),
        fee_payer: Owner::House,
        allow_partial: false,
        policy: support::config().policy,
        authority_epoch: 1,
        expires_at: 1000,
    };
    let approval = orders::Approval {
        account: account(),
        intent_hash: intent.digest().unwrap(),
        authority_epoch: 1,
    };
    // Existing funds admission requires a qualified current reconciliation and
    // collateral policy. A finalized escrow handoff alone does not supply them.
    let c = &cfg().units()[0].ledger.clone();
    let l = j.state().unwrap().unit(uid(40)).unwrap().ledger();
    let version = l.version();
    let event_key = EventKey {
        scope: c.sources[0].scope,
        event: EconomicEventId::new(&[200]).unwrap(),
        leg: 0,
    };
    let tx = Transaction {
        id: CommitId::new([200; 32]).unwrap(),
        expected: j.head(),
        at: 6,
        evidence: vec![],
        inputs: vec![Input {
            source: c.sources[0].scope,
            source_cut: Some(10),
            authority_epoch: 1,
            observed_at: 6,
            raw: raw(b"qualified native check fixture"),
            event: Some(Event {
                key: RecordKey::Economic(event_key.clone()),
                policy: c.policy,
                change: Change::Reconcile(NativeCheck {
                    expected_version: version,
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
        controls: vec![Control::Collateral(collateral::Cut {
            expected_version: version + 1,
            policy: collateral::Policy {
                revision: c.policy,
                markets: vec![collateral::MarginRule {
                    market: c.markets[0].unit(),
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
                price: support::p(100),
                evidence: event_key,
                observed_at: 6,
                valid_until: 1000,
                qualified: true,
            }],
        })],
    };
    assert_eq!(
        j.select(uid(40))
            .unwrap()
            .commit(tx)
            .unwrap()
            .receipt
            .controls,
        None
    );
    assert_eq!(
        commit(
            &mut j,
            40,
            6,
            vec![Control::Escrow(escrow::Action::Fund {
                request: req(20),
                intent: Box::new(intent),
                approval
            })]
        )
        .receipt
        .controls,
        None
    );
    assert_eq!(
        commit(
            &mut j,
            40,
            7,
            vec![
                Control::Funds(funds::Action::Prepare {
                    attempt: key(30),
                    net: support::cash(100)
                }),
                Control::Expose(key(30))
            ]
        )
        .receipt
        .controls,
        None
    );
    assert!(
        commit(
            &mut j,
            40,
            8,
            vec![Control::Escrow(escrow::Action::Finish(req(20)))]
        )
        .receipt
        .controls
        .is_some()
    );
    assert!(
        commit(
            &mut j,
            50,
            9,
            vec![Control::Funds(funds::Action::Finalize(req(30)))]
        )
        .receipt
        .controls
        .is_some()
    );
    let s = j.state().unwrap().unit(uid(40)).unwrap();
    assert!(!s.native_funding_ready());
    assert_eq!(
        s.ledger()
            .book(Owner::Customer(account()))
            .unwrap()
            .cash()
            .atoms(),
        100
    );
    assert!(s.holds().iter().any(|h| h.active));
}
fn recovery(j: &Store) -> units::Change {
    units::Change::BeginRecovery {
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
    }
}
#[test]
fn recovery_cutover_is_atomic_all_unit_and_cannot_be_inferred_from_one_local_freeze() {
    let t = Temp::new();
    let mut j = fixture(&t);
    assert!(
        commit(
            &mut j,
            40,
            2,
            vec![Control::Recovery(recovery::Action::Begin {
                expected_version: 0,
                authority_epoch: 7,
                valid_until: 1000,
                fence: [99; 32]
            })]
        )
        .receipt
        .controls
        .is_some()
    );
    assert!(!j.state().unwrap().in_recovery());
    let mut invalid = recovery(&j);
    if let units::Change::BeginRecovery { versions, .. } = &mut invalid {
        versions[1].1 += 1;
    }
    let tx = units::Transaction {
        id: CommitId::new([3; 32]).unwrap(),
        expected: j.head(),
        at: 3,
        change: invalid,
    };
    let head = j.head();
    assert!(j.commit(tx).is_err());
    assert_eq!(j.head(), head);
    for u in cfg().units() {
        assert!(!j.state().unwrap().unit(u.id).unwrap().frozen());
    }
    let change = recovery(&j);
    j.commit(units::Transaction {
        id: CommitId::new([4; 32]).unwrap(),
        expected: j.head(),
        at: 4,
        change,
    })
    .unwrap();
    assert!(j.state().unwrap().in_recovery());
    for u in cfg().units() {
        assert!(j.state().unwrap().unit(u.id).unwrap().frozen());
    }
    let expected = j.state().unwrap().clone();
    drop(j);
    let j = Store::open(
        SqliteBackend::open(&t.db, Migration::None).unwrap(),
        FixtureProtection,
        cfg(),
    )
    .unwrap();
    assert_eq!(j.state().unwrap(), &expected);
}
#[test]
fn recovery_cuts_need_global_cutover_current_shared_head_and_their_own_final_check() {
    let t = Temp::new();
    let mut j = fixture(&t);
    let event_key = |u: usize| EventKey {
        scope: cfg().units()[u].ledger.sources[0].scope,
        event: EconomicEventId::new(&[201]).unwrap(),
        leg: 0,
    };
    assert_eq!(
        commit(&mut j, 40, 2, vec![Control::Funds(funds::Action::Freeze)])
            .receipt
            .controls,
        None
    );
    let head = j.head();
    assert_eq!(
        j.select(uid(40))
            .unwrap()
            .recovery_cut(head, event_key(0))
            .unwrap_err(),
        recovery::CutError::NotFrozen
    );
    let change = recovery(&j);
    j.commit(units::Transaction {
        id: CommitId::new([3; 32]).unwrap(),
        expected: j.head(),
        at: 3,
        change,
    })
    .unwrap();
    for (n, unit, index) in [(4, 40, 0), (5, 50, 1)] {
        let c = cfg().units()[index].ledger.clone();
        let s = j.state().unwrap().unit(uid(unit)).unwrap();
        let l = s.ledger();
        let tx = Transaction {
            id: CommitId::new([n; 32]).unwrap(),
            expected: j.head(),
            at: u64::from(n),
            evidence: vec![],
            inputs: vec![Input {
                source: c.sources[0].scope,
                source_cut: Some(100),
                authority_epoch: 1,
                observed_at: u64::from(n),
                raw: raw(b"independently qualified zero-asset final check"),
                event: Some(Event {
                    key: RecordKey::Economic(event_key(index)),
                    policy: c.policy,
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
        assert_eq!(
            j.select(uid(unit))
                .unwrap()
                .commit(tx)
                .unwrap()
                .receipt
                .inputs,
            [InputResult::Normalized(Disposition::Applied)]
        );
    }
    let head = j.head();
    assert!(
        j.select(uid(50))
            .unwrap()
            .recovery_cut(head, event_key(0))
            .is_err()
    );
    let a = j
        .select(uid(40))
        .unwrap()
        .recovery_cut(head, event_key(0))
        .unwrap();
    let b = j
        .select(uid(50))
        .unwrap()
        .recovery_cut(head, event_key(1))
        .unwrap();
    assert_eq!(a.head(), b.head());
    assert_eq!(a.total().atoms(), 0);
    assert_eq!(b.total().atoms(), 0);
    commit(&mut j, 50, 6, vec![]);
    assert_eq!(
        j.select(uid(40))
            .unwrap()
            .recovery_cut(head, event_key(0))
            .unwrap_err(),
        recovery::CutError::Stale
    );
}
