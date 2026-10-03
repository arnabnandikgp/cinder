//! Synthetic native observations exercise the real journal/replay projection.
//! These are not service-to-SBF, live fencing, encrypted delivery or Nitro evidence.
mod support;
use cinder_journal::{collateral, funds, model::*, orders, recovery::CutError, sqlite::*, *};
use cinder_kernel::{
    identity::*,
    ledger::{
        economics::{EconomicChange, FundingRate},
        evidence::{Disposition, EvidencePolicy, MarkObservation, NativeCheck},
        funds::{Destination, FundsChange, Leg},
        *,
    },
    math::Rounding,
};
use std::{cell::Cell, rc::Rc};
use support::{FixtureProtection, Store, Temp, attempt, cash, p, q, raw, request, user};

fn config() -> Config {
    let mut c = support::config();
    for (n, location) in [(8, Location::Vault), (9, Location::Broker)] {
        let mut source = c.sources[0];
        source.scope.namespace = NamespaceId::new([n; 32]).unwrap();
        source.location = location;
        c.sources.push(source);
    }
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
fn receipt(n: u64, owner: Owner, location: Location, amount: i128) -> Event {
    event(
        n,
        location,
        Change::Receipt {
            owner,
            location,
            amount: cash(amount),
        },
    )
}
fn transaction(s: &Store, events: Vec<Event>, controls: Vec<Control>) -> Transaction {
    let n = u8::try_from(s.head().sequence + 1).unwrap();
    let mut tx = support::transaction(s.head(), n, events, controls);
    for i in &mut tx.inputs {
        if let Some(Event {
            key: RecordKey::Economic(k),
            ..
        }) = &i.event
        {
            i.source = k.scope;
        }
    }
    tx
}
fn commit(s: &mut Store, events: Vec<Event>, controls: Vec<Control>) -> Committed {
    let tx = transaction(s, events, controls);
    let result = s.commit(tx).unwrap();
    assert_eq!(result.receipt.controls, None);
    assert!(
        result
            .receipt
            .inputs
            .iter()
            .all(|r| *r == InputResult::Normalized(Disposition::Applied))
    );
    result
}
fn setup(t: &Temp) -> Store {
    setup_with_house(t, 100)
}
fn setup_with_house(t: &Temp, capital: i128) -> Store {
    let mut s = Journal::create(
        SqliteBackend::create(&t.db).unwrap(),
        FixtureProtection,
        config(),
    )
    .unwrap();
    commit(
        &mut s,
        vec![
            receipt(1, Owner::Customer(user(1)), Location::Vault, 100),
            receipt(2, Owner::Customer(user(2)), Location::Vault, 200),
            receipt(3, Owner::House, Location::Vault, capital),
        ],
        vec![Control::Order(orders::Action::AdvanceAuthority {
            account: user(1),
            epoch: 1,
        })],
    );
    s
}
fn check(s: &Store, n: u64) -> Event {
    let l = s.state().unwrap().ledger();
    event(
        n,
        Location::Venue,
        Change::Reconcile(NativeCheck {
            expected_version: l.version(),
            cash: Some(l.venue().cash()),
            funding: Some(l.venue().funding()),
            positions: Some(l.venue().positions().to_vec()),
            complete: true,
            resolves: vec![],
        }),
    )
}
fn freeze(s: &mut Store) {
    commit(s, vec![], vec![Control::Funds(funds::Action::Freeze)]);
}
fn final_check(s: &mut Store, n: u64) -> EventKey {
    let e = check(s, n);
    commit(s, vec![e], vec![]);
    key(n, Location::Venue)
}
fn reopen(t: &Temp) -> Store {
    Journal::open(
        SqliteBackend::open(&t.db, Migration::None).unwrap(),
        FixtureProtection,
        config(),
    )
    .unwrap()
}
fn bind(id: AttemptKey, side: Side) -> Event {
    Event {
        key: RecordKey::Attempt(id),
        policy: config().policy,
        change: Change::BindExecution {
            market: q(0).unit(),
            side,
        },
    }
}
fn fill(n: u64, target: FillTarget, quantity: i64, price: u64) -> Event {
    event(
        n,
        Location::Venue,
        Change::Fill {
            target,
            quantity: q(quantity),
            price: p(price),
        },
    )
}
fn collateral(s: &mut Store) {
    let version = s.state().unwrap().ledger().version();
    let e = check(s, 30);
    commit(
        s,
        vec![e],
        vec![Control::Collateral(collateral::Cut {
            expected_version: version + 1,
            policy: collateral::Policy {
                revision: config().policy,
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
                evidence: key(31, Location::Venue),
                observed_at: 10,
                valid_until: 100,
                qualified: true,
            }],
        })],
    );
}
fn accept_funds(s: &mut Store, prepare: bool) {
    collateral(s);
    let i = funds::Intent {
        request: request(10),
        source: Location::Vault,
        destination: Destination::Recipient([42; 32]),
        net: cash(20),
        maximum_fee: cash(2),
        fee_payer: Owner::Customer(user(1)),
        allow_partial: true,
        policy: config().policy,
        authority_epoch: 1,
        expires_at: 100,
    };
    let mut controls = vec![Control::Funds(funds::Action::Accept {
        approval: orders::Approval {
            account: user(1),
            authority_epoch: 1,
            intent_hash: i.digest().unwrap(),
        },
        intent: Box::new(i),
    })];
    if prepare {
        controls.extend([
            Control::Funds(funds::Action::Prepare {
                attempt: attempt(10),
                net: cash(20),
            }),
            Control::Expose(attempt(10)),
        ]);
    }
    commit(s, vec![], controls);
}
fn payment(s: &mut Store, finalize: bool) {
    accept_funds(s, true);
    commit(
        s,
        vec![
            event(
                40,
                Location::Vault,
                Change::Funds(FundsChange::Observe {
                    attempt: attempt(10),
                    leg: Leg::Debit,
                    amount: cash(22),
                    fee: cash(0),
                }),
            ),
            event(
                41,
                Location::Vault,
                Change::Funds(FundsChange::Observe {
                    attempt: attempt(10),
                    leg: Leg::Arrive(Destination::Recipient([42; 32])),
                    amount: cash(22),
                    fee: cash(2),
                }),
            ),
        ],
        vec![],
    );
    if finalize {
        let mut tx = transaction(
            s,
            vec![],
            vec![Control::Funds(funds::Action::Finalize(request(10)))],
        );
        tx.funds_observations.push(funds::Observation {
            key: key(42, Location::Vault),
            terminal: funds::Terminal {
                attempt: attempt(10),
                debit: cash(22),
                settled: cash(22),
                receipts: vec![key(40, Location::Vault), key(41, Location::Vault)],
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
            raw: raw(b"synthetic terminal funding capability"),
        });
        let result = s.commit(tx).unwrap();
        assert_eq!(result.receipt.controls, None);
        assert_eq!(result.receipt.funds_observations, [true]);
    }
}

#[test]
fn settled_cut_includes_all_customers_and_replays_without_mutation() {
    let t = Temp::new();
    let mut s = setup(&t);
    freeze(&mut s);
    let check = final_check(&mut s, 90);
    let before = s.state().unwrap().clone();
    let cut = s.recovery_cut(s.head(), check.clone()).unwrap();
    assert_eq!(cut.accounts().len(), 2);
    assert_eq!(cut.accounts()[0].account(), user(1));
    assert_eq!(cut.accounts()[1].cash(), cash(200));
    assert_eq!(cut.total(), cash(300));
    assert_eq!(cut.deficits(), cash(0));
    assert_eq!(cut.vault(), cash(400));
    assert_eq!(cut.house(), cash(100));
    assert_eq!(cut.suspense(), cash(0));
    assert_eq!(cut.configuration(), &config());
    assert_eq!(cut.check(), &check);
    assert_eq!(s.state().unwrap(), &before);
    assert_eq!(s.head(), cut.head());
    assert_eq!(format!("{cut:?}"), "RecoveryCut([PRIVATE])");
    assert_eq!(format!("{:?}", cut.accounts()[0]), "AccountCut([PRIVATE])");
    drop(s);
    let mut reopened = reopen(&t);
    assert_eq!(reopened.recovery_cut(reopened.head(), check).unwrap(), cut);
}

#[test]
fn ordinary_payment_and_rail_fee_are_not_subtracted_twice() {
    let t = Temp::new();
    let mut s = setup(&t);
    payment(&mut s, true);
    let tx = s
        .transaction(CommitId::new([4; 32]).unwrap())
        .unwrap()
        .clone();
    assert!(s.commit(tx).unwrap().duplicate);
    freeze(&mut s);
    let check = final_check(&mut s, 90);
    let cut = s.recovery_cut(s.head(), check.clone()).unwrap();
    assert_eq!(cut.accounts()[0].cash(), cash(78));
    assert_eq!(cut.accounts()[0].payable(), cash(78));
    assert_eq!(cut.accounts()[0].paid(), cash(20));
    assert_eq!(cut.total(), cash(278));
    assert_eq!(cut.vault(), cash(378));
    drop(s);
    let mut reopened = reopen(&t);
    assert_eq!(reopened.recovery_cut(reopened.head(), check).unwrap(), cut);
}

#[test]
fn durably_frozen_and_exact_reviewed_head_are_required() {
    let t = Temp::new();
    let mut s = setup(&t);
    let check = final_check(&mut s, 90);
    assert_eq!(
        s.recovery_cut(s.head(), check.clone()),
        Err(CutError::NotFrozen)
    );
    let old = s.head();
    freeze(&mut s);
    assert_eq!(s.recovery_cut(old, check.clone()), Err(CutError::Stale));
    assert!(s.recovery_cut(s.head(), check).is_ok());
    assert_eq!(
        s.recovery_cut(s.head(), key(91, Location::Venue)),
        Err(CutError::IncompleteCheck)
    );
}

#[test]
fn late_deposit_requires_new_reconciliation_and_cannot_mutate_old_cut() {
    let t = Temp::new();
    let mut s = setup(&t);
    freeze(&mut s);
    let check = final_check(&mut s, 90);
    let old = s.recovery_cut(s.head(), check.clone()).unwrap();
    commit(
        &mut s,
        vec![receipt(91, Owner::Customer(user(1)), Location::Vault, 1)],
        vec![],
    );
    assert_eq!(
        s.recovery_cut(old.head(), check.clone()),
        Err(CutError::Stale)
    );
    assert_eq!(
        s.recovery_cut(s.head(), check),
        Err(CutError::IncompleteCheck)
    );
    assert_eq!(old.total(), cash(300));
    let check = final_check(&mut s, 92);
    assert_eq!(s.recovery_cut(s.head(), check).unwrap().total(), cash(301));
}

#[test]
fn causal_source_coverage_is_not_inferred_from_a_matching_snapshot() {
    let t = Temp::new();
    let mut s = setup(&t);
    freeze(&mut s);
    let e = check(&s, 90);
    let mut tx = transaction(&s, vec![e], vec![]);
    tx.inputs[0].source_cut = None;
    assert_eq!(
        s.commit(tx).unwrap().receipt.inputs,
        [InputResult::Normalized(Disposition::Applied)]
    );
    assert_eq!(
        s.recovery_cut(s.head(), key(90, Location::Venue)),
        Err(CutError::IncompleteCheck)
    );
}

#[test]
fn net_zero_is_not_gross_private_settlement() {
    let t = Temp::new();
    let mut s = setup(&t);
    let mut short = attempt(20);
    short.request.account = user(2);
    commit(
        &mut s,
        vec![
            bind(attempt(19), Side::Buy),
            bind(short, Side::Sell),
            fill(19, FillTarget::Customer(attempt(19)), 1, 100),
            fill(20, FillTarget::Customer(short), -1, 100),
        ],
        vec![],
    );
    assert_eq!(
        s.state().unwrap().ledger().venue().positions()[0].quantity(),
        q(0)
    );
    freeze(&mut s);
    let check = final_check(&mut s, 90);
    assert_eq!(s.recovery_cut(s.head(), check), Err(CutError::NotSettled));
}

#[test]
fn zero_net_funding_still_requires_each_boundary_to_settle() {
    let t = Temp::new();
    let mut s = setup(&t);
    let version = s.state().unwrap().ledger().version();
    commit(
        &mut s,
        vec![
            event(
                50,
                Location::Venue,
                Change::Economics(EconomicChange::FundingBoundary {
                    market: q(0).unit(),
                    expected_version: version,
                }),
            ),
            event(
                51,
                Location::Venue,
                Change::Economics(EconomicChange::FundingInputs {
                    boundary: key(50, Location::Venue),
                    rate: Some(FundingRate {
                        numerator: cash(0),
                        denominator: 1,
                        rounding: Rounding::TowardZero,
                    }),
                    native: Some(cash(0)),
                }),
            ),
        ],
        vec![],
    );
    freeze(&mut s);
    let check = final_check(&mut s, 90);
    assert_eq!(s.state().unwrap().ledger().venue().funding(), cash(0));
    assert_eq!(s.recovery_cut(s.head(), check), Err(CutError::NotSettled));
    commit(
        &mut s,
        vec![event(
            52,
            Location::Venue,
            Change::Economics(EconomicChange::FundingSettlement {
                boundary: key(50, Location::Venue),
                native: cash(0),
            }),
        )],
        vec![],
    );
    let check = final_check(&mut s, 91);
    assert!(s.recovery_cut(s.head(), check).is_ok());
}

#[test]
fn assets_outside_the_vault_are_not_recovery_cash() {
    for location in [Location::Venue, Location::Broker] {
        let t = Temp::new();
        let mut s = setup(&t);
        commit(&mut s, vec![receipt(60, Owner::House, location, 1)], vec![]);
        freeze(&mut s);
        let check = final_check(&mut s, 90);
        assert_eq!(
            s.recovery_cut(s.head(), check),
            Err(CutError::AssetsNotReturned)
        );
    }
    let t = Temp::new();
    let mut s = setup(&t);
    commit(
        &mut s,
        vec![event(
            60,
            Location::Vault,
            Change::TransferDebit {
                source: Location::Vault,
                destination: Location::Venue,
                amount: cash(1),
            },
        )],
        vec![],
    );
    freeze(&mut s);
    let check = final_check(&mut s, 90);
    assert_eq!(
        s.recovery_cut(s.head(), check),
        Err(CutError::AssetsNotReturned)
    );
}

#[test]
fn queued_or_unfinalized_funds_cannot_be_treated_as_completed() {
    for exposed in [false, true] {
        let t = Temp::new();
        let mut s = setup(&t);
        if exposed {
            payment(&mut s, false);
        } else {
            accept_funds(&mut s, false);
        }
        freeze(&mut s);
        let check = final_check(&mut s, 90);
        assert_eq!(
            s.recovery_cut(s.head(), check),
            Err(CutError::OpenCommitments)
        );
    }
}

#[test]
fn released_generic_attempt_does_not_supply_native_closure_evidence() {
    let t = Temp::new();
    let mut s = setup(&t);
    commit(
        &mut s,
        vec![],
        vec![
            Control::Reserve {
                request: request(20),
                reservations: vec![Reservation {
                    resource: Resource::Customer(user(1)),
                    amount: cash(1),
                }],
            },
            support::prepare(20),
        ],
    );
    commit(&mut s, vec![], vec![Control::Release(request(20))]);
    freeze(&mut s);
    let check = final_check(&mut s, 90);
    assert_eq!(
        s.recovery_cut(s.head(), check),
        Err(CutError::OpenCommitments)
    );
}

#[test]
fn raw_and_native_reconciliation_gaps_are_not_zeroes() {
    for raw_gap in [false, true] {
        let t = Temp::new();
        let mut s = setup(&t);
        freeze(&mut s);
        let mut e = check(&s, 90);
        if !raw_gap {
            let Change::Reconcile(c) = &mut e.change else {
                panic!()
            };
            c.complete = false;
        }
        let mut tx = transaction(&s, vec![e], vec![]);
        if raw_gap {
            tx.inputs[0].event = None;
        }
        s.commit(tx).unwrap();
        assert_eq!(
            s.recovery_cut(s.head(), key(90, Location::Venue)),
            Err(CutError::UnresolvedEvidence)
        );
    }
}

#[test]
fn incurred_remediation_is_not_erased_when_vault_backing_is_insufficient() {
    for house in [false, true] {
        let t = Temp::new();
        let mut s = setup(&t);
        // A separately owed remediation overdraws house capital, creating an
        // underbacked positive claim without erasing the original obligation.
        commit(
            &mut s,
            vec![Event {
                key: RecordKey::Request(request(70)),
                policy: config().policy,
                change: Change::Protection(
                    cinder_kernel::ledger::protection::ProtectionChange::Recognize {
                        kind: cinder_kernel::ledger::protection::Kind::Remediation,
                        cause: [7; 32],
                        amount: cash(101),
                    },
                ),
            }],
            vec![],
        );
        if house {
            commit(
                &mut s,
                vec![receipt(71, Owner::House, Location::Vault, 1)],
                vec![],
            );
        }
        freeze(&mut s);
        let check = final_check(&mut s, 90);
        if house {
            let cut = s.recovery_cut(s.head(), check).unwrap();
            assert_eq!(cut.total(), cash(401));
            assert_eq!(cut.vault(), cash(401));
        } else {
            assert_eq!(s.recovery_cut(s.head(), check), Err(CutError::Unbacked));
            assert_eq!(
                s.state()
                    .unwrap()
                    .ledger()
                    .book(Owner::Customer(user(1)))
                    .unwrap()
                    .cash(),
                cash(201)
            );
        }
    }
}

#[test]
fn negative_customer_debt_never_offsets_another_customers_positive_claim() {
    for capital in [50, 100] {
        let t = Temp::new();
        let mut s = setup_with_house(&t, capital);
        let mut short = attempt(20);
        short.request.account = user(2);
        let mut short_close = attempt(22);
        short_close.request.account = user(2);
        commit(
            &mut s,
            vec![
                bind(attempt(19), Side::Buy),
                bind(short, Side::Sell),
                bind(attempt(21), Side::Sell),
                bind(short_close, Side::Buy),
                fill(19, FillTarget::Customer(attempt(19)), 2, 100),
                fill(20, FillTarget::Customer(short), -2, 100),
                fill(21, FillTarget::Customer(attempt(21)), -2, 1),
                fill(22, FillTarget::Customer(short_close), 2, 1),
            ],
            vec![],
        );
        freeze(&mut s);
        let check = final_check(&mut s, 90);
        assert_eq!(
            s.state()
                .unwrap()
                .ledger()
                .book(Owner::Customer(user(1)))
                .unwrap()
                .cash(),
            cash(-98)
        );
        if capital == 50 {
            assert_eq!(s.recovery_cut(s.head(), check), Err(CutError::Unbacked));
        } else {
            let cut = s.recovery_cut(s.head(), check).unwrap();
            assert_eq!(cut.accounts()[0].cash(), cash(-98));
            assert_eq!(cut.accounts()[0].payable(), cash(0));
            assert_eq!(cut.accounts()[1].payable(), cash(398));
            assert_eq!(cut.total(), cash(398));
            assert_eq!(cut.deficits(), cash(98));
            assert_eq!(cut.vault(), cash(400));
        }
    }
}

#[test]
fn zero_accounts_cannot_be_omitted_from_the_configured_inventory() {
    let t = Temp::new();
    let mut c = config();
    c.customers.push(user(3));
    let mut s =
        Journal::create(SqliteBackend::create(&t.db).unwrap(), FixtureProtection, c).unwrap();
    commit(
        &mut s,
        vec![receipt(1, Owner::Customer(user(1)), Location::Vault, 100)],
        vec![],
    );
    freeze(&mut s);
    let check = final_check(&mut s, 90);
    let cut = s.recovery_cut(s.head(), check).unwrap();
    assert_eq!(cut.accounts().len(), 3);
    assert_eq!(cut.accounts()[1].payable(), cash(0));
    assert_eq!(cut.accounts()[2].account(), user(3));
    assert_eq!(cut.accounts()[2].paid(), cash(0));
    assert_eq!(cut.total(), cash(100));
}

#[test]
fn unattributed_deposits_do_not_become_an_extra_customer_or_house_credit() {
    let t = Temp::new();
    let mut s = setup(&t);
    commit(
        &mut s,
        vec![receipt(80, Owner::Suspense, Location::Vault, 1)],
        vec![],
    );
    freeze(&mut s);
    let check = final_check(&mut s, 90);
    assert_eq!(
        s.recovery_cut(s.head(), check),
        Err(CutError::UnresolvedEvidence)
    );
}

// Explicit failure port, not an independent rollback witness qualification.
struct RejectCurrent {
    inner: SqliteBackend,
    reject: Rc<Cell<bool>>,
}
impl Backend for RejectCurrent {
    fn load(&mut self) -> Result<Vec<Frame>, Error> {
        self.inner.load()
    }
    fn append(&mut self, expected: Option<Head>, frame: &Frame) -> Result<(), Error> {
        self.inner.append(expected, frame)
    }
    fn check_current(&mut self, _: Head) -> Result<(), Error> {
        if self.reject.get() {
            Err(Error::Stale)
        } else {
            Ok(())
        }
    }
}

#[test]
fn lost_freshness_authority_blocks_cut_and_poisons_cached_state() {
    let t = Temp::new();
    let mut ordinary = setup(&t);
    freeze(&mut ordinary);
    let check = final_check(&mut ordinary, 90);
    drop(ordinary);
    let reject = Rc::new(Cell::new(false));
    let mut s = Journal::open(
        RejectCurrent {
            inner: SqliteBackend::open(&t.db, Migration::None).unwrap(),
            reject: reject.clone(),
        },
        FixtureProtection,
        config(),
    )
    .unwrap();
    assert!(s.recovery_cut(s.head(), check.clone()).is_ok());
    reject.set(true);
    assert_eq!(
        s.recovery_cut(s.head(), check.clone()),
        Err(CutError::Journal(Error::Stale))
    );
    reject.set(false);
    assert_eq!(
        s.recovery_cut(s.head(), check),
        Err(CutError::Journal(Error::Poisoned))
    );
}
