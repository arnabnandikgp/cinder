//! Disposable offline accounts and synthetic policy. No RPC/venue/wallet access.
#[path = "../../journal/tests/support/mod.rs"]
mod support;
use cinder_api::{wire::*, *};
use cinder_journal::{
    Journal, collateral,
    encrypted::RecordCipher,
    model::*,
    orders, risk,
    sqlite::{Migration, SqliteBackend},
};
use cinder_kernel::{
    identity::*,
    ledger::{economics::EconomicChange, evidence::*, funds::*, *},
};
use ed25519_dalek::{Signer, SigningKey};
use support::{Temp, cash, event, key, p, q, raw, user};
use zeroize::Zeroizing;
type Store = Journal<SqliteBackend, RecordCipher>;

fn config() -> Config {
    let mut c = support::config();
    let mut source = c.sources[0];
    source.location = Location::Vault;
    source.scope.namespace = NamespaceId::new([8; 32]).unwrap();
    c.sources.push(source);
    c
}
fn cipher() -> RecordCipher {
    RecordCipher::new(Zeroizing::new([99; 32]), 1, [42; 32]).unwrap()
}
fn open(t: &Temp) -> Store {
    Journal::open(
        SqliteBackend::open(&t.db, Migration::None).unwrap(),
        cipher(),
        config(),
    )
    .unwrap()
}
struct Holds;
impl Admission for Holds {
    fn order_holds(&self, _: &State, intent: &orders::Intent) -> Result<Vec<Reservation>, Error> {
        // Synthetic qualified margin port; P09 still evaluates all pending outcomes.
        Ok(vec![
            Reservation {
                resource: Resource::Customer(intent.request.account),
                amount: cash(100),
            },
            Reservation {
                resource: Resource::Location(Location::Venue),
                amount: cash(100),
            },
        ])
    }
}
struct Channel([u8; 32]);
impl ConfidentialChannel for Channel {
    fn domain(&self) -> Domain {
        config().domain
    }
    fn binding(&self) -> [u8; 32] {
        self.0
    }
    fn expires_at(&self) -> u64 {
        100
    }
}
fn wallet(n: u8) -> SigningKey {
    SigningKey::from_bytes(&[8 + n; 32])
}
fn contract() -> Contract {
    Contract {
        owners: (1..=2)
            .map(|n| OwnerBinding {
                account: user(n),
                wallet: wallet(n).verifying_key().to_bytes(),
                tokens: [80 + n; 32],
            })
            .collect(),
        maximum_auth_lifetime: 100,
        maximum_grant_lifetime: 1000,
    }
}
fn service() -> Service<Holds> {
    Service::new(contract(), Holds).unwrap()
}
fn commit(
    s: &mut Store,
    n: u8,
    events: Vec<Event>,
    controls: Vec<Control>,
) -> cinder_journal::Committed {
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
    s.commit(tx).unwrap()
}
fn seed(t: &Temp) -> Store {
    let mut s = Journal::create(SqliteBackend::create(&t.db).unwrap(), cipher(), config()).unwrap();
    let vault = Event {
        key: RecordKey::Economic(EventKey {
            scope: config().sources[1].scope,
            event: EconomicEventId::new(b"vault").unwrap(),
            leg: 0,
        }),
        policy: config().policy,
        change: Change::Receipt {
            owner: Owner::Customer(user(1)),
            location: Location::Vault,
            amount: cash(1000),
        },
    };
    assert_eq!(
        commit(
            &mut s,
            1,
            vec![
                support::receipt(1, Owner::Customer(user(1)), 1000),
                support::receipt(2, Owner::Customer(user(2)), 1000),
                support::receipt(3, Owner::House, 1000),
                vault
            ],
            vec![]
        )
        .receipt
        .controls,
        None
    );
    let l = s.state().unwrap().ledger();
    let check = event(
        20,
        Change::Reconcile(NativeCheck {
            expected_version: l.version(),
            cash: Some(l.venue().cash()),
            funding: Some(l.venue().funding()),
            positions: Some(l.venue().positions().to_vec()),
            complete: true,
            resolves: vec![],
        }),
    );
    commit(&mut s, 2, vec![check], vec![]);
    let cut = collateral::Cut {
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
    let policy = risk::Policy {
        revision: config().policy,
        markets: vec![risk::MarketRule {
            market: q(0).unit(),
            maximum_leverage: 100_000,
            maintenance_bps: 500,
            native_maintenance_bps: 500,
            gross_limit: cash(1_000_000),
            net_limit: cash(1_000_000),
        }],
        buffer: cash(0),
        horizon_ms: 50,
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
                        accessible_bps: 10_000,
                        due: cash(0),
                    })
                    .collect(),
            }],
        }],
    };
    let expected_version = s.state().unwrap().ledger().version();
    assert_eq!(
        commit(
            &mut s,
            3,
            vec![],
            vec![
                Control::Collateral(cut),
                Control::Risk(risk::Action::Install {
                    expected_version,
                    policy: Box::new(policy)
                })
            ]
        )
        .receipt
        .controls,
        None
    );
    service().initialize(&mut s, 10).unwrap();
    s
}
fn order() -> Command {
    Command::Order {
        market: q(0).unit().market,
        lots: 2,
        minimum: 90,
        maximum: 110,
        fee: 1,
        tif: 0,
        reduce_only: false,
        good_until: 90,
    }
}
fn request(
    n: u8,
    customer: u8,
    epoch: u64,
    command: Command,
    signer: &SigningKey,
    binding: [u8; 32],
) -> Request {
    let mut req = Request {
        domain: config().domain,
        account: user(customer),
        id: RequestId::new([n; 32]).unwrap(),
        policy: config().policy,
        epoch,
        signer: signer.verifying_key().to_bytes(),
        session: binding,
        expires_at: 100,
        command,
        signature: [0; 64],
    };
    req.signature = signer.sign(&req.message()).to_bytes();
    req
}
fn call(s: &mut Store, req: &Request) -> Result<Response, Error> {
    service().handle(s, &Channel(req.session), &req.encode().unwrap(), 10)
}
fn receipt(response: Response) -> cinder_api::Receipt {
    match response {
        Response::Receipt(r) => r,
        _ => panic!("expected operation"),
    }
}
fn owner_request(n: u8, command: Command) -> Request {
    request(n, 1, 1, command, &wallet(1), [8; 32])
}
fn query(kind: u8, limit: u16, cursor: [u8; 40]) -> Command {
    Command::Read(reads::Query {
        kind,
        limit,
        cursor,
    })
}
fn page(r: Response) -> reads::Page {
    match r {
        Response::Read(p) => p,
        _ => panic!("expected private page"),
    }
}

#[test]
fn warm_and_cold_indexes_preserve_all_private_read_families_views_receipts_and_replay() {
    let t = Temp::new();
    let mut s = seed(&t);
    let api = service();
    let operation = owner_request(40, grant(&wallet(3)));
    api.handle(
        &mut s,
        &Channel(operation.session),
        &operation.encode().unwrap(),
        10,
    )
    .unwrap();
    let another = owner_request(41, grant(&wallet(4)));
    api.handle(
        &mut s,
        &Channel(another.session),
        &another.encode().unwrap(),
        10,
    )
    .unwrap();
    let binding = cinder_journal::read::Binding {
        stream: cinder_journal::replicated::Stream {
            domain: config().domain,
            id: [42; 32],
        },
        epoch: 1,
        api: api.release_commitment(s.configuration()).unwrap(),
    };
    let attach = |s: &mut Store| s.attach_reader(binding).unwrap();
    let reader = attach(&mut s);
    struct Witness(cinder_journal::replicated::Anchor);
    impl cinder_journal::read::Witness for Witness {
        fn read(
            &self,
            _: cinder_journal::replicated::Stream,
        ) -> Result<cinder_journal::replicated::Anchor, cinder_journal::Error> {
            Ok(self.0)
        }
    }
    let compare = |s: &mut Store, reader: &cinder_journal::read::Reader, req: &Request| {
        let warm = api
            .handle(s, &Channel(req.session), &req.encode().unwrap(), 10)
            .unwrap();
        let cold = service()
            .handle(s, &Channel(req.session), &req.encode().unwrap(), 10)
            .unwrap();
        assert_eq!(warm, cold);
        assert_eq!(warm.encode().unwrap(), cold.encode().unwrap());
        let verified = reader
            .verify(
                reader.capture().unwrap(),
                &Witness(cinder_journal::replicated::Anchor {
                    epoch: 1,
                    head: Some(s.head()),
                }),
            )
            .unwrap();
        let independent = api
            .handle_read(&verified, &Channel(req.session), &req.encode().unwrap(), 10)
            .unwrap();
        assert_eq!(warm, independent);
        assert_eq!(warm.encode().unwrap(), independent.encode().unwrap());
        api.revalidate_read(&verified, &Channel(req.session), &req.encode().unwrap(), 10)
            .unwrap();
        verified
            .release(std::time::Instant::now() + std::time::Duration::from_secs(1))
            .unwrap();
        warm
    };
    for kind in 0..reads::KINDS {
        let mut cursor = [0; 40];
        loop {
            let p = page(compare(
                &mut s,
                &reader,
                &owner_request(90, query(kind, 1, cursor)),
            ));
            cursor = p.next;
            if cursor == [0; 40] {
                break;
            }
        }
    }
    compare(&mut s, &reader, &owner_request(91, Command::View));
    compare(
        &mut s,
        &reader,
        &owner_request(92, Command::Operation(operation.id)),
    );
    let head = s.head();
    drop(s);
    let mut s = open(&t);
    let reader = attach(&mut s);
    for kind in 0..reads::KINDS {
        compare(
            &mut s,
            &reader,
            &owner_request(90, query(kind, 64, [0; 40])),
        );
    }
    compare(&mut s, &reader, &owner_request(91, Command::View));
    compare(
        &mut s,
        &reader,
        &owner_request(92, Command::Operation(operation.id)),
    );
    assert_eq!(s.head(), head);
}

#[test]
fn read_families_are_signed_scoped_bounded_and_do_not_commit() {
    let t = Temp::new();
    let mut s = seed(&t);
    let head = s.head();
    for kind in 0..reads::KINDS {
        let req = owner_request(90, query(kind, 64, [0; 40]));
        assert_eq!(
            Request::decode(req.encode().unwrap().as_bytes()).unwrap(),
            req
        );
        let p = page(call(&mut s, &req).unwrap());
        assert_eq!(p.kind, kind);
        assert!(p.rows.len() <= 64);
        assert!(p.revision.iter().any(|b| *b != 0));
        assert!(Response::Read(p).encode().unwrap().as_bytes().len() < 1_048_576);
        let foreign = request(90, 2, 1, query(kind, 64, [0; 40]), &wallet(1), [8; 32]);
        assert_eq!(call(&mut s, &foreign), Err(Error::Unauthorized));
    }
    assert_eq!(s.head(), head);
    for (kind, limit) in [(10, 1), (0, 0), (0, 65)] {
        assert_eq!(
            call(&mut s, &owner_request(91, query(kind, limit, [0; 40]))),
            Err(Error::Invalid)
        );
    }
}
#[test]
fn read_cursors_bind_own_query_and_changed_snapshot_but_not_other_activity() {
    let t = Temp::new();
    let mut s = seed(&t);
    call(&mut s, &owner_request(30, grant(&wallet(2)))).unwrap();
    call(&mut s, &owner_request(40, order())).unwrap();
    let first = page(call(&mut s, &owner_request(90, query(2, 1, [0; 40]))).unwrap());
    assert_ne!(first.next, [0; 40]);
    let other = request(31, 2, 1, grant(&wallet(1)), &wallet(2), [8; 32]);
    call(&mut s, &other).unwrap();
    let same = page(call(&mut s, &owner_request(90, query(2, 1, [0; 40]))).unwrap());
    assert_eq!(same, first);
    let second = page(call(&mut s, &owner_request(90, query(2, 1, first.next))).unwrap());
    assert_eq!(second.rows.len(), 1);
    assert_eq!(
        call(&mut s, &owner_request(90, query(3, 1, first.next))),
        Err(Error::Conflict)
    );
    call(
        &mut s,
        &owner_request(
            42,
            Command::Cancel {
                target: RequestId::new([40; 32]).unwrap(),
                attempt: AttemptId::new([43; 32]).unwrap(),
                good_until: 90,
            },
        ),
    )
    .unwrap();
    assert_eq!(
        call(&mut s, &owner_request(90, query(2, 1, first.next))),
        Err(Error::Conflict)
    );
    drop(s);
    let mut s = open(&t);
    let a = page(call(&mut s, &owner_request(90, query(2, 64, [0; 40]))).unwrap());
    let mut q = request(90, 1, 1, query(2, 64, [0; 40]), &wallet(1), [9; 32]);
    q.signature = wallet(1).sign(&q.message()).to_bytes();
    assert_eq!(page(call(&mut s, &q).unwrap()), a);
}
#[test]
fn read_agent_directory_and_history_do_not_leak_other_grant_keys() {
    let t = Temp::new();
    let mut s = seed(&t);
    let agent = wallet(2);
    let second = SigningKey::from_bytes(&[77; 32]);
    call(&mut s, &owner_request(30, grant(&agent))).unwrap();
    call(&mut s, &owner_request(31, grant(&second))).unwrap();
    let owner = page(call(&mut s, &owner_request(90, query(7, 64, [0; 40]))).unwrap());
    assert_eq!(owner.rows.len(), 2);
    for kind in [2, 7] {
        let q = request(90, 1, 1, query(kind, 64, [0; 40]), &agent, [8; 32]);
        let p = page(call(&mut s, &q).unwrap());
        assert!(p.rows.iter().all(|r| {
            !r.as_bytes()
                .windows(32)
                .any(|b| b == second.verifying_key().to_bytes())
        }));
        if kind == 7 {
            assert_eq!(p.rows.len(), 1);
        }
    }
    call(&mut s, &owner_request(32, Command::Revoke)).unwrap();
    assert_eq!(
        call(
            &mut s,
            &request(90, 1, 1, query(7, 64, [0; 40]), &agent, [8; 32])
        ),
        Err(Error::Unauthorized)
    );
}
#[test]
fn read_history_includes_ingested_book_effects_and_replays_without_duplicate_fills() {
    let t = Temp::new();
    let mut s = seed(&t);
    call(&mut s, &owner_request(40, order())).unwrap();
    let attempt = AttemptKey {
        request: RequestKey {
            domain: config().domain,
            account: user(1),
            request: RequestId::new([40; 32]).unwrap(),
        },
        attempt: AttemptId::new([40; 32]).unwrap(),
    };
    commit(
        &mut s,
        81,
        vec![],
        vec![
            Control::Order(orders::Action::Prepare { attempt }),
            Control::Expose(attempt),
        ],
    );
    let fill = event(
        81,
        Change::Economics(EconomicChange::Execution {
            target: FillTarget::Customer(attempt),
            quantity: q(1),
            price: p(100),
            fee: cash(1),
            pnl: None,
        }),
    );
    commit(&mut s, 82, vec![fill.clone()], vec![]);
    commit(&mut s, 83, vec![fill], vec![]);
    let f = page(call(&mut s, &owner_request(90, query(4, 64, [0; 40]))).unwrap());
    assert_eq!(f.rows.len(), 1);
    let h = page(call(&mut s, &owner_request(90, query(6, 64, [0; 40]))).unwrap());
    assert_eq!(h.rows.len(), 2);
    let head = s.head();
    assert_eq!(
        s.customer_book_history(user(1))
            .unwrap()
            .last()
            .unwrap()
            .after
            .cash()
            .atoms(),
        1999
    );
    assert_eq!(s.head(), head);
    drop(s);
    let mut s = open(&t);
    assert_eq!(
        page(call(&mut s, &owner_request(90, query(4, 64, [0; 40]))).unwrap()),
        f
    );
    assert_eq!(
        page(call(&mut s, &owner_request(90, query(6, 64, [0; 40]))).unwrap()),
        h
    );
}
#[test]
fn derived_risk_checks_read_clock_without_mutating_or_silently_refreshing_marks() {
    let t = Temp::new();
    let s = seed(&t);
    let before = s.state().unwrap().clone();
    assert!(before.risk_report_at(10).is_ok());
    assert!(before.risk_report_at(10_000).is_err());
    assert!(before.read_marks(10_000).is_err());
    assert_eq!(s.state().unwrap(), &before);
}
#[test]
fn read_funding_unknown_zero_and_settlement_are_distinct_and_owner_scoped() {
    let t = Temp::new();
    let mut s = seed(&t);
    let version = s.state().unwrap().ledger().version();
    commit(
        &mut s,
        84,
        vec![event(
            84,
            Change::Economics(EconomicChange::FundingBoundary {
                market: q(0).unit(),
                expected_version: version,
            }),
        )],
        vec![],
    );
    let unknown = page(call(&mut s, &owner_request(90, query(5, 64, [0; 40]))).unwrap());
    assert_eq!(unknown.rows.len(), 1);
    // market(36) + eligible lots(8), then private allocation's Option tag.
    assert_eq!(unknown.rows[0].as_bytes()[44], 0);
    assert_eq!(unknown.rows[0].as_bytes()[45], 0);
    commit(
        &mut s,
        85,
        vec![event(
            85,
            Change::Economics(EconomicChange::FundingInputs {
                boundary: key(84),
                rate: Some(cinder_kernel::ledger::economics::FundingRate {
                    numerator: cash(0),
                    denominator: 1,
                    rounding: cinder_kernel::math::Rounding::TowardZero,
                }),
                native: Some(cash(0)),
            }),
        )],
        vec![],
    );
    let known = page(call(&mut s, &owner_request(90, query(5, 64, [0; 40]))).unwrap());
    assert_eq!(known.rows[0].as_bytes()[44], 1);
    assert_eq!(&known.rows[0].as_bytes()[45..61], &[0; 16]);
    assert_eq!(known.rows[0].as_bytes()[61], 0);
    commit(
        &mut s,
        86,
        vec![event(
            86,
            Change::Economics(EconomicChange::FundingSettlement {
                boundary: key(84),
                native: cash(0),
            }),
        )],
        vec![],
    );
    let settled = page(call(&mut s, &owner_request(90, query(5, 64, [0; 40]))).unwrap());
    assert_eq!(settled.rows[0].as_bytes()[61], 1);
    assert_eq!(
        s.state()
            .unwrap()
            .ledger()
            .funding_views(Owner::Customer(user(1)))[0]
            .payment,
        Some(cash(0))
    );
    drop(s);
    let mut s = open(&t);
    assert_eq!(
        page(call(&mut s, &owner_request(90, query(5, 64, [0; 40]))).unwrap()),
        settled
    );
}
fn grant(agent: &SigningKey) -> Command {
    Command::Grant(Grant {
        key: agent.verifying_key().to_bytes(),
        methods: READ | TRADE | CANCEL,
        market: q(0).unit().market,
        maximum_lots: 2,
        maximum_fee: 1,
        maximum_orders: 1,
        expires_at: 90,
    })
}

#[test]
fn exact_retry_survives_restart_and_new_session_without_new_native_attempt() {
    let t = Temp::new();
    let mut s = seed(&t);
    let req = owner_request(40, order());
    let first = receipt(call(&mut s, &req).unwrap());
    assert_eq!(first.outcome, Outcome::Accepted);
    let head = s.head();
    drop(s);
    let mut s = open(&t);
    let new = request(40, 1, 1, order(), &wallet(1), [7; 32]);
    assert_eq!(receipt(call(&mut s, &new).unwrap()), first);
    assert_eq!(s.head(), head);
    assert!(s.state().unwrap().attempts().is_empty());
    let mut changed = order();
    if let Command::Order { lots, .. } = &mut changed {
        *lots = 1;
    }
    assert_eq!(
        call(&mut s, &owner_request(40, changed)),
        Err(Error::Conflict)
    );
}
#[test]
fn request_ids_are_account_scoped_and_replay_preserves_history_order() {
    let t = Temp::new();
    let mut s = seed(&t);
    let first = receipt(call(&mut s, &owner_request(40, order())).unwrap());
    let second = receipt(call(&mut s, &request(40, 2, 1, order(), &wallet(2), [8; 32])).unwrap());
    assert_eq!(first.outcome, Outcome::Accepted);
    assert_eq!(second.outcome, Outcome::Accepted);
    let next = receipt(call(&mut s, &owner_request(41, grant(&wallet(3)))).unwrap());
    let head = s.head();
    drop(s);
    let mut s = open(&t);
    for (customer, expected) in [(1, &first), (2, &second)] {
        assert_eq!(
            receipt(
                call(
                    &mut s,
                    &request(
                        42,
                        customer,
                        1,
                        Command::Operation(RequestId::new([40; 32]).unwrap()),
                        &wallet(customer),
                        [7; 32],
                    ),
                )
                .unwrap(),
            ),
            *expected,
        );
    }
    let Response::View(view) = call(&mut s, &owner_request(43, Command::View)).unwrap() else {
        panic!("expected account view");
    };
    assert_eq!(view.operations, vec![first.id, next.id]);
    assert_eq!(s.head(), head);
}
#[test]
fn duplicate_retained_api_record_fails_closed_before_and_after_restart() {
    let t = Temp::new();
    let mut s = seed(&t);
    call(&mut s, &owner_request(40, order())).unwrap();
    let duplicate = s
        .transactions()
        .flat_map(|tx| &tx.evidence)
        .find(|e| e.as_bytes().starts_with(b"CINDER-API-RECORD-1\0"))
        .unwrap()
        .as_bytes()
        .to_vec();
    let mut tx = support::transaction(s.head(), 50, vec![], vec![]);
    tx.evidence.push(PrivateBytes::new(duplicate).unwrap());
    assert!(s.commit(tx).unwrap().receipt.controls.is_none());
    let head = s.head();
    assert_eq!(
        call(&mut s, &owner_request(41, Command::View)),
        Err(Error::Unavailable),
    );
    assert_eq!(s.head(), head);
    drop(s);
    let mut s = open(&t);
    assert_eq!(
        call(&mut s, &owner_request(41, Command::View)),
        Err(Error::Unavailable),
    );
    assert_eq!(s.head(), head);
}
#[test]
fn authentication_precedes_dedupe_query_and_foreign_cancel() {
    let t = Temp::new();
    let mut s = seed(&t);
    call(&mut s, &owner_request(40, order())).unwrap();
    let head = s.head();
    for cmd in [
        order(),
        Command::Operation(RequestId::new([40; 32]).unwrap()),
        Command::Cancel {
            target: RequestId::new([40; 32]).unwrap(),
            attempt: AttemptId::new([50; 32]).unwrap(),
            good_until: 90,
        },
    ] {
        assert_eq!(
            call(&mut s, &request(40, 1, 1, cmd, &wallet(2), [8; 32])),
            Err(Error::Unauthorized)
        );
    }
    assert_eq!(
        call(
            &mut s,
            &request(
                44,
                2,
                1,
                Command::Operation(RequestId::new([40; 32]).unwrap()),
                &wallet(2),
                [8; 32]
            )
        ),
        Err(Error::NotFound)
    );
    assert_eq!(s.head(), head);
}
#[test]
fn tampered_domain_signature_session_policy_expiry_epoch_and_clock_refuse() {
    let t = Temp::new();
    let mut s = seed(&t);
    let req = owner_request(40, Command::View);
    let mut bad = req.clone();
    bad.signature[0] ^= 1;
    assert_eq!(call(&mut s, &bad), Err(Error::Unauthorized));
    let mut bad = req.clone();
    bad.domain.deployment = DeploymentId::new([99; 32]).unwrap();
    assert_eq!(call(&mut s, &bad), Err(Error::Unauthorized));
    let mut bad = req.clone();
    bad.policy = PolicyVersion::new(2).unwrap();
    assert_eq!(call(&mut s, &bad), Err(Error::Unauthorized));
    assert_eq!(
        service().handle(&mut s, &Channel([1; 32]), &req.encode().unwrap(), 10),
        Err(Error::Unauthorized)
    );
    assert_eq!(
        service().handle(&mut s, &Channel([8; 32]), &req.encode().unwrap(), 100),
        Err(Error::Unauthorized)
    );
    assert_eq!(
        service().handle(&mut s, &Channel([8; 32]), &req.encode().unwrap(), 9),
        Err(Error::Unavailable)
    );
    assert_eq!(
        call(
            &mut s,
            &request(40, 1, 2, Command::View, &wallet(1), [8; 32])
        ),
        Err(Error::Unauthorized)
    );
}
#[test]
fn grant_budgets_permissions_and_revocation_are_durable() {
    let t = Temp::new();
    let mut s = seed(&t);
    let agent = SigningKey::from_bytes(&[50; 32]);
    call(&mut s, &owner_request(30, grant(&agent))).unwrap();
    let a = request(40, 1, 1, order(), &agent, [8; 32]);
    call(&mut s, &a).unwrap();
    drop(s);
    let mut s = open(&t);
    assert_eq!(
        receipt(call(&mut s, &a).unwrap()).outcome,
        Outcome::Accepted
    );
    assert_eq!(
        call(&mut s, &request(41, 1, 1, order(), &agent, [8; 32])),
        Err(Error::Unauthorized)
    );
    for command in [
        Command::Revoke,
        grant(&wallet(2)),
        Command::Payout {
            net: 10,
            maximum_fee: 1,
            allow_partial: false,
            good_until: 90,
        },
        Command::Leverage {
            market: q(0).unit().market,
            leverage: 20_000,
            good_until: 90,
        },
    ] {
        assert_eq!(
            call(&mut s, &request(60, 1, 1, command, &agent, [8; 32])),
            Err(Error::Unauthorized)
        );
    }
    call(&mut s, &owner_request(70, Command::Revoke)).unwrap();
    assert_eq!(call(&mut s, &a), Err(Error::Unauthorized));
    assert_eq!(
        call(&mut s, &request(71, 1, 2, Command::View, &agent, [8; 32])),
        Err(Error::Unauthorized)
    );
    let head = s.head();
    assert_eq!(
        receipt(call(&mut s, &request(40, 1, 2, order(), &wallet(1), [9; 32])).unwrap()).outcome,
        Outcome::Accepted
    );
    assert_eq!(s.head(), head);
    let attempt = AttemptKey {
        request: RequestKey {
            domain: config().domain,
            account: user(1),
            request: RequestId::new([40; 32]).unwrap(),
        },
        attempt: AttemptId::new([1; 32]).unwrap(),
    };
    assert_eq!(
        commit(
            &mut s,
            90,
            vec![],
            vec![Control::Order(orders::Action::Prepare { attempt })]
        )
        .receipt
        .controls,
        None
    );
    let refused = commit(&mut s, 91, vec![], vec![Control::Expose(attempt)]);
    assert_eq!(refused.receipt.controls, Some(ControlError::Invalid));
    assert!(refused.exposures.is_empty());
}
#[test]
fn grants_bind_market_size_fee_methods_expiry_and_cannot_be_replaced() {
    let t = Temp::new();
    let mut s = seed(&t);
    let agent = SigningKey::from_bytes(&[50; 32]);
    call(&mut s, &owner_request(30, grant(&agent))).unwrap();
    for (lots, fee, market) in [
        (3, 1, q(0).unit().market),
        (2, 2, q(0).unit().market),
        (2, 1, MarketId::new([99; 32]).unwrap()),
    ] {
        let mut cmd = order();
        if let Command::Order {
            lots: l,
            fee: f,
            market: m,
            ..
        } = &mut cmd
        {
            *l = lots;
            *f = fee;
            *m = market;
        }
        assert_eq!(
            call(&mut s, &request(40, 1, 1, cmd, &agent, [8; 32])),
            Err(Error::Unauthorized)
        );
    }
    assert_eq!(
        call(&mut s, &owner_request(31, grant(&agent))),
        Err(Error::Invalid)
    );
    let req = request(45, 1, 1, Command::View, &agent, [8; 32]);
    assert_eq!(
        service().handle(&mut s, &Channel([8; 32]), &req.encode().unwrap(), 90),
        Err(Error::Unauthorized)
    );
}
#[test]
fn view_contains_only_own_book_and_local_ids_other_activity_does_not_change_bytes() {
    let t = Temp::new();
    let mut s = seed(&t);
    let req = owner_request(40, Command::View);
    let before = call(&mut s, &req).unwrap().encode().unwrap();
    call(&mut s, &request(44, 2, 1, order(), &wallet(2), [8; 32])).unwrap();
    assert_eq!(call(&mut s, &req).unwrap().encode().unwrap(), before);
    let Response::View(v) = call(&mut s, &req).unwrap() else {
        panic!()
    };
    assert_eq!(v.cash, 2000);
    assert_eq!(v.funding, 0);
    assert!(v.operations.is_empty());
    assert_eq!(
        format!("{req:?} {v:?} {:?}", contract()),
        "[PRIVATE API] [PRIVATE API] [PRIVATE API]"
    );
}
#[test]
fn unexposed_cancel_is_scoped_and_restores_hold_without_venue_send() {
    let t = Temp::new();
    let mut s = seed(&t);
    call(&mut s, &owner_request(40, order())).unwrap();
    let cancel = Command::Cancel {
        target: RequestId::new([40; 32]).unwrap(),
        attempt: AttemptId::new([50; 32]).unwrap(),
        good_until: 90,
    };
    assert_eq!(
        receipt(call(&mut s, &owner_request(50, cancel.clone())).unwrap()).outcome,
        Outcome::Complete
    );
    assert!(s.state().unwrap().orders()[0].abandoned);
    assert!(s.state().unwrap().attempts().is_empty());
    assert_eq!(
        call(&mut s, &request(51, 2, 1, cancel, &wallet(2), [8; 32])),
        Err(Error::NotFound)
    );
}
#[test]
fn agent_can_cancel_only_its_original_market_orders_not_owner_orders() {
    let t = Temp::new();
    let mut s = seed(&t);
    let agent = SigningKey::from_bytes(&[50; 32]);
    call(&mut s, &owner_request(30, grant(&agent))).unwrap();
    call(&mut s, &owner_request(40, order())).unwrap();
    let cancel = Command::Cancel {
        target: RequestId::new([40; 32]).unwrap(),
        attempt: AttemptId::new([50; 32]).unwrap(),
        good_until: 90,
    };
    assert_eq!(
        call(&mut s, &request(50, 1, 1, cancel, &agent, [8; 32])),
        Err(Error::Unauthorized)
    );
}
#[test]
fn ack_partial_and_unknown_never_become_fills_or_retry_authority() {
    let t = Temp::new();
    let mut s = seed(&t);
    call(&mut s, &owner_request(40, order())).unwrap();
    let attempt = AttemptKey {
        request: RequestKey {
            domain: config().domain,
            account: user(1),
            request: RequestId::new([40; 32]).unwrap(),
        },
        attempt: AttemptId::new([40; 32]).unwrap(),
    };
    assert_eq!(
        commit(
            &mut s,
            81,
            vec![],
            vec![
                Control::Order(orders::Action::Prepare { attempt }),
                Control::Expose(attempt)
            ]
        )
        .receipt
        .controls,
        None
    );
    let query = owner_request(60, Command::Operation(attempt.request.request));
    assert_eq!(
        receipt(call(&mut s, &query).unwrap()).outcome,
        Outcome::Dispatched
    );
    let mut tx = support::transaction(s.head(), 82, vec![], vec![]);
    tx.order_observations = vec![orders::Observation {
        key: key(80),
        attempt,
        status: orders::Status::Acknowledged,
        authority_epoch: 1,
        observed_at: 10,
        raw: raw(b"synthetic-ack"),
    }];
    s.commit(tx).unwrap();
    let ack = receipt(call(&mut s, &query).unwrap());
    assert_eq!(ack.filled, 0);
    assert_eq!(ack.outcome, Outcome::Acknowledged);
    commit(
        &mut s,
        83,
        vec![event(
            81,
            Change::Economics(EconomicChange::Execution {
                target: FillTarget::Customer(attempt),
                quantity: q(1),
                price: p(100),
                fee: cash(1),
                pnl: None,
            }),
        )],
        vec![],
    );
    let partial = receipt(call(&mut s, &query).unwrap());
    assert_eq!(partial.filled, 1);
    assert_eq!(partial.outcome, Outcome::Partial);
    assert_eq!(partial.fee_cap, 2);
    let mut tx = support::transaction(s.head(), 84, vec![], vec![]);
    tx.order_observations = vec![orders::Observation {
        key: key(82),
        attempt,
        status: orders::Status::Unknown,
        authority_epoch: 1,
        observed_at: 10,
        raw: raw(b"synthetic-timeout"),
    }];
    s.commit(tx).unwrap();
    assert_eq!(
        receipt(call(&mut s, &query).unwrap()).outcome,
        Outcome::Unknown
    );
    let head = s.head();
    call(&mut s, &owner_request(40, order())).unwrap();
    assert_eq!(s.head(), head);
    assert_eq!(s.state().unwrap().attempts().len(), 1);
}
#[test]
fn payout_is_owner_bound_net_plus_fee_and_partial_consent_remains_explicit() {
    let t = Temp::new();
    let mut s = seed(&t);
    let cmd = Command::Payout {
        net: 10,
        maximum_fee: 2,
        allow_partial: true,
        good_until: 90,
    };
    let r = receipt(call(&mut s, &owner_request(40, cmd)).unwrap());
    assert_eq!(r.outcome, Outcome::Accepted);
    assert_eq!(r.paid, 0);
    assert_eq!(r.rail_fees, 0);
    assert_eq!(r.fee_cap, 2);
    assert!(r.allow_partial);
    let op = &s.state().unwrap().funds()[0];
    assert_eq!(op.intent.destination, Destination::Recipient([81; 32]));
    assert_eq!(op.intent.fee_payer, Owner::Customer(user(1)));
    assert_eq!(op.intent.net.atoms(), 10);
}
#[test]
fn rejected_control_is_durable_and_same_id_cannot_retry_with_different_action() {
    let t = Temp::new();
    let mut s = seed(&t);
    let mut cmd = order();
    if let Command::Order { lots, .. } = &mut cmd {
        *lots = 1_000_000;
    }
    let req = owner_request(40, cmd);
    assert_eq!(
        receipt(call(&mut s, &req).unwrap()).outcome,
        Outcome::Rejected
    );
    let head = s.head();
    drop(s);
    let mut s = open(&t);
    assert_eq!(
        receipt(call(&mut s, &req).unwrap()).outcome,
        Outcome::Rejected
    );
    assert_eq!(s.head(), head);
    assert_eq!(
        call(&mut s, &owner_request(40, Command::Revoke)),
        Err(Error::Conflict)
    );
}
#[test]
fn changed_owner_contract_or_missing_joined_risk_cannot_silently_initialize() {
    let t = Temp::new();
    let mut s = seed(&t);
    let mut changed = contract();
    changed.owners[0].tokens = [88; 32];
    let other = Service::new(changed, Holds).unwrap();
    assert_eq!(other.initialize(&mut s, 10), Err(Error::Unavailable));
    assert_eq!(
        other.handle(
            &mut s,
            &Channel([8; 32]),
            &owner_request(40, Command::View).encode().unwrap(),
            10
        ),
        Err(Error::Unavailable)
    );
    let t = Temp::new();
    let mut s = Journal::create(SqliteBackend::create(&t.db).unwrap(), cipher(), config()).unwrap();
    service().initialize(&mut s, 10).unwrap();
    assert_eq!(
        call(&mut s, &owner_request(40, order())),
        Err(Error::Unavailable)
    );
}
#[test]
fn strict_parser_trailing_unknown_boolean_and_oversize_are_rejected() {
    let req = owner_request(40, order());
    let bytes = req.encode().unwrap();
    assert_eq!(Request::decode(bytes.as_bytes()).unwrap(), req);
    for len in 0..bytes.as_bytes().len() {
        assert_eq!(
            Request::decode(&bytes.as_bytes()[..len]),
            Err(Error::Invalid)
        );
    }
    let mut trailing = bytes.as_bytes().to_vec();
    trailing.push(0);
    assert_eq!(Request::decode(&trailing), Err(Error::Invalid));
    assert_eq!(
        Request::decode(&vec![1; MAX_REQUEST + 1]),
        Err(Error::Invalid)
    );
    let mut bad = bytes.as_bytes().to_vec();
    let command_at = req.message().len() - 83;
    bad[command_at] = 255;
    assert_eq!(Request::decode(&bad), Err(Error::Invalid));
    let mut bad = bytes.as_bytes().to_vec();
    bad[req.message().len() - 9] = 2;
    assert_eq!(Request::decode(&bad), Err(Error::Invalid));
}

#[test]
fn concurrent_cas_loser_reloads_and_never_exposes_a_duplicate_action() {
    let t = Temp::new();
    let mut s = seed(&t);
    let mut stale = open(&t);
    let req = owner_request(40, order());
    call(&mut s, &req).unwrap();
    assert_eq!(call(&mut stale, &req), Err(Error::Unavailable));
    stale.reload().unwrap();
    let head = stale.head();
    assert_eq!(
        receipt(call(&mut stale, &req).unwrap()).outcome,
        Outcome::Accepted
    );
    assert_eq!(stale.head(), head);
    assert!(stale.state().unwrap().attempts().is_empty());
}
#[test]
fn grant_dispatch_deadline_and_trade_only_replay_cannot_bypass_scope() {
    let t = Temp::new();
    let mut s = seed(&t);
    let agent = SigningKey::from_bytes(&[50; 32]);
    let mut g = grant(&agent);
    if let Command::Grant(g) = &mut g {
        g.methods = TRADE;
        g.expires_at = 50;
    }
    call(&mut s, &owner_request(30, g)).unwrap();
    assert_eq!(
        call(&mut s, &request(40, 1, 1, order(), &agent, [8; 32])),
        Err(Error::Unauthorized)
    );
    let mut cmd = order();
    if let Command::Order { good_until, .. } = &mut cmd {
        *good_until = 50;
    }
    call(&mut s, &owner_request(40, cmd.clone())).unwrap();
    assert_eq!(
        call(&mut s, &request(40, 1, 1, cmd, &agent, [8; 32])),
        Err(Error::Unauthorized)
    );
    assert_eq!(
        call(&mut s, &request(41, 1, 1, Command::View, &agent, [8; 32])),
        Err(Error::Unauthorized)
    );
}
#[test]
fn payout_source_debit_does_not_discharge_claim_but_partial_beneficiary_receipt_does() {
    let t = Temp::new();
    let mut s = seed(&t);
    call(
        &mut s,
        &owner_request(
            40,
            Command::Payout {
                net: 10,
                maximum_fee: 2,
                allow_partial: true,
                good_until: 90,
            },
        ),
    )
    .unwrap();
    let attempt = AttemptKey {
        request: RequestKey {
            domain: config().domain,
            account: user(1),
            request: RequestId::new([40; 32]).unwrap(),
        },
        attempt: AttemptId::new([40; 32]).unwrap(),
    };
    assert_eq!(
        commit(
            &mut s,
            81,
            vec![],
            vec![
                Control::Funds(cinder_journal::funds::Action::Prepare {
                    attempt,
                    net: cash(5)
                }),
                Control::Expose(attempt)
            ]
        )
        .receipt
        .controls,
        None
    );
    let leg = |n: u8, leg: Leg, amount, fee| Event {
        key: RecordKey::Economic(EventKey {
            scope: config().sources[1].scope,
            event: EconomicEventId::new(&[n]).unwrap(),
            leg: 0,
        }),
        policy: config().policy,
        change: Change::Funds(FundsChange::Observe {
            attempt,
            leg,
            amount: cash(amount),
            fee: cash(fee),
        }),
    };
    let query = owner_request(60, Command::Operation(attempt.request.request));
    commit(&mut s, 82, vec![leg(80, Leg::Debit, 6, 0)], vec![]);
    let r = receipt(call(&mut s, &query).unwrap());
    assert_eq!(r.paid, 0);
    assert_eq!(r.outcome, Outcome::Dispatched);
    commit(
        &mut s,
        83,
        vec![leg(81, Leg::Arrive(Destination::Recipient([81; 32])), 5, 1)],
        vec![],
    );
    let r = receipt(call(&mut s, &query).unwrap());
    assert_eq!(r.paid, 4);
    assert_eq!(r.rail_fees, 1);
    assert_eq!(r.outcome, Outcome::Partial);
}
#[test]
fn exposed_cancel_prepares_one_distinct_attempt_and_ack_does_not_prove_terminal() {
    let t = Temp::new();
    let mut s = seed(&t);
    call(&mut s, &owner_request(40, order())).unwrap();
    let request = RequestKey {
        domain: config().domain,
        account: user(1),
        request: RequestId::new([40; 32]).unwrap(),
    };
    let original = AttemptKey {
        request,
        attempt: AttemptId::new([40; 32]).unwrap(),
    };
    assert_eq!(
        commit(
            &mut s,
            81,
            vec![],
            vec![
                Control::Order(orders::Action::Prepare { attempt: original }),
                Control::Expose(original)
            ]
        )
        .receipt
        .controls,
        None
    );
    let cmd = Command::Cancel {
        target: request.request,
        attempt: AttemptId::new([50; 32]).unwrap(),
        good_until: 90,
    };
    assert_eq!(
        receipt(call(&mut s, &owner_request(50, cmd.clone())).unwrap()).outcome,
        Outcome::Accepted
    );
    let cancel = AttemptKey {
        request,
        attempt: AttemptId::new([50; 32]).unwrap(),
    };
    assert_eq!(
        commit(&mut s, 82, vec![], vec![Control::Expose(cancel)])
            .receipt
            .controls,
        None
    );
    let mut tx = support::transaction(s.head(), 83, vec![], vec![]);
    tx.order_observations = vec![orders::Observation {
        key: key(80),
        // The qualified native status names the original order, not its cancel wire.
        attempt: original,
        status: orders::Status::CancelAcknowledged,
        authority_epoch: 1,
        observed_at: 10,
        raw: raw(b"synthetic-cancel-ack"),
    }];
    s.commit(tx).unwrap();
    let r = receipt(
        call(
            &mut s,
            &owner_request(60, Command::Operation(RequestId::new([50; 32]).unwrap())),
        )
        .unwrap(),
    );
    assert_eq!(r.outcome, Outcome::Acknowledged);
    assert!(r.possibly_exposed);
    let head = s.head();
    call(&mut s, &owner_request(50, cmd)).unwrap();
    assert_eq!(s.head(), head);
    assert_eq!(s.state().unwrap().attempts().len(), 2);
}
