//! Synthetic offline unit composition. No native credentials, network or funds.
mod support;
use cinder_journal::{
    Backend, Error, Frame, Head, Journal as Legacy, Protection, RecordContext,
    encrypted::RecordCipher,
    model::*,
    onboarding, orders,
    sqlite::{Migration, SqliteBackend},
    units::{self, Change, Configuration, Descriptor, Recipient},
};
use cinder_kernel::{
    amounts::QuoteAtoms,
    identity::*,
    ledger::{Change as EconomicChange, *},
};
use std::sync::{Arc, Mutex};
use support::{FixtureProtection, Temp, raw};
use zeroize::Zeroizing;

type Store = units::Journal<SqliteBackend, FixtureProtection>;
fn uid(n: u8) -> UnitId {
    UnitId::new([n; 32]).unwrap()
}
fn descriptors() -> Vec<Descriptor> {
    [40_u8, 50]
        .into_iter()
        .map(|n| {
            let mut ledger = support::config();
            ledger.customers.clear();
            ledger.venue = VenueId::new([n + 4; 32]).unwrap();
            ledger.venue_account = VenueAccountId::new([n + 5; 32]).unwrap();
            ledger.sources[0].scope.venue = ledger.venue;
            ledger.sources[0].scope.account = ledger.venue_account;
            Descriptor {
                id: uid(n),
                environment: [90; 32],
                native_account: [n + 1; 32],
                funding_wallet: [n + 2; 32],
                custody_config: [n + 3; 32],
                payout_vault: [n + 6; 32],
                key_policy: [n + 7; 32],
                ledger,
            }
        })
        .collect()
}
fn configuration() -> Configuration {
    Configuration::new(support::config().domain, descriptors()).unwrap()
}
fn create(t: &Temp) -> Store {
    Store::create(
        SqliteBackend::create(&t.db).unwrap(),
        FixtureProtection,
        configuration(),
    )
    .unwrap()
}
fn open(t: &Temp) -> Store {
    Store::open(
        SqliteBackend::open(&t.db, Migration::None).unwrap(),
        FixtureProtection,
        configuration(),
    )
    .unwrap()
}
fn owner(n: u8) -> AccountId {
    onboarding::account(support::config().domain, [n; 32]).unwrap()
}
fn global(head: Head, n: u8, change: Change) -> units::Transaction {
    units::Transaction {
        id: CommitId::new([n; 32]).unwrap(),
        expected: head,
        at: u64::from(n),
        change,
    }
}
fn enrollment(head: Head, n: u8, wallet: u8) -> units::Transaction {
    global(
        head,
        n,
        Change::Enroll {
            wallet: [wallet; 32],
            recipients: vec![
                Recipient {
                    unit: uid(40),
                    tokens: [wallet + 1; 32],
                },
                Recipient {
                    unit: uid(50),
                    tokens: [wallet + 2; 32],
                },
            ],
        },
    )
}
fn proposal(
    head: Head,
    n: u8,
    inputs: Vec<Input>,
    controls: Vec<Control>,
) -> cinder_journal::model::Transaction {
    let mut tx = support::transaction(head, n, vec![], controls);
    tx.at = u64::from(n);
    tx.inputs = inputs;
    tx
}
fn unit(
    head: Head,
    n: u8,
    which: u8,
    inputs: Vec<Input>,
    controls: Vec<Control>,
) -> units::Transaction {
    units::Transaction::unit(uid(which), proposal(head, n, inputs, controls))
}
fn credit(which: u8, customer: AccountId, number: u64, amount: i128) -> Input {
    let c = descriptors()
        .into_iter()
        .find(|d| d.id == uid(which))
        .unwrap()
        .ledger;
    Input {
        source: c.sources[0].scope,
        source_cut: Some(number),
        authority_epoch: 1,
        observed_at: 1,
        raw: raw(b"synthetic-qualified-receipt"),
        event: Some(Event {
            key: RecordKey::Economic(EventKey {
                scope: c.sources[0].scope,
                event: EconomicEventId::new(&number.to_be_bytes()).unwrap(),
                leg: 0,
            }),
            policy: c.policy,
            change: EconomicChange::Receipt {
                owner: Owner::Customer(customer),
                location: Location::Venue,
                amount: QuoteAtoms::new(c.quote, amount),
            },
        }),
    }
}
fn request(n: u8) -> RequestKey {
    RequestKey {
        domain: support::config().domain,
        account: owner(70),
        request: RequestId::new([n; 32]).unwrap(),
    }
}
fn reserve(n: u8, amount: i128) -> Control {
    Control::Reserve {
        request: request(n),
        reservations: vec![
            Reservation {
                resource: Resource::Customer(owner(70)),
                amount: support::cash(amount),
            },
            Reservation {
                resource: Resource::Location(Location::Venue),
                amount: support::cash(amount),
            },
        ],
    }
}
fn attempt(n: u8) -> AttemptKey {
    AttemptKey {
        request: request(n),
        attempt: AttemptId::new([n; 32]).unwrap(),
    }
}
fn prepare(n: u8, epoch: u64) -> Control {
    Control::Prepare {
        key: attempt(n),
        message: raw(b"foundation-only-unsigned-action"),
        authority_epoch: epoch,
        expires_at: 1000,
    }
}
fn outcome(receipt: &units::Receipt) -> &Receipt {
    let units::Receipt::Unit { receipt, .. } = receipt else {
        panic!("expected unit receipt")
    };
    receipt
}
fn cash(s: &units::State, which: u8) -> i128 {
    s.unit(uid(which))
        .unwrap()
        .ledger()
        .book(Owner::Customer(owner(70)))
        .unwrap()
        .cash()
        .atoms()
}
fn seeded(t: &Temp) -> Store {
    let mut s = create(t);
    s.commit(enrollment(s.head(), 1, 70)).unwrap();
    s.commit(unit(
        s.head(),
        2,
        40,
        vec![credit(40, owner(70), 1, 100)],
        vec![],
    ))
    .unwrap();
    s.commit(unit(
        s.head(),
        3,
        50,
        vec![credit(50, owner(70), 1, 200)],
        vec![],
    ))
    .unwrap();
    s
}

#[test]
fn registry_is_canonical_empty_and_rejects_ambiguous_physical_ownership() {
    assert!(UnitId::new([0; 32]).is_err());
    let domain = support::config().domain;
    let mut reversed = descriptors();
    reversed.reverse();
    assert_eq!(
        Configuration::new(domain, reversed).unwrap(),
        configuration()
    );
    assert_eq!(
        Configuration::new(domain, vec![]).unwrap_err(),
        Error::Limit
    );
    let mut too_many = descriptors();
    too_many.push(too_many[0].clone());
    assert_eq!(
        Configuration::new(domain, too_many).unwrap_err(),
        Error::Limit
    );
    for field in 0..11 {
        let mut d = descriptors();
        match field {
            0 => d[1].id = d[0].id,
            1 => d[1].funding_wallet = d[0].funding_wallet,
            2 => d[1].payout_vault = d[0].payout_vault,
            3 => d[1].custody_config = d[0].custody_config,
            4 => d[1].key_policy = d[0].key_policy,
            5 => d[1].funding_wallet = d[0].payout_vault,
            6 => {
                // Namespace aliases cannot duplicate a physical account.
                d[1].ledger.venue = d[0].ledger.venue;
                d[1].ledger.venue_account = d[0].ledger.venue_account;
                d[1].ledger.sources[0].scope = d[0].ledger.sources[0].scope;
                d[1].ledger.sources[0].scope.namespace = NamespaceId::new([99; 32]).unwrap();
            }
            7 => {
                d[1].native_account = d[0].native_account;
                d[1].ledger.venue = d[0].ledger.venue;
                d[1].ledger.sources[0].scope.venue = d[0].ledger.venue;
            }
            8 => d[1].ledger.customers.push(owner(70)),
            9 => d[1].funding_wallet = [0; 32],
            10 => d[1].ledger.domain.deployment = DeploymentId::new([99; 32]).unwrap(),
            _ => unreachable!(),
        }
        assert!(Configuration::new(domain, d).is_err(), "field {field}");
    }
}

#[test]
fn each_unit_keeps_key_controlled_funding_separate_from_program_payouts() {
    let domain = support::config().domain;
    // Include the first/only descriptor: inter-unit checks cannot cover it.
    for count in 1..=2 {
        for index in 0..count {
            let mut units = descriptors();
            units.truncate(count);
            units[index].payout_vault = units[index].funding_wallet;
            assert_eq!(
                Configuration::new(domain, units).unwrap_err(),
                Error::Invalid
            );
        }
    }
    // A venue identity may legitimately be the funding wallet, not the payout PDA.
    let mut units = descriptors();
    units[0].native_account = units[0].funding_wallet;
    assert!(Configuration::new(domain, units).is_ok());
}

#[test]
fn enrollment_is_atomic_asset_specific_and_replay_derived() {
    let t = Temp::new();
    let mut s = create(&t);
    let head = s.head();
    let mut bad = enrollment(head, 1, 70);
    let Change::Enroll { recipients, .. } = &mut bad.change else {
        unreachable!()
    };
    recipients[1].tokens = [0; 32];
    assert_eq!(s.commit(bad).unwrap_err(), Error::Invalid);
    assert_eq!(s.head(), head);
    assert!(s.state().unwrap().customers().is_empty());
    assert!(
        s.state()
            .unwrap()
            .unit(uid(40))
            .unwrap()
            .registrations()
            .is_empty()
    );
    let tx = enrollment(head, 1, 70);
    let accepted = s.commit(tx.clone()).unwrap();
    assert_eq!(accepted.receipt, units::Receipt::Shared);
    assert!(s.commit(tx.clone()).unwrap().duplicate);
    assert_eq!(s.state().unwrap().customers()[0].epoch, 1);
    for (which, tokens) in [(40, 71), (50, 72)] {
        let component = s.state().unwrap().unit(uid(which)).unwrap();
        assert_eq!(component.authority_epoch(owner(70)), Some(1));
        assert_eq!(component.registrations()[0].tokens, [tokens; 32]);
        assert_eq!(cash(s.state().unwrap(), which), 0);
    }
    let before = s.state().unwrap().clone();
    drop(s);
    let mut reopened = open(&t);
    assert_eq!(reopened.state().unwrap(), &before);
    assert!(reopened.commit(tx).unwrap().duplicate);
    assert_eq!(
        reopened
            .commit(enrollment(reopened.head(), 2, 70))
            .unwrap_err(),
        Error::Invalid
    );
}

#[test]
fn same_owner_market_and_native_id_have_independent_assets_and_once_only_replay() {
    let t = Temp::new();
    let mut s = seeded(&t);
    assert_eq!(s.head().sequence, 3);
    assert_eq!(
        (cash(s.state().unwrap(), 40), cash(s.state().unwrap(), 50)),
        (100, 200)
    );
    let b = s.state().unwrap().unit(uid(50)).unwrap().clone();
    let duplicate = s
        .commit(unit(
            s.head(),
            4,
            40,
            vec![credit(40, owner(70), 1, 100)],
            vec![],
        ))
        .unwrap();
    assert_eq!(
        outcome(&duplicate.receipt).inputs,
        vec![InputResult::Normalized(evidence::Disposition::Duplicate)]
    );
    assert_eq!(cash(s.state().unwrap(), 40), 100);
    assert_eq!(s.state().unwrap().unit(uid(50)).unwrap(), &b);
    let before = s.state().unwrap().clone();
    let head = s.head();
    drop(s);
    let s = open(&t);
    assert_eq!(s.head(), head);
    assert_eq!(s.state().unwrap(), &before);
}

#[test]
fn foreign_source_is_retained_locally_and_cannot_credit_another_unit() {
    let t = Temp::new();
    let mut s = seeded(&t);
    let b = s.state().unwrap().unit(uid(50)).unwrap().clone();
    let result = s
        .commit(unit(
            s.head(),
            4,
            40,
            vec![credit(50, owner(70), 2, 90)],
            vec![],
        ))
        .unwrap();
    assert_eq!(
        outcome(&result.receipt).inputs,
        vec![InputResult::EnvelopeRejected]
    );
    assert_eq!(
        s.state().unwrap().unit(uid(40)).unwrap().unresolved_raw(),
        1
    );
    assert_eq!(s.state().unwrap().unit(uid(50)).unwrap(), &b);
    assert_eq!(cash(s.state().unwrap(), 40), 100);
    // A's containment does not freeze a distinct B reservation.
    let r = s
        .commit(unit(s.head(), 5, 50, vec![], vec![reserve(9, 100)]))
        .unwrap();
    assert_eq!(outcome(&r.receipt).controls, None);
    let before = s.state().unwrap().clone();
    drop(s);
    assert_eq!(open(&t).state().unwrap(), &before);
}

#[test]
fn native_attempt_and_fill_references_cannot_bypass_deployment_request_ownership() {
    let t = Temp::new();
    let mut s = seeded(&t);
    s.commit(unit(
        s.head(),
        4,
        40,
        vec![],
        vec![reserve(9, 20), prepare(9, 1)],
    ))
    .unwrap();
    let binding = |which| {
        let mut input = credit(which, owner(70), 2, 1);
        input.event = Some(Event {
            key: RecordKey::Attempt(attempt(9)),
            policy: support::config().policy,
            change: EconomicChange::BindExecution {
                market: support::q(1).unit(),
                side: Side::Buy,
            },
        });
        input
    };
    let b = s.state().unwrap().unit(uid(50)).unwrap().ledger().clone();
    let wrong = s
        .commit(unit(s.head(), 5, 50, vec![binding(50)], vec![]))
        .unwrap();
    assert_eq!(
        outcome(&wrong.receipt).inputs,
        vec![InputResult::EnvelopeRejected]
    );
    assert_eq!(s.state().unwrap().unit(uid(50)).unwrap().ledger(), &b);
    let accepted = s
        .commit(unit(s.head(), 6, 40, vec![binding(40)], vec![]))
        .unwrap();
    assert_eq!(
        outcome(&accepted.receipt).inputs,
        vec![InputResult::Normalized(evidence::Disposition::Applied)]
    );
    // Current admission may be unavailable, but a real late fill remains recordable.
    let fill = |which| {
        let mut input = credit(which, owner(70), 3, 1);
        input.event.as_mut().unwrap().change = EconomicChange::Fill {
            target: FillTarget::Customer(attempt(9)),
            quantity: support::q(1),
            price: support::p(100),
        };
        input
    };
    let wrong = s
        .commit(unit(s.head(), 7, 50, vec![fill(50)], vec![]))
        .unwrap();
    assert_eq!(
        outcome(&wrong.receipt).inputs,
        vec![InputResult::EnvelopeRejected]
    );
    let late = s
        .commit(unit(
            s.head(),
            8,
            40,
            vec![fill(40)],
            vec![Control::Expose(attempt(9))],
        ))
        .unwrap();
    assert_eq!(
        outcome(&late.receipt).inputs,
        vec![InputResult::Normalized(evidence::Disposition::Applied)]
    );
    assert_eq!(outcome(&late.receipt).controls, Some(ControlError::Invalid));
    assert_eq!(s.state().unwrap().unit(uid(50)).unwrap().ledger(), &b);
    let before = s.state().unwrap().clone();
    drop(s);
    assert_eq!(open(&t).state().unwrap(), &before);
}

#[test]
fn different_quote_assets_keep_recipients_and_reservation_precision_separate() {
    let t = Temp::new();
    let mut descriptors = descriptors();
    let quote = cinder_kernel::amounts::AssetUnit {
        asset: AssetId::new([99; 32]).unwrap(),
        precision: PrecisionVersion::new(2).unwrap(),
    };
    descriptors[1].ledger.quote = quote;
    let mut market = descriptors[1].ledger.markets[0].unit();
    market.quote = quote;
    descriptors[1].ledger.markets[0] = cinder_kernel::position::Market::new(market, 1, 1).unwrap();
    let config = Configuration::new(support::config().domain, descriptors).unwrap();
    let mut s = Store::create(
        SqliteBackend::create(&t.db).unwrap(),
        FixtureProtection,
        config.clone(),
    )
    .unwrap();
    s.commit(enrollment(s.head(), 1, 70)).unwrap();
    let mut input = credit(50, owner(70), 1, 200);
    let EconomicChange::Receipt { amount, .. } = &mut input.event.as_mut().unwrap().change else {
        unreachable!()
    };
    *amount = QuoteAtoms::new(quote, 200);
    s.commit(unit(s.head(), 2, 50, vec![input], vec![]))
        .unwrap();
    let wrong = s
        .commit(unit(s.head(), 3, 50, vec![], vec![reserve(9, 20)]))
        .unwrap();
    assert!(outcome(&wrong.receipt).controls.is_some());
    assert!(s.state().unwrap().route(request(9)).is_none());
    assert_eq!(cash(s.state().unwrap(), 40), 0);
    assert_eq!(
        s.state()
            .unwrap()
            .unit(uid(50))
            .unwrap()
            .ledger()
            .book(Owner::Customer(owner(70)))
            .unwrap()
            .cash(),
        QuoteAtoms::new(quote, 200)
    );
    let before = s.state().unwrap().clone();
    drop(s);
    let reopened = Store::open(
        SqliteBackend::open(&t.db, Migration::None).unwrap(),
        FixtureProtection,
        config,
    )
    .unwrap();
    assert_eq!(reopened.state().unwrap(), &before);
}

#[test]
fn reservation_route_digest_and_attempt_survive_release_and_restart() {
    let t = Temp::new();
    let mut s = seeded(&t);
    let tx = unit(s.head(), 4, 40, vec![], vec![reserve(9, 80), prepare(9, 1)]);
    assert_eq!(
        outcome(&s.commit(tx.clone()).unwrap().receipt).controls,
        None
    );
    let route = s.state().unwrap().route(request(9)).unwrap().clone();
    let a = s.state().unwrap().unit(uid(40)).unwrap();
    assert!(!a.attempts()[0].possibly_exposed);
    assert_eq!(a.holds().len(), 1);
    assert!(s.commit(tx).unwrap().duplicate);
    let wrong = s
        .commit(unit(s.head(), 5, 50, vec![], vec![reserve(9, 80)]))
        .unwrap();
    assert_eq!(
        outcome(&wrong.receipt).controls,
        Some(ControlError::Invalid)
    );
    assert!(s.state().unwrap().unit(uid(50)).unwrap().holds().is_empty());
    let changed = s
        .commit(unit(s.head(), 6, 40, vec![], vec![reserve(9, 81)]))
        .unwrap();
    assert_eq!(
        outcome(&changed.receipt).controls,
        Some(ControlError::Invalid)
    );
    let release = s
        .commit(unit(
            s.head(),
            7,
            40,
            vec![],
            vec![Control::Release(request(9))],
        ))
        .unwrap();
    assert_eq!(outcome(&release.receipt).controls, None);
    assert!(!s.state().unwrap().unit(uid(40)).unwrap().holds()[0].active);
    drop(s);
    let mut s = open(&t);
    assert_eq!(s.state().unwrap().route(request(9)), Some(&route));
    let wrong = s
        .commit(unit(s.head(), 8, 50, vec![], vec![prepare(9, 1)]))
        .unwrap();
    assert_eq!(
        outcome(&wrong.receipt).controls,
        Some(ControlError::Invalid)
    );
}

#[test]
fn failed_controls_never_pin_a_route_or_discard_observations() {
    let t = Temp::new();
    let mut s = seeded(&t);
    // Insufficient A capacity cannot borrow B's 200 or leave a phantom route.
    let r = s
        .commit(unit(s.head(), 4, 40, vec![], vec![reserve(9, 150)]))
        .unwrap();
    assert_eq!(outcome(&r.receipt).controls, Some(ControlError::Capacity));
    assert!(s.state().unwrap().route(request(9)).is_none());
    let b = s.state().unwrap().unit(uid(50)).unwrap().clone();
    // Direct epoch change would bypass shared revocation; facts still commit.
    let r = s
        .commit(unit(
            s.head(),
            5,
            40,
            vec![credit(40, owner(70), 2, 10)],
            vec![
                reserve(10, 20),
                Control::Order(orders::Action::AdvanceAuthority {
                    account: owner(70),
                    epoch: 8,
                }),
            ],
        ))
        .unwrap();
    assert_eq!(outcome(&r.receipt).controls, Some(ControlError::Invalid));
    assert_eq!(cash(s.state().unwrap(), 40), 110);
    assert!(s.state().unwrap().route(request(10)).is_none());
    assert_eq!(
        s.state()
            .unwrap()
            .unit(uid(40))
            .unwrap()
            .authority_epoch(owner(70)),
        Some(1)
    );
    assert_eq!(s.state().unwrap().unit(uid(50)).unwrap(), &b);
}

#[test]
fn shared_revocation_applies_despite_local_containment_and_never_exposes() {
    let t = Temp::new();
    let mut s = seeded(&t);
    s.commit(unit(
        s.head(),
        4,
        40,
        vec![],
        vec![reserve(9, 20), prepare(9, 1)],
    ))
    .unwrap();
    let mut unknown = credit(40, owner(70), 2, 10);
    unknown.event = None;
    s.commit(unit(s.head(), 5, 40, vec![unknown], vec![]))
        .unwrap();
    let tx = global(
        s.head(),
        6,
        Change::AdvanceAuthority {
            account: owner(70),
            epoch: 2,
        },
    );
    s.commit(tx.clone()).unwrap();
    assert!(s.commit(tx).unwrap().duplicate);
    for which in [40, 50] {
        assert_eq!(
            s.state()
                .unwrap()
                .unit(uid(which))
                .unwrap()
                .authority_epoch(owner(70)),
            Some(2)
        );
    }
    let old = s
        .commit(unit(
            s.head(),
            7,
            50,
            vec![],
            vec![reserve(10, 20), prepare(10, 1)],
        ))
        .unwrap();
    assert_eq!(outcome(&old.receipt).controls, Some(ControlError::Invalid));
    assert!(s.state().unwrap().route(request(10)).is_none());
    let new = s
        .commit(unit(
            s.head(),
            8,
            50,
            vec![],
            vec![reserve(10, 20), prepare(10, 2)],
        ))
        .unwrap();
    assert_eq!(outcome(&new.receipt).controls, None);
    let exposed = s
        .commit(unit(
            s.head(),
            9,
            50,
            vec![],
            vec![Control::Expose(attempt(10))],
        ))
        .unwrap();
    assert_eq!(
        outcome(&exposed.receipt).controls,
        Some(ControlError::Invalid)
    );
    assert!(!s.state().unwrap().unit(uid(50)).unwrap().attempts()[0].possibly_exposed);
    assert_eq!(
        s.commit(global(
            s.head(),
            10,
            Change::AdvanceAuthority {
                account: owner(70),
                epoch: 2
            }
        ))
        .unwrap_err(),
        Error::Invalid
    );
    let before = s.state().unwrap().clone();
    drop(s);
    assert_eq!(open(&t).state().unwrap(), &before);
}

#[test]
fn headers_unknown_units_regression_and_commit_id_conflicts_cannot_retarget() {
    let t = Temp::new();
    let mut s = seeded(&t);
    let head = s.head();
    let mut bad = unit(head, 4, 40, vec![], vec![]);
    bad.at = 5;
    assert_eq!(s.commit(bad).unwrap_err(), Error::Invalid);
    assert_eq!(
        s.commit(unit(head, 4, 99, vec![], vec![])).unwrap_err(),
        Error::Invalid
    );
    assert_eq!(
        s.commit(unit(head, 2, 40, vec![], vec![])).unwrap_err(),
        Error::Conflict
    );
    let mut regressed = unit(head, 4, 40, vec![], vec![]);
    regressed.at = 0;
    let Change::Unit { proposal, .. } = &mut regressed.change else {
        unreachable!()
    };
    proposal.at = 0;
    assert_eq!(s.commit(regressed).unwrap_err(), Error::Invalid);
    let tx = unit(head, 4, 40, vec![], vec![]);
    s.commit(tx.clone()).unwrap();
    let mut changed = tx.clone();
    let Change::Unit { unit, .. } = &mut changed.change else {
        unreachable!()
    };
    *unit = uid(50);
    assert_eq!(s.commit(changed).unwrap_err(), Error::Conflict);
    assert!(s.commit(tx).unwrap().duplicate);
}

#[test]
fn stale_writers_share_one_cas_even_when_selecting_different_units() {
    let t = Temp::new();
    let mut a = seeded(&t);
    let mut b = open(&t);
    let stale = unit(b.head(), 5, 50, vec![credit(50, owner(70), 2, 25)], vec![]);
    a.commit(unit(
        a.head(),
        4,
        40,
        vec![credit(40, owner(70), 2, 15)],
        vec![],
    ))
    .unwrap();
    assert_eq!(b.commit(stale).unwrap_err(), Error::Stale);
    b.reload().unwrap();
    b.commit(unit(
        b.head(),
        5,
        50,
        vec![credit(50, owner(70), 2, 25)],
        vec![],
    ))
    .unwrap();
    a.reload().unwrap();
    assert_eq!(a.state().unwrap(), b.state().unwrap());
    assert_eq!(
        (cash(a.state().unwrap(), 40), cash(a.state().unwrap(), 50)),
        (115, 225)
    );
}

#[derive(Default)]
struct Faults {
    append_error: bool,
    panic_after: bool,
    freshness: bool,
}
struct FaultBackend {
    inner: SqliteBackend,
    faults: Arc<Mutex<Faults>>,
}
impl Backend for FaultBackend {
    fn load(&mut self) -> Result<Vec<Frame>, Error> {
        self.inner.load()
    }
    fn append(&mut self, expected: Option<Head>, frame: &Frame) -> Result<(), Error> {
        self.inner.append(expected, frame)?;
        let f = self.faults.lock().unwrap();
        let (error, panic) = (f.append_error, f.panic_after);
        drop(f);
        assert!(!panic, "synthetic post-commit interruption");
        if error { Err(Error::Storage) } else { Ok(()) }
    }
    fn check_current(&mut self, _: Head) -> Result<(), Error> {
        if self.faults.lock().unwrap().freshness {
            Err(Error::Storage)
        } else {
            Ok(())
        }
    }
}

#[test]
fn unknown_commit_and_unwinding_fence_all_units_until_replay_without_resend() {
    for panic in [false, true] {
        let t = Temp::new();
        let faults = Arc::new(Mutex::new(Faults::default()));
        let backend = FaultBackend {
            inner: SqliteBackend::create(&t.db).unwrap(),
            faults: faults.clone(),
        };
        let mut s = units::Journal::create(backend, FixtureProtection, configuration()).unwrap();
        let tx = enrollment(s.head(), 1, 70);
        {
            let mut f = faults.lock().unwrap();
            f.append_error = !panic;
            f.panic_after = panic;
        }
        if panic {
            assert!(
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| s.commit(tx.clone())))
                    .is_err()
            );
        } else {
            assert_eq!(s.commit(tx.clone()).unwrap_err(), Error::Storage);
        }
        assert_eq!(s.state().unwrap_err(), Error::Poisoned);
        assert_eq!(s.verified_state().unwrap_err(), Error::Poisoned);
        assert!(matches!(s.transactions(), Err(Error::Poisoned)));
        assert_eq!(s.commit(tx.clone()).unwrap_err(), Error::Poisoned);
        *faults.lock().unwrap() = Faults::default();
        s.reload().unwrap();
        assert_eq!(s.state().unwrap().customers().len(), 1);
        for which in [40, 50] {
            assert_eq!(
                s.state()
                    .unwrap()
                    .unit(uid(which))
                    .unwrap()
                    .registrations()
                    .len(),
                1
            );
        }
        assert!(s.commit(tx).unwrap().duplicate);
        assert_eq!(s.head().sequence, 1);
    }
}

#[test]
fn current_freshness_is_required_even_for_duplicate_and_reload() {
    let t = Temp::new();
    let faults = Arc::new(Mutex::new(Faults::default()));
    let mut s = units::Journal::create(
        FaultBackend {
            inner: SqliteBackend::create(&t.db).unwrap(),
            faults: faults.clone(),
        },
        FixtureProtection,
        configuration(),
    )
    .unwrap();
    let tx = enrollment(s.head(), 1, 70);
    s.commit(tx.clone()).unwrap();
    faults.lock().unwrap().freshness = true;
    assert_eq!(s.commit(tx).unwrap_err(), Error::Storage);
    assert_eq!(s.state().unwrap_err(), Error::Poisoned);
    assert_eq!(s.reload().unwrap_err(), Error::Storage);
    assert_eq!(s.state().unwrap_err(), Error::Poisoned);
    faults.lock().unwrap().freshness = false;
    s.reload().unwrap();
    assert_eq!(s.verified_state().unwrap().customers().len(), 1);
}

#[test]
fn legacy_and_new_histories_cannot_be_reinterpreted_or_overwritten() {
    let legacy = Temp::new();
    let store = legacy.create();
    drop(store);
    let backend = SqliteBackend::open(&legacy.db, Migration::None).unwrap();
    assert_eq!(
        Store::open(backend, FixtureProtection, configuration()).unwrap_err(),
        Error::Codec
    );
    let backend = SqliteBackend::open(&legacy.db, Migration::None).unwrap();
    assert_eq!(
        Store::create(backend, FixtureProtection, configuration()).unwrap_err(),
        Error::Conflict
    );
    let t = Temp::new();
    drop(seeded(&t));
    assert_eq!(
        Legacy::open(
            SqliteBackend::open(&t.db, Migration::None).unwrap(),
            FixtureProtection,
            support::config()
        )
        .unwrap_err(),
        Error::Codec
    );
    let mut d = descriptors();
    d[1].key_policy = [99; 32];
    let config = Configuration::new(support::config().domain, d).unwrap();
    assert_eq!(
        Store::open(
            SqliteBackend::open(&t.db, Migration::None).unwrap(),
            FixtureProtection,
            config
        )
        .unwrap_err(),
        Error::Conflict
    );
    assert_eq!(open(&t).head().sequence, 3);
}

fn cipher() -> RecordCipher {
    // Disposable fixture-only key. Production loading/attestation is not in scope.
    RecordCipher::new(Zeroizing::new([42; 32]), 1, [98; 32]).unwrap()
}
struct Playback(Vec<Frame>);
impl Backend for Playback {
    fn load(&mut self) -> Result<Vec<Frame>, Error> {
        Ok(self.0.clone())
    }
    fn append(&mut self, _: Option<Head>, _: &Frame) -> Result<(), Error> {
        Err(Error::Storage)
    }
}
fn rehash(frame: &mut Frame) {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    hash.update(b"CINDER-OPAQUE-FRAME-1\0");
    hash.update(frame.head.sequence.to_be_bytes());
    hash.update(frame.previous);
    hash.update((frame.opaque.as_bytes().len() as u64).to_be_bytes());
    hash.update(frame.opaque.as_bytes());
    frame.head.hash = hash.finalize().into();
}

#[test]
fn real_aead_replay_rejects_host_rehashed_corruption_and_wrong_key() {
    let t = Temp::new();
    let mut s = units::Journal::create(
        SqliteBackend::create(&t.db).unwrap(),
        cipher(),
        configuration(),
    )
    .unwrap();
    s.commit(enrollment(s.head(), 1, 70)).unwrap();
    let before = s.state().unwrap().clone();
    drop(s);
    let mut backend = SqliteBackend::open(&t.db, Migration::None).unwrap();
    let mut frames = backend.load().unwrap();
    let body = frames[1].opaque.as_bytes();
    assert!(!body.windows(32).any(|w| w == [70; 32]));
    let opened = units::Journal::open(Playback(frames.clone()), cipher(), configuration()).unwrap();
    assert_eq!(opened.state().unwrap(), &before);
    let wrong = RecordCipher::new(Zeroizing::new([43; 32]), 1, [98; 32]).unwrap();
    assert_eq!(
        units::Journal::open(Playback(frames.clone()), wrong, configuration()).unwrap_err(),
        Error::Protection
    );
    let mut bytes = frames[1].opaque.as_bytes().to_vec();
    let n = bytes.len();
    bytes[n - 1] ^= 1;
    frames[1].opaque = PrivateBytes::new(bytes).unwrap();
    rehash(&mut frames[1]);
    assert_eq!(
        units::Journal::open(Playback(frames), cipher(), configuration()).unwrap_err(),
        Error::Protection
    );
}

#[test]
fn replay_checks_versions_receipts_and_complete_projection_not_only_frame_hash() {
    let t = Temp::new();
    drop(seeded(&t));
    let frames = SqliteBackend::open(&t.db, Migration::None)
        .unwrap()
        .load()
        .unwrap();
    // Fixture protection lets us deliberately reseal invalid authenticated data.
    // Last frame avoids changing downstream chaining; each mutation is independent.
    for target in 0..4 {
        let mut corrupted = frames.clone();
        let f = corrupted.last_mut().unwrap();
        let context = RecordContext {
            domain: configuration().domain(),
            sequence: f.head.sequence,
            previous: f.previous,
        };
        let mut body = FixtureProtection
            .open(context, &f.opaque)
            .unwrap()
            .as_bytes()
            .to_vec();
        match target {
            0 => body[13] ^= 1, // unit format version
            1 => body[15] ^= 1, // reused financial-engine revision
            2 => {
                let n = body.len();
                body[n - 33] ^= 1;
            } // stored receipt
            3 => {
                let n = body.len();
                body[n - 1] ^= 1;
            } // entire state commitment
            _ => unreachable!(),
        }
        f.opaque = FixtureProtection.seal(context, &body).unwrap();
        rehash(f);
        let result = units::Journal::open(Playback(corrupted), FixtureProtection, configuration());
        assert!(result.is_err());
    }
}

#[test]
fn sixty_four_shared_records_keep_unit_inventory_and_retry_identity() {
    let t = Temp::new();
    let mut s = seeded(&t);
    for n in 4..=64 {
        let which = if n % 2 == 0 { 40 } else { 50 };
        s.commit(unit(
            s.head(),
            n,
            which,
            vec![credit(which, owner(70), u64::from(n), 1)],
            vec![],
        ))
        .unwrap();
    }
    assert_eq!(s.head().sequence, 64);
    assert_eq!(s.transactions().unwrap().count(), 64);
    assert_eq!(
        (cash(s.state().unwrap(), 40), cash(s.state().unwrap(), 50)),
        (131, 230)
    );
    let before = s.state().unwrap().clone();
    drop(s);
    assert_eq!(open(&t).state().unwrap(), &before);
}

#[test]
fn oversized_records_and_history_never_replace_an_accepted_state() {
    let t = Temp::new();
    let mut s = seeded(&t);
    let head = s.head();
    let before = s.state().unwrap().clone();
    let mut input = credit(40, owner(70), 2, 1);
    input.raw = PrivateBytes::new(vec![7; cinder_journal::wire::MAX_RECORD]).unwrap();
    assert_eq!(
        s.commit(unit(head, 4, 40, vec![input], vec![]))
            .unwrap_err(),
        Error::Limit
    );
    assert_eq!(s.head(), head);
    assert_eq!(s.state().unwrap(), &before);
    let frames = SqliteBackend::open(&t.db, Migration::None)
        .unwrap()
        .load()
        .unwrap();
    assert_eq!(
        units::Journal::open(
            Playback(vec![frames[0].clone(); cinder_journal::MAX_RECORDS + 1]),
            FixtureProtection,
            configuration()
        )
        .unwrap_err(),
        Error::Codec
    );
    assert_eq!(
        units::Journal::open(Playback(vec![]), FixtureProtection, configuration()).unwrap_err(),
        Error::Codec
    );
    let mut missing = frames.clone();
    missing.remove(1);
    assert_eq!(
        units::Journal::open(Playback(missing), FixtureProtection, configuration()).unwrap_err(),
        Error::Codec
    );
    let mut repeated = frames;
    repeated.push(repeated[1].clone());
    assert_eq!(
        units::Journal::open(Playback(repeated), FixtureProtection, configuration()).unwrap_err(),
        Error::Codec
    );
}

#[test]
fn opaque_history_byte_budget_precedes_acceptance_and_still_allows_exact_retry() {
    // Explicit fixture padding to reach the 64MiB limit without 4,096 appends.
    struct Padded;
    impl Protection for Padded {
        fn seal(&self, c: RecordContext, p: &[u8]) -> Result<PrivateBytes, Error> {
            let inner = FixtureProtection.seal(c, p)?;
            let mut out = (inner.as_bytes().len() as u32).to_be_bytes().to_vec();
            out.extend_from_slice(inner.as_bytes());
            out.resize(cinder_journal::wire::MAX_RECORD, 0);
            PrivateBytes::new(out)
        }
        fn open(&self, c: RecordContext, b: &PrivateBytes) -> Result<PrivateBytes, Error> {
            let size = u32::from_be_bytes(b.as_bytes()[..4].try_into().unwrap()) as usize;
            FixtureProtection.open(c, &PrivateBytes::new(b.as_bytes()[4..4 + size].to_vec())?)
        }
    }
    let t = Temp::new();
    let mut s = units::Journal::create(
        SqliteBackend::create(&t.db).unwrap(),
        Padded,
        configuration(),
    )
    .unwrap();
    let first = enrollment(s.head(), 1, 70);
    s.commit(first.clone()).unwrap();
    let frames = cinder_journal::MAX_HISTORY_BYTES / cinder_journal::wire::MAX_RECORD;
    for n in 2..frames {
        s.commit(unit(s.head(), n as u8, 40, vec![], vec![]))
            .unwrap();
    }
    let head = s.head();
    let next = unit(head, frames as u8, 50, vec![], vec![]);
    assert_eq!(s.commit(next.clone()).unwrap_err(), Error::Limit);
    assert_eq!(s.head(), head);
    assert!(s.commit(first).unwrap().duplicate);
    drop(s);
    let mut reopened = units::Journal::open(
        SqliteBackend::open(&t.db, Migration::None).unwrap(),
        Padded,
        configuration(),
    )
    .unwrap();
    assert_eq!(reopened.head(), head);
    assert_eq!(reopened.commit(next).unwrap_err(), Error::Limit);
}

// Trusted journal-header fixtures only. Real signed-wire interpretation lives
// in cinder-api's P31 tests; this placeholder is not accepted grant evidence.
fn api_record(id: u8, which: u8, effect: units::api::Effect) -> units::api::Record {
    units::api::Record {
        unit: uid(which),
        request: request(id),
        policy: PolicyVersion::new(1).unwrap(),
        digest: [id; 32],
        epoch: 1,
        signer: [70; 32],
        contract: [90; 32],
        effect,
        wire: raw(b"journal-api-placeholder"),
    }
}
fn api_tx(head: Head, id: u8, record: units::api::Record) -> units::Transaction {
    global(head, id, Change::Api(Box::new(record)))
}

fn decision(
    head: Head,
    n: u8,
    kind: units::api::Kind,
    controls: Vec<Control>,
    inputs: Vec<Input>,
) -> units::Transaction {
    let proposal = proposal(head, n, inputs, controls);
    global(
        head,
        n,
        Change::Admit(Box::new(units::api::Admission {
            record: api_record(20, 40, units::api::Effect::Recorded),
            kind,
            proposal,
        })),
    )
}
#[test]
fn financial_join_refusal_is_terminal_and_retains_actual_facts_without_routes() {
    let t = Temp::new();
    let mut j = seeded(&t);
    let other = j.state().unwrap().unit(uid(50)).unwrap().clone();
    let tx = decision(
        j.head(),
        4,
        units::api::Kind::Order,
        vec![reserve(20, 10)],
        vec![credit(40, owner(70), 2, 25)],
    );
    let result = j.commit(tx.clone()).unwrap();
    let units::Receipt::Admitted { receipt, .. } = &result.receipt else {
        panic!("decision receipt");
    };
    assert_eq!(receipt.controls, Some(ControlError::Invalid));
    assert_eq!(cash(j.state().unwrap(), 40), 125);
    assert_eq!(j.state().unwrap().api_records().len(), 1);
    assert_eq!(j.state().unwrap().api_decisions().len(), 1);
    assert!(j.state().unwrap().route(request(20)).is_none());
    assert!(j.state().unwrap().unit(uid(40)).unwrap().holds().is_empty());
    assert_eq!(j.state().unwrap().unit(uid(50)).unwrap(), &other);
    assert!(j.commit(tx).unwrap().duplicate);
    let next = decision(
        j.head(),
        5,
        units::api::Kind::Order,
        vec![reserve(20, 10)],
        vec![],
    );
    assert_eq!(j.commit(next).unwrap_err(), Error::Invalid);
    drop(j);
    assert_eq!(open(&t).state().unwrap().api_decisions().len(), 1);
}
#[test]
fn financial_join_requires_original_record_epoch_headers_and_joined_risk() {
    let t = Temp::new();
    let mut j = seeded(&t);
    let selection = cinder_journal::risk::Selection {
        request: request(20),
        market: support::q(0).unit(),
        leverage: 20_000,
        policy: PolicyVersion::new(1).unwrap(),
        authority_epoch: 1,
        expires_at: 1000,
    };
    let control = Control::Risk(cinder_journal::risk::Action::Select {
        approval: orders::Approval {
            account: owner(70),
            intent_hash: selection.digest().unwrap(),
            authority_epoch: 1,
        },
        selection: Box::new(selection),
    });
    let mut bad = decision(
        j.head(),
        4,
        units::api::Kind::Leverage,
        vec![control.clone()],
        vec![],
    );
    if let Change::Admit(a) = &mut bad.change {
        a.proposal.at += 1;
    }
    assert_eq!(j.commit(bad).unwrap_err(), Error::Invalid);
    let result = j
        .commit(decision(
            j.head(),
            4,
            units::api::Kind::Leverage,
            vec![control],
            vec![],
        ))
        .unwrap();
    assert!(matches!(
        result.receipt,
        units::Receipt::Admitted {
            receipt: Receipt {
                controls: Some(ControlError::Unqualified),
                ..
            },
            ..
        }
    ));
    let bypass = unit(j.head(), 5, 40, vec![], vec![reserve(20, 10)]);
    assert_eq!(
        outcome(&j.commit(bypass).unwrap().receipt).controls,
        Some(ControlError::Invalid)
    );
    assert!(j.state().unwrap().route(request(20)).is_none());
    assert_eq!(
        format!("{:?}", j.state().unwrap().api_decisions()[0]),
        "ApiDecision([PRIVATE])"
    );
}
#[test]
fn prior_unit_genesis_revisions_are_not_silently_upgraded_for_native_execution() {
    for revision in [1, 2, 3, 4, 5] {
        let t = Temp::new();
        drop(create(&t));
        let mut frames = SqliteBackend::open(&t.db, Migration::None)
            .unwrap()
            .load()
            .unwrap();
        let frame = &mut frames[0];
        let context = RecordContext {
            domain: support::config().domain,
            sequence: 0,
            previous: [0; 32],
        };
        let mut body = FixtureProtection
            .open(context, &frame.opaque)
            .unwrap()
            .as_bytes()
            .to_vec();
        body[13] = revision;
        frame.opaque = FixtureProtection.seal(context, &body).unwrap();
        rehash(frame);
        assert_eq!(
            units::Journal::open(Playback(frames), FixtureProtection, configuration()).unwrap_err(),
            Error::Version
        );
    }
}

#[test]
fn shared_api_headers_bind_contract_owner_policy_and_capacity_without_financial_effects() {
    let t = Temp::new();
    let mut j = seeded(&t);
    let before = j.state().unwrap().clone();
    let tx = api_tx(
        j.head(),
        4,
        api_record(20, 40, units::api::Effect::Recorded),
    );
    assert_eq!(j.commit(tx.clone()).unwrap().receipt, units::Receipt::Api);
    let head = j.head();
    assert!(j.commit(tx).unwrap().duplicate);
    assert_eq!(j.head(), head);
    for which in [40, 50] {
        assert_eq!(j.state().unwrap().unit(uid(which)), before.unit(uid(which)));
    }
    for change in 0..9 {
        let mut r = api_record(21, 40, units::api::Effect::Recorded);
        let expected = match change {
            0 => {
                r.request.account = AccountId::new([1; 32]).unwrap();
                Error::Invalid
            }
            1 => {
                r.unit = uid(99);
                Error::Invalid
            }
            2 => {
                r.policy = PolicyVersion::new(2).unwrap();
                Error::Invalid
            }
            3 => {
                r.epoch = 2;
                Error::Invalid
            }
            4 => {
                r.contract = [91; 32];
                Error::Conflict
            }
            5 => {
                r.digest = [0; 32];
                Error::Invalid
            }
            6 => {
                r.wire = raw(&vec![1; 1025]);
                Error::Invalid
            }
            7 => {
                r.effect = units::api::Effect::Grant;
                r.signer = [71; 32];
                Error::Invalid
            }
            _ => {
                r.request = request(20);
                r.unit = uid(50);
                Error::Conflict
            }
        };
        assert_eq!(j.commit(api_tx(head, 5, r)).unwrap_err(), expected);
        assert_eq!(j.head(), head);
    }
    let accepted = j.state().unwrap().clone();
    drop(j);
    let j = open(&t);
    assert_eq!(j.state().unwrap(), &accepted);
}

#[test]
fn api_intents_and_foundation_reservations_cannot_reassign_each_others_unit() {
    let t = Temp::new();
    let mut j = seeded(&t);
    j.commit(api_tx(
        j.head(),
        4,
        api_record(20, 40, units::api::Effect::Recorded),
    ))
    .unwrap();
    let denied = j
        .commit(unit(j.head(), 5, 50, vec![], vec![reserve(20, 5)]))
        .unwrap();
    assert_eq!(
        outcome(&denied.receipt).controls,
        Some(ControlError::Invalid)
    );
    assert!(j.state().unwrap().route(request(20)).is_none());
    j.commit(unit(j.head(), 6, 50, vec![], vec![reserve(21, 5)]))
        .unwrap();
    assert_eq!(j.state().unwrap().route(request(21)).unwrap().unit, uid(50));
    let head = j.head();
    for which in [40, 50] {
        assert_eq!(
            j.commit(api_tx(
                head,
                7,
                api_record(21, which, units::api::Effect::Recorded)
            ))
            .unwrap_err(),
            Error::Conflict
        );
    }
    assert_eq!(j.head(), head);
    drop(j);
    let j = open(&t);
    assert_eq!(j.state().unwrap().api_records().len(), 1);
}

#[test]
fn api_records_share_the_same_cas_between_units_and_keep_losing_state_unmodified() {
    let t = Temp::new();
    let mut a = seeded(&t);
    let mut b = open(&t);
    a.commit(api_tx(
        a.head(),
        4,
        api_record(20, 40, units::api::Effect::Recorded),
    ))
    .unwrap();
    let losing = api_tx(
        b.head(),
        5,
        api_record(21, 50, units::api::Effect::Recorded),
    );
    assert_eq!(b.commit(losing.clone()).unwrap_err(), Error::Stale);
    assert!(b.state().unwrap().api_records().is_empty());
    b.reload().unwrap();
    let mut retry = losing;
    retry.expected = b.head();
    b.commit(retry).unwrap();
    assert_eq!(b.state().unwrap().api_records().len(), 2);
    drop(a);
    drop(b);
    assert_eq!(open(&t).state().unwrap().api_records().len(), 2);
}

#[test]
fn api_record_bytes_and_new_replay_revision_are_authenticated_in_complete_state() {
    let t = Temp::new();
    let mut j = seeded(&t);
    j.commit(api_tx(
        j.head(),
        4,
        api_record(20, 40, units::api::Effect::Recorded),
    ))
    .unwrap();
    drop(j);
    let frames = SqliteBackend::open(&t.db, Migration::None)
        .unwrap()
        .load()
        .unwrap();
    for old_revision in [false, true] {
        let mut changed = frames.clone();
        let index = if old_revision { 0 } else { changed.len() - 1 };
        let frame = &mut changed[index];
        let context = RecordContext {
            domain: support::config().domain,
            sequence: frame.head.sequence,
            previous: frame.previous,
        };
        let mut body = FixtureProtection
            .open(context, &frame.opaque)
            .unwrap()
            .as_bytes()
            .to_vec();
        if old_revision {
            body[13] = 1;
        } else {
            let marker = b"journal-api-placeholder";
            let at = body
                .windows(marker.len())
                .position(|b| b == marker)
                .unwrap();
            body[at] ^= 1;
        }
        frame.opaque = FixtureProtection.seal(context, &body).unwrap();
        rehash(frame);
        let error = units::Journal::open(Playback(changed), FixtureProtection, configuration())
            .unwrap_err();
        assert_eq!(
            error,
            if old_revision {
                Error::Version
            } else {
                Error::Codec
            }
        );
    }
}

// Synthetic independent storage/witness ports, not deployed availability proof.
#[derive(Clone)]
struct MemoryReplica {
    id: u8,
    data: Arc<Mutex<std::collections::BTreeMap<[u8; 32], Frame>>>,
}
impl cinder_journal::replicated::Replica for MemoryReplica {
    fn identity(&self) -> [u8; 32] {
        [self.id; 32]
    }
    fn put(&mut self, frame: &Frame) -> Result<(), Error> {
        self.data
            .lock()
            .unwrap()
            .insert(frame.head.hash, frame.clone());
        Ok(())
    }
    fn get(&mut self, hash: [u8; 32]) -> Result<Frame, Error> {
        self.data
            .lock()
            .unwrap()
            .get(&hash)
            .cloned()
            .ok_or(Error::Storage)
    }
}
#[derive(Clone)]
struct MemoryWitness(Arc<Mutex<(cinder_journal::replicated::Anchor, bool)>>);
impl cinder_journal::replicated::Witness for MemoryWitness {
    fn read(
        &mut self,
        _: cinder_journal::replicated::Stream,
    ) -> Result<cinder_journal::replicated::Anchor, Error> {
        Ok(self.0.lock().unwrap().0)
    }
    fn accept(
        &mut self,
        _: cinder_journal::replicated::Stream,
        expected: cinder_journal::replicated::Anchor,
        next: Head,
    ) -> Result<(), Error> {
        let mut current = self.0.lock().unwrap();
        if current.0 != expected {
            return Err(Error::Stale);
        }
        current.0.head = Some(next);
        if current.1 {
            Err(Error::Storage)
        } else {
            Ok(())
        }
    }
}

#[test]
fn replicated_aead_history_preserves_both_units_after_lost_ack_replica_loss_and_epoch_fence() {
    use cinder_journal::replicated::{Anchor, Replicated, Stream};
    let a = MemoryReplica {
        id: 1,
        data: Arc::default(),
    };
    let b = MemoryReplica {
        id: 2,
        data: Arc::default(),
    };
    let witness = MemoryWitness(Arc::new(Mutex::new((
        Anchor {
            epoch: 1,
            head: None,
        },
        false,
    ))));
    let backend = |epoch| {
        Replicated::new(
            Stream {
                domain: configuration().domain(),
                id: [98; 32],
            },
            epoch,
            a.clone(),
            b.clone(),
            witness.clone(),
        )
        .unwrap()
    };
    let mut s = units::Journal::create(backend(1), cipher(), configuration()).unwrap();
    s.commit(enrollment(s.head(), 1, 70)).unwrap();
    s.commit(unit(
        s.head(),
        2,
        40,
        vec![credit(40, owner(70), 1, 100)],
        vec![],
    ))
    .unwrap();
    let original = unit(s.head(), 3, 50, vec![credit(50, owner(70), 1, 200)], vec![]);
    witness.0.lock().unwrap().1 = true;
    assert_eq!(s.commit(original.clone()).unwrap_err(), Error::Storage);
    assert_eq!(s.state().unwrap_err(), Error::Poisoned);
    witness.0.lock().unwrap().1 = false;
    a.data.lock().unwrap().clear(); // Surviving replica + fresh accepted head suffice for replay.
    s.reload().unwrap();
    assert!(s.commit(original).unwrap().duplicate);
    assert_eq!(
        (cash(s.state().unwrap(), 40), cash(s.state().unwrap(), 50)),
        (100, 200)
    );
    let before = s.state().unwrap().clone();
    witness.0.lock().unwrap().0.epoch = 2;
    assert_eq!(s.verified_state().unwrap_err(), Error::Stale);
    assert_eq!(s.state().unwrap_err(), Error::Poisoned);
    assert_eq!(s.reload().unwrap_err(), Error::Stale);
    let replacement = units::Journal::open(backend(2), cipher(), configuration()).unwrap();
    assert_eq!(replacement.state().unwrap(), &before);
}
