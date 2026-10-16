//! Synthetic joined policy only; no native venue or calibrated capital claims.
use super::*;
use cinder_api::Admission as _;
use cinder_journal::{collateral, orders, risk};
use cinder_kernel::ledger::{evidence::*, *};

fn read_view(
    j: &mut Store,
    service: &api::service::Service<Holds>,
) -> (cinder_journal::read::Reader, cinder_journal::read::Binding) {
    let binding = cinder_journal::read::Binding {
        stream: cinder_journal::replicated::Stream {
            domain: domain(),
            id: [91; 32],
        },
        epoch: 1,
        api: service.commitment(j.configuration()).unwrap(),
    };
    (j.attach_reader(binding).unwrap(), binding)
}
struct ReadWitness(Head);
impl cinder_journal::read::Witness for ReadWitness {
    fn read(
        &self,
        _: cinder_journal::replicated::Stream,
    ) -> Result<cinder_journal::replicated::Anchor, cinder_journal::Error> {
        Ok(cinder_journal::replicated::Anchor {
            epoch: 1,
            head: Some(self.0),
        })
    }
}
fn read(
    service: &api::service::Service<Holds>,
    reader: &cinder_journal::read::Reader,
    head: Head,
    request: &api::Request,
) -> Result<Response, Error> {
    let ticket = reader.capture().unwrap();
    service.authenticate_read(
        ticket.unit_configuration().unwrap(),
        &Channel::default(),
        &request.encode().unwrap(),
        10,
    )?;
    let verified = reader.verify(ticket, &ReadWitness(head)).unwrap();
    let result = service.read(
        &verified,
        &Channel::default(),
        &request.encode().unwrap(),
        10,
    );
    verified
        .release(std::time::Instant::now() + std::time::Duration::from_secs(1))
        .unwrap();
    result
}

#[test]
fn unit_service_reads_recording_admission_lifecycle_without_legacy_digest_or_other_unit_data() {
    let t = Temp::new();
    let mut j = seed(&t);
    let service = api::service::Service::new(authorizer(), Holds(100));
    let staged = named(90, order(), 9);
    authorizer()
        .capture(&mut j, &Channel::default(), &staged.encode().unwrap(), 10)
        .unwrap();
    let (reader, _) = read_view(&mut j, &service);
    let lookup = named(91, wire::Command::Operation(staged.body.id), 9);
    let Response::Receipt(r) = read(&service, &reader, j.head(), &lookup).unwrap() else {
        panic!()
    };
    assert_eq!(r.outcome, cinder_api::Outcome::Recorded);
    assert_eq!(r.digest, staged.digest());
    assert!(component(&j).holds().is_empty());
    let Response::Receipt(r) = service
        .mutate(&mut j, &Channel::default(), &staged.encode().unwrap(), 10)
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(r.outcome, cinder_api::Outcome::Accepted);
    assert_eq!(r.digest, staged.digest());
    let Response::Receipt(r) = read(&service, &reader, j.head(), &lookup).unwrap() else {
        panic!()
    };
    assert_eq!(r.outcome, cinder_api::Outcome::Accepted);
    for which in [40, 50] {
        for kind in 0..cinder_api::reads::KINDS {
            let r = request(
                which,
                wire::Command::Read(cinder_api::reads::Query {
                    kind,
                    cursor: [0; 40],
                    limit: 64,
                }),
                9,
            );
            assert!(matches!(
                read(&service, &reader, j.head(), &r),
                Ok(Response::Read(_))
            ));
        }
        let r = request(which, wire::Command::View, 9);
        let Response::View(v) = read(&service, &reader, j.head(), &r).unwrap() else {
            panic!()
        };
        assert_eq!(v.cash, if which == 40 { 2000 } else { 0 });
        assert_eq!(v.operations.len(), usize::from(which == 40));
    }
    let other = request(50, wire::Command::Operation(staged.body.id), 9);
    assert!(matches!(
        read(&service, &reader, j.head(), &other),
        Err(Error::NotFound)
    ));
}

#[test]
fn shared_publication_invalidates_prepared_unit_reads_on_other_unit_revoke_and_refuses_legacy_source()
 {
    use cinder_journal::read::{Failure, Source};
    let t = Temp::new();
    let mut j = seed(&t);
    grant_agent(&mut j);
    let service = api::service::Service::new(authorizer(), Holds(100));
    let (reader, _) = read_view(&mut j, &service);
    let view = reader
        .verify(reader.capture().unwrap(), &ReadWitness(j.head()))
        .unwrap();
    assert!(Source::state(&view).is_err());
    let agent = named(91, wire::Command::View, 10);
    service
        .read(&view, &Channel::default(), &agent.encode().unwrap(), 10)
        .unwrap();
    let other = request(50, wire::Command::View, 10);
    assert!(matches!(
        service.read(&view, &Channel::default(), &other.encode().unwrap(), 10),
        Err(Error::Unauthorized)
    ));
    let revoke = request(50, wire::Command::Revoke, 9);
    service
        .mutate(&mut j, &Channel::default(), &revoke.encode().unwrap(), 10)
        .unwrap();
    assert_eq!(
        view.release(std::time::Instant::now() + std::time::Duration::from_secs(1)),
        Err(Failure::Raced)
    );
    assert!(matches!(
        read(&service, &reader, j.head(), &agent),
        Err(Error::Unauthorized)
    ));
}

#[test]
fn unit_cursor_is_not_reusable_on_another_unit_even_for_identical_market_rows() {
    let t = Temp::new();
    let mut j = seed(&t);
    let service = api::service::Service::new(authorizer(), Holds(100));
    let (reader, _) = read_view(&mut j, &service);
    let query = cinder_api::reads::Query {
        kind: 8,
        cursor: [0; 40],
        limit: 1,
    };
    let a = request(40, wire::Command::Read(query), 9);
    let b = request(50, wire::Command::Read(query), 9);
    let Response::Read(a) = read(&service, &reader, j.head(), &a).unwrap() else {
        panic!()
    };
    let Response::Read(b) = read(&service, &reader, j.head(), &b).unwrap() else {
        panic!()
    };
    assert_ne!(a.revision, b.revision);
    let mut cursor = [0; 40];
    cursor[..32].copy_from_slice(&a.revision);
    cursor[39] = 1;
    let other = request(
        50,
        wire::Command::Read(cinder_api::reads::Query { cursor, ..query }),
        9,
    );
    assert!(matches!(
        read(&service, &reader, j.head(), &other),
        Err(Error::Conflict)
    ));
}

#[test]
fn operation_capture_timestamp_remains_original_after_later_admission() {
    let t = Temp::new();
    let mut j = seed(&t);
    let service = api::service::Service::new(authorizer(), Holds(100));
    let staged = named(90, order(), 9);
    authorizer()
        .capture(&mut j, &Channel::default(), &staged.encode().unwrap(), 10)
        .unwrap();
    let (reader, _) = read_view(&mut j, &service);
    service
        .mutate(&mut j, &Channel::default(), &staged.encode().unwrap(), 20)
        .unwrap();
    for kind in [2, 3] {
        let view = reader
            .verify(reader.capture().unwrap(), &ReadWitness(j.head()))
            .unwrap();
        let req = request(
            40,
            wire::Command::Read(cinder_api::reads::Query {
                kind,
                cursor: [0; 40],
                limit: 64,
            }),
            9,
        );
        let Response::Read(page) = service
            .read(&view, &Channel::default(), &req.encode().unwrap(), 20)
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(page.rows.len(), 1);
        assert_eq!(&page.rows[0].as_bytes()[..8], &10_u64.to_be_bytes());
        view.release(std::time::Instant::now() + std::time::Duration::from_secs(1))
            .unwrap();
    }
}

fn financial_config() -> units::Configuration {
    let mut descriptors = config().units().to_vec();
    for d in &mut descriptors {
        let mut vault = d.ledger.sources[0];
        vault.location = Location::Vault;
        vault.scope.namespace = NamespaceId::new([8; 32]).unwrap();
        d.ledger.sources.push(vault);
    }
    units::Configuration::new(domain(), descriptors).unwrap()
}
fn open(t: &Temp) -> Store {
    Store::open(
        SqliteBackend::open(&t.db, Migration::None).unwrap(),
        cipher(),
        financial_config(),
    )
    .unwrap()
}
fn commit(j: &mut Store, n: u8, events: Vec<Event>, controls: Vec<Control>) -> units::Receipt {
    let c = financial_config();
    let c = &c.units()[0].ledger;
    let mut p = support::transaction(j.head(), n, vec![], controls);
    p.at = 10;
    p.inputs = events
        .into_iter()
        .map(|e| {
            let RecordKey::Economic(k) = &e.key else {
                panic!("economic fixture");
            };
            Input {
                source: k.scope,
                source_cut: Some(u64::from(n)),
                authority_epoch: 1,
                observed_at: 10,
                raw: support::raw(b"qualified-unit-financial-fixture"),
                event: Some(e),
            }
        })
        .collect();
    assert_eq!(c.domain, domain());
    j.commit(units::Transaction::unit(uid(40), p))
        .unwrap()
        .receipt
}
fn event(c: &Config, n: u8, source: usize, change: Change) -> Event {
    Event {
        key: RecordKey::Economic(EventKey {
            scope: c.sources[source].scope,
            event: EconomicEventId::new(&[n]).unwrap(),
            leg: 0,
        }),
        policy: c.policy,
        change,
    }
}
fn seed(t: &Temp) -> Store {
    let mut j = Store::create(
        SqliteBackend::create(&t.db).unwrap(),
        cipher(),
        financial_config(),
    )
    .unwrap();
    j.commit(enrollment(j.head())).unwrap();
    let registry = financial_config();
    let c = &registry.units()[0].ledger;
    let events = vec![
        event(
            c,
            1,
            0,
            Change::Receipt {
                owner: Owner::Customer(account()),
                location: Location::Venue,
                amount: support::cash(1000),
            },
        ),
        event(
            c,
            2,
            0,
            Change::Receipt {
                owner: Owner::House,
                location: Location::Venue,
                amount: support::cash(1000),
            },
        ),
        event(
            c,
            3,
            1,
            Change::Receipt {
                owner: Owner::Customer(account()),
                location: Location::Vault,
                amount: support::cash(1000),
            },
        ),
    ];
    assert!(matches!(
        commit(&mut j, 2, events, vec![]),
        units::Receipt::Unit {
            receipt: Receipt { controls: None, .. },
            ..
        }
    ));
    let l = j.state().unwrap().unit(uid(40)).unwrap().ledger();
    let check = event(
        c,
        4,
        0,
        Change::Reconcile(NativeCheck {
            expected_version: l.version(),
            cash: Some(l.venue().cash()),
            funding: Some(l.venue().funding()),
            positions: Some(l.venue().positions().to_vec()),
            complete: true,
            resolves: vec![],
        }),
    );
    commit(&mut j, 3, vec![check], vec![]);
    let expected_version = j.state().unwrap().unit(uid(40)).unwrap().ledger().version();
    let controls = vec![
        Control::Collateral(collateral::Cut {
            expected_version,
            policy: collateral::Policy {
                revision: c.policy,
                markets: vec![collateral::MarginRule {
                    market: support::q(0).unit(),
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
                price: support::p(100),
                evidence: EventKey {
                    scope: c.sources[0].scope,
                    event: EconomicEventId::new(b"mark").unwrap(),
                    leg: 0,
                },
                observed_at: 10,
                valid_until: 100,
                qualified: true,
            }],
        }),
        Control::Risk(risk::Action::Install {
            expected_version,
            policy: Box::new(risk::Policy {
                revision: c.policy,
                markets: vec![risk::MarketRule {
                    market: support::q(0).unit(),
                    maximum_leverage: 100_000,
                    maintenance_bps: 500,
                    native_maintenance_bps: 500,
                    gross_limit: support::cash(1_000_000),
                    net_limit: support::cash(1_000_000),
                }],
                buffer: support::cash(0),
                horizon_ms: 50,
                valid_until: 100,
                paths: vec![risk::Path {
                    id: [1; 32],
                    steps: vec![risk::Step {
                        after_ms: 1,
                        marks: vec![support::p(100)],
                        events: vec![],
                        liquidity: [Location::Vault, Location::Venue]
                            .into_iter()
                            .map(|location| risk::Liquidity {
                                location,
                                accessible_bps: 10_000,
                                due: support::cash(0),
                            })
                            .collect(),
                    }],
                }],
            }),
        }),
    ];
    assert!(matches!(
        commit(&mut j, 4, vec![], controls),
        units::Receipt::Unit {
            receipt: Receipt { controls: None, .. },
            ..
        }
    ));
    j
}
struct Holds(i128);
impl cinder_api::Admission for Holds {
    fn order_holds(
        &self,
        state: &State,
        intent: &orders::Intent,
    ) -> Result<Vec<Reservation>, Error> {
        assert_eq!(
            state
                .ledger()
                .book(Owner::Customer(account()))
                .unwrap()
                .cash()
                .atoms(),
            2000
        );
        Ok(vec![
            Reservation {
                resource: Resource::Customer(intent.request.account),
                amount: support::cash(self.0),
            },
            Reservation {
                resource: Resource::Location(Location::Venue),
                amount: support::cash(self.0),
            },
        ])
    }
}
fn admit(
    j: &mut Store,
    r: &api::Request,
    holds: i128,
    now: u64,
) -> Result<api::admission::Decided, Error> {
    authorizer().admit(
        j,
        &Channel::default(),
        &r.encode().unwrap(),
        &Holds(holds),
        now,
    )
}
fn named(id: u8, command: wire::Command, signer: u8) -> api::Request {
    let mut r = request(40, command, signer);
    r.body.id = RequestId::new([id; 32]).unwrap();
    sign(&mut r, signer);
    r
}
fn component(j: &Store) -> &State {
    j.state().unwrap().unit(uid(40)).unwrap()
}
fn grant_agent(j: &mut Store) {
    let r = named(60, wire::Command::Grant(grant()), 9);
    authorizer()
        .capture(j, &Channel::default(), &r.encode().unwrap(), 10)
        .unwrap();
}

#[test]
fn actual_admission_is_atomic_unit_local_and_replays_without_exposure() {
    let t = Temp::new();
    let mut j = seed(&t);
    let other = j.state().unwrap().unit(uid(50)).unwrap().clone();
    let r = named(21, order(), 9);
    let head = j.head();
    let d = admit(&mut j, &r, 100, 10).unwrap();
    assert_eq!(d.refusal, None);
    assert!(!d.duplicate);
    assert_eq!(j.head().sequence, head.sequence + 1);
    assert_eq!(j.state().unwrap().api_records().len(), 1);
    assert_eq!(j.state().unwrap().api_decisions().len(), 1);
    assert_eq!(component(&j).orders().len(), 1);
    assert_eq!(component(&j).holds().len(), 1);
    assert!(component(&j).attempts().is_empty());
    assert_eq!(j.state().unwrap().unit(uid(50)).unwrap(), &other);
    let accepted = j.state().unwrap().clone();
    drop(j);
    let mut j = open(&t);
    assert_eq!(j.state().unwrap(), &accepted);
    let head = j.head();
    assert!(admit(&mut j, &r, 100, 11).unwrap().duplicate);
    assert_eq!(j.head(), head);
    assert_eq!(format!("{d:?}"), "Decided([PRIVATE])");
}
#[test]
fn recorded_only_conversion_keeps_original_and_cannot_renew_epoch_or_deadline() {
    let t = Temp::new();
    let mut j = seed(&t);
    let r = named(21, order(), 9);
    authorizer()
        .capture(&mut j, &Channel::default(), &r.encode().unwrap(), 10)
        .unwrap();
    let original = j.state().unwrap().api_records()[0].clone();
    assert_eq!(admit(&mut j, &r, 100, 70).unwrap_err(), Error::Invalid);
    assert!(j.state().unwrap().api_decisions().is_empty());
    assert_eq!(j.state().unwrap().api_records()[0], original);
    let revoke = named(22, wire::Command::Revoke, 9);
    authorizer()
        .capture(&mut j, &Channel::default(), &revoke.encode().unwrap(), 71)
        .unwrap();
    let mut fresh = r.clone();
    fresh.body.epoch = 2;
    sign(&mut fresh, 9);
    assert_eq!(
        admit(&mut j, &fresh, 100, 72).unwrap_err(),
        Error::Unauthorized
    );
}
#[test]
fn original_record_converts_once_and_refreshed_auth_never_replaces_it() {
    let t = Temp::new();
    let mut j = seed(&t);
    let r = named(21, order(), 9);
    authorizer()
        .capture(&mut j, &Channel::default(), &r.encode().unwrap(), 10)
        .unwrap();
    let original = j.state().unwrap().api_records()[0].clone();
    let mut retry = r.clone();
    retry.body.expires_at = 95;
    sign(&mut retry, 9);
    let d = admit(&mut j, &retry, 100, 11).unwrap();
    assert_eq!(d.refusal, None);
    assert_eq!(d.request, r);
    assert_eq!(j.state().unwrap().api_records(), &[original]);
    assert_eq!(component(&j).orders()[0].intent.expires_at, 70);
}
#[test]
fn refused_candidate_is_durable_and_does_not_consume_agent_budget() {
    let t = Temp::new();
    let mut j = seed(&t);
    grant_agent(&mut j);
    let r = named(21, order(), 10);
    let d = admit(&mut j, &r, 1_000_000, 10).unwrap();
    assert!(d.refusal.is_some());
    assert!(component(&j).orders().is_empty());
    assert!(component(&j).holds().is_empty());
    let head = j.head();
    assert_eq!(admit(&mut j, &r, 100, 11).unwrap().refusal, d.refusal);
    assert_eq!(j.head(), head);
    let next = named(22, order(), 10);
    assert_eq!(admit(&mut j, &next, 100, 11).unwrap().refusal, None);
    let next = named(23, order(), 10);
    assert_eq!(
        admit(&mut j, &next, 100, 11).unwrap_err(),
        Error::Unauthorized
    );
    drop(j);
    let mut j = open(&t);
    assert!(admit(&mut j, &r, 100, 12).unwrap().duplicate);
}
#[test]
fn staged_agent_cannot_evade_quota_via_owner_retry_or_after_cancel() {
    let t = Temp::new();
    let mut j = seed(&t);
    grant_agent(&mut j);
    let staged = named(22, order(), 10);
    authorizer()
        .capture(&mut j, &Channel::default(), &staged.encode().unwrap(), 10)
        .unwrap();
    let r = named(21, order(), 10);
    assert_eq!(admit(&mut j, &r, 100, 10).unwrap().refusal, None);
    let cancel = named(
        23,
        wire::Command::Cancel {
            target: r.body.id,
            attempt: AttemptId::new([23; 32]).unwrap(),
            good_until: 70,
        },
        10,
    );
    assert_eq!(admit(&mut j, &cancel, 100, 11).unwrap().refusal, None);
    assert!(component(&j).orders()[0].abandoned);
    let mut retry = staged.clone();
    retry.body.signer = key(9).verifying_key().to_bytes();
    sign(&mut retry, 9);
    assert_eq!(
        admit(&mut j, &retry, 100, 12).unwrap_err(),
        Error::Unauthorized
    );
    assert!(admit(&mut j, &r, 100, 12).unwrap().duplicate);
}
#[test]
fn cancel_requires_actual_same_unit_order_and_original_agent_ownership() {
    let t = Temp::new();
    let mut j = seed(&t);
    grant_agent(&mut j);
    let r = named(21, order(), 9);
    authorizer()
        .capture(&mut j, &Channel::default(), &r.encode().unwrap(), 10)
        .unwrap();
    let cancel = named(
        22,
        wire::Command::Cancel {
            target: r.body.id,
            attempt: AttemptId::new([22; 32]).unwrap(),
            good_until: 70,
        },
        9,
    );
    assert_eq!(
        admit(&mut j, &cancel, 100, 11).unwrap_err(),
        Error::NotFound
    );
    admit(&mut j, &r, 100, 11).unwrap();
    let mut agent_cancel = cancel.clone();
    agent_cancel.body.signer = key(10).verifying_key().to_bytes();
    sign(&mut agent_cancel, 10);
    assert_eq!(
        admit(&mut j, &agent_cancel, 100, 12).unwrap_err(),
        Error::Unauthorized
    );
    let mut wrong = cancel.clone();
    wrong.unit = uid(50);
    sign(&mut wrong, 9);
    assert_eq!(admit(&mut j, &wrong, 100, 12).unwrap_err(), Error::NotFound);
    assert_eq!(admit(&mut j, &cancel, 100, 12).unwrap().refusal, None);
    assert!(!component(&j).holds()[0].active);
    assert!(component(&j).attempts().is_empty());
}
#[test]
fn payout_and_leverage_use_same_joined_engine_and_selected_recipient() {
    let t = Temp::new();
    let mut j = seed(&t);
    let r = named(
        21,
        wire::Command::Leverage {
            market: MarketId::new([7; 32]).unwrap(),
            leverage: 20_000,
            good_until: 70,
        },
        9,
    );
    assert_eq!(admit(&mut j, &r, 100, 10).unwrap().refusal, None);
    let payout = named(
        22,
        wire::Command::Payout {
            net: 50,
            maximum_fee: 1,
            allow_partial: false,
            good_until: 70,
        },
        9,
    );
    assert_eq!(admit(&mut j, &payout, 100, 11).unwrap().refusal, None);
    let f = &component(&j).funds()[0];
    assert_eq!(
        f.intent.destination,
        funds::Destination::Recipient([71; 32])
    );
    assert_eq!(f.intent.net.atoms(), 50);
    assert!(component(&j).attempts().is_empty());
    drop(j);
    let mut j = open(&t);
    assert!(admit(&mut j, &payout, 100, 12).unwrap().duplicate);
    let bad = named(
        23,
        wire::Command::Leverage {
            market: MarketId::new([7; 32]).unwrap(),
            leverage: 200_000,
            good_until: 70,
        },
        9,
    );
    assert!(admit(&mut j, &bad, 100, 12).unwrap().refusal.is_some());
}
#[test]
fn second_unit_cannot_use_first_units_credit_or_risk_policy() {
    let t = Temp::new();
    let mut j = seed(&t);
    let before = j.state().unwrap().clone();
    let mut r = named(21, order(), 9);
    r.unit = uid(50);
    sign(&mut r, 9);
    assert_eq!(admit(&mut j, &r, 100, 10).unwrap_err(), Error::Unavailable);
    assert_eq!(j.state().unwrap(), &before);
}

#[test]
fn unknown_financial_append_requires_explicit_reload_and_never_repeats_holds() {
    for after in [false, true] {
        let t = Temp::new();
        drop(seed(&t));
        let fail = Arc::new(AtomicBool::new(false));
        let mut j = units::Journal::open(
            ApiAppendFault {
                inner: SqliteBackend::open(&t.db, Migration::None).unwrap(),
                fail: fail.clone(),
                after,
            },
            cipher(),
            financial_config(),
        )
        .unwrap();
        let r = named(21, order(), 9);
        let before = j.head();
        fail.store(true, Ordering::SeqCst);
        assert_eq!(
            authorizer()
                .admit(
                    &mut j,
                    &Channel::default(),
                    &r.encode().unwrap(),
                    &Holds(100),
                    10
                )
                .unwrap_err(),
            Error::Unavailable
        );
        assert!(j.state().is_err());
        fail.store(false, Ordering::SeqCst);
        assert!(api::history::History::load(&mut j, &authorizer()).is_err());
        j.reload().unwrap();
        assert_eq!(j.head().sequence, before.sequence + u64::from(after));
        let d = authorizer()
            .admit(
                &mut j,
                &Channel::default(),
                &r.encode().unwrap(),
                &Holds(100),
                11,
            )
            .unwrap();
        assert_eq!(d.duplicate, after);
        assert_eq!(d.refusal, None);
        let s = j.state().unwrap().unit(uid(40)).unwrap();
        assert_eq!(s.orders().len(), 1);
        assert_eq!(s.holds().len(), 1);
        assert!(s.attempts().is_empty());
        assert_eq!(j.state().unwrap().api_records().len(), 1);
    }
}
#[test]
fn stale_writer_cannot_bypass_quota_or_spend_pending_reservations() {
    let t = Temp::new();
    let mut first = seed(&t);
    grant_agent(&mut first);
    let mut second = open(&t);
    let r = named(21, order(), 10);
    assert_eq!(admit(&mut first, &r, 100, 10).unwrap().refusal, None);
    let next = named(22, order(), 10);
    assert_eq!(
        admit(&mut second, &next, 100, 11).unwrap_err(),
        Error::Unavailable
    );
    // SQLite's explicit failed CAS is known not committed, not an uncertain
    // append. Its cached state stays unchanged until explicit reload.
    assert!(component(&second).orders().is_empty());
    second.reload().unwrap();
    assert_eq!(
        admit(&mut second, &next, 100, 11).unwrap_err(),
        Error::Unauthorized
    );
    assert_eq!(component(&second).orders().len(), 1);
    assert_eq!(component(&second).holds().len(), 1);
}

#[test]
fn concurrent_append_during_hold_computation_cannot_commit_stale_admission() {
    struct Race(std::cell::RefCell<Option<Store>>);
    impl cinder_api::Admission for Race {
        fn order_holds(
            &self,
            state: &State,
            intent: &orders::Intent,
        ) -> Result<Vec<Reservation>, Error> {
            let mut other = self.0.borrow_mut().take().unwrap();
            assert_eq!(
                admit(&mut other, &named(21, order(), 10), 100, 10)
                    .unwrap()
                    .refusal,
                None
            );
            Holds(100).order_holds(state, intent)
        }
    }
    let t = Temp::new();
    let mut j = seed(&t);
    grant_agent(&mut j);
    let race = Race(std::cell::RefCell::new(Some(open(&t))));
    let r = named(22, order(), 10);
    let head = j.head();
    assert_eq!(
        authorizer()
            .admit(&mut j, &Channel::default(), &r.encode().unwrap(), &race, 10)
            .unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(j.head(), head);
    assert!(component(&j).orders().is_empty());
    j.reload().unwrap();
    assert_eq!(j.head().sequence, head.sequence + 1);
    assert_eq!(component(&j).orders().len(), 1);
    assert_eq!(admit(&mut j, &r, 100, 11).unwrap_err(), Error::Unauthorized);
    assert!(
        j.state()
            .unwrap()
            .api_records()
            .iter()
            .all(|a| a.record.request.request != r.body.id)
    );
}
#[test]
fn duplicate_authentication_and_unit_economics_are_checked_before_decision_return() {
    let t = Temp::new();
    let mut j = seed(&t);
    let r = named(21, order(), 9);
    admit(&mut j, &r, 100, 10).unwrap();
    let head = j.head();
    let mut bad = r.clone();
    bad.body.signature[0] ^= 1;
    assert_eq!(
        admit(&mut j, &bad, 100, 11).unwrap_err(),
        Error::Unauthorized
    );
    let mut changed = r.clone();
    changed.unit = uid(50);
    sign(&mut changed, 9);
    assert_eq!(
        admit(&mut j, &changed, 100, 11).unwrap_err(),
        Error::Conflict
    );
    let mut changed = r.clone();
    if let wire::Command::Order { lots, .. } = &mut changed.body.command {
        *lots = 1;
    }
    sign(&mut changed, 9);
    assert_eq!(
        admit(&mut j, &changed, 100, 11).unwrap_err(),
        Error::Conflict
    );
    let revoke = named(22, wire::Command::Revoke, 9);
    authorizer()
        .capture(&mut j, &Channel::default(), &revoke.encode().unwrap(), 11)
        .unwrap();
    assert_eq!(admit(&mut j, &r, 100, 12).unwrap_err(), Error::Unauthorized);
    let mut current = r;
    current.body.epoch = 2;
    sign(&mut current, 9);
    assert!(admit(&mut j, &current, 100, 12).unwrap().duplicate);
    assert_eq!(j.head().sequence, head.sequence + 1);
}
#[test]
fn original_agent_expiry_is_not_bypassed_by_an_owner_converting_a_staged_order() {
    let t = Temp::new();
    let mut j = seed(&t);
    let mut g = grant();
    g.expires_at = 30;
    let gr = named(60, wire::Command::Grant(g), 9);
    authorizer()
        .capture(&mut j, &Channel::default(), &gr.encode().unwrap(), 10)
        .unwrap();
    let mut command = order();
    if let wire::Command::Order { good_until, .. } = &mut command {
        *good_until = 30;
    }
    let r = named(21, command, 10);
    authorizer()
        .capture(&mut j, &Channel::default(), &r.encode().unwrap(), 10)
        .unwrap();
    let mut owner = r;
    owner.body.signer = key(9).verifying_key().to_bytes();
    sign(&mut owner, 9);
    assert_eq!(
        admit(&mut j, &owner, 100, 30).unwrap_err(),
        Error::Unauthorized
    );
    assert!(component(&j).orders().is_empty());
}
#[test]
fn original_wire_mismatch_in_a_trusted_financial_join_fails_api_reinterpretation() {
    let t = Temp::new();
    let mut j = seed(&t);
    let r = named(21, order(), 9);
    authorizer()
        .capture(&mut j, &Channel::default(), &r.encode().unwrap(), 10)
        .unwrap();
    let record = j.state().unwrap().api_records()[0].record.clone();
    let intent = orders::Intent {
        request: record.request,
        quantity: support::q(1),
        minimum: support::p(90),
        maximum: support::p(110),
        maximum_fee_per_lot: support::cash(1),
        time_in_force: orders::TimeInForce::ImmediateOrCancel,
        reduce_only: false,
        policy: record.policy,
        authority_epoch: 1,
        expires_at: 70,
    };
    // Deliberately malformed trusted controller: valid engine approval, but one
    // lot instead of the two lots in the original signed API request.
    let approval = orders::Approval {
        account: account(),
        intent_hash: intent.digest().unwrap(),
        authority_epoch: 1,
    };
    let controls = vec![Control::Order(orders::Action::Accept {
        reservations: Holds(100).order_holds(component(&j), &intent).unwrap(),
        intent: Box::new(intent.clone()),
        approval,
    })];
    let proposal = support::transaction(j.head(), 90, vec![], controls);
    let tx = units::Transaction {
        id: proposal.id,
        expected: proposal.expected,
        at: proposal.at,
        change: units::Change::Admit(Box::new(units::api::Admission {
            record,
            kind: units::api::Kind::Order,
            proposal,
        })),
    };
    j.commit(tx).unwrap();
    assert!(api::history::History::load(&mut j, &authorizer()).is_err());
    drop(j);
    let mut j = open(&t);
    assert!(api::history::History::load(&mut j, &authorizer()).is_err());
}

#[test]
fn pending_payouts_and_orders_share_capacity_and_unknown_evidence_blocks_new_risk() {
    let t = Temp::new();
    let mut j = seed(&t);
    let p = named(
        21,
        wire::Command::Payout {
            net: 1500,
            maximum_fee: 0,
            allow_partial: false,
            good_until: 70,
        },
        9,
    );
    assert_eq!(admit(&mut j, &p, 100, 10).unwrap().refusal, None);
    let second = named(
        22,
        wire::Command::Payout {
            net: 600,
            maximum_fee: 0,
            allow_partial: false,
            good_until: 70,
        },
        9,
    );
    assert!(admit(&mut j, &second, 100, 11).unwrap().refusal.is_some());
    assert_eq!(component(&j).funds().len(), 1);
    let r = named(23, order(), 9);
    assert!(admit(&mut j, &r, 1600, 11).unwrap().refusal.is_some());
    assert!(component(&j).orders().is_empty());
    let mut proposal = support::transaction(j.head(), 90, vec![], vec![]);
    proposal.at = 12;
    proposal.inputs.push(Input {
        source: financial_config().units()[0].ledger.sources[0].scope,
        source_cut: Some(10),
        authority_epoch: 1,
        observed_at: 12,
        raw: support::raw(b"actual-unnormalized-adverse-input"),
        event: None,
    });
    j.commit(units::Transaction::unit(uid(40), proposal))
        .unwrap();
    assert_eq!(component(&j).unresolved_raw(), 1);
    let r = named(24, order(), 9);
    assert!(admit(&mut j, &r, 100, 12).unwrap().refusal.is_some());
    assert_eq!(component(&j).unresolved_raw(), 1);
    assert!(
        j.state()
            .unwrap()
            .unit(uid(50))
            .unwrap()
            .orders()
            .is_empty()
    );
}
