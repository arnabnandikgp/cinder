//! Governed allocation with the actual original-deposit recognizer, P08 journal
//! and separated chain codec/signers. RPC, setup reads and time are synthetic.
use super::*;
use crate::demo_funding::{Authorization, Policy as Allocation, Step};

pub(crate) fn original() -> crate::customer_deposit::Locator {
    let v: Value = serde_json::from_str(include_str!(
        "../../pacifica/tests/fixtures/customer-deposit-public.json"
    ))
    .unwrap();
    let wire: Vec<u8> = serde_json::from_value(v["deposits"][0]["wire"].clone()).unwrap();
    crate::customer_deposit::Locator {
        account: [1; 32],
        operation: [41; 32],
        signature: wire[1..65].to_vec(),
    }
}
pub(crate) fn allocation() -> Allocation {
    Allocation {
        revision: 1,
        original: original(),
        amount: 20_000_000,
        release_request: [10; 32],
        deposit_request: [11; 32],
        authority_epoch: 1,
        expires_at: 10_000,
    }
}
fn bind(c: &Controller, g: &Gateway, p: Allocation) -> Authorization {
    let chain = loaded(c, vec![original()]);
    Authorization::bind(p, &config(), c, g, &demo_tests::policy(), &chain).unwrap()
}
fn workflow(temp: &support::Temp, recognize: bool) -> (support::Store, Controller, Gateway) {
    fresh(
        Journal::create(
            SqliteBackend::create(&temp.db).unwrap(),
            support::FixtureProtection,
            config(),
        )
        .unwrap(),
        recognize,
    )
}
pub(crate) fn fresh<B: Backend, P: Protection>(
    mut j: Journal<B, P>,
    recognize: bool,
) -> (Journal<B, P>, Controller, Gateway) {
    let mut profile = profile();
    profile.fills = Level::Unknown;
    let mut route = route();
    route.withdrawal = Level::Unknown;
    route.settings = Level::Observed;
    let c = Controller::new_demo_ingress(profile.clone(), route, Zeroizing::new([9; 32])).unwrap();
    if recognize {
        let f = fake(10);
        let mut port = deposit(&f, &c);
        assert!(port.observe_deposit(&mut j, &c, 0).unwrap() == Outcome::Settled);
    } else {
        // Cash is available, but this unrelated receipt is not the approved
        // original owner-signed deposit and must not authorize an allocation.
        let t = tx(
            &j,
            1,
            vec![(
                Location::Vault,
                Change::Receipt {
                    owner: Owner::Customer(support::user(1)),
                    location: Location::Vault,
                    amount: atoms(100_000_000),
                },
            )],
            vec![],
        );
        j.commit(t).unwrap();
    }
    let t = tx(
        &j,
        2,
        vec![],
        vec![Control::Order(orders::Action::AdvanceAuthority {
            account: support::user(1),
            epoch: 1,
        })],
    );
    assert_eq!(j.commit(t).unwrap().receipt.controls, None);
    c.bind(&mut j, id(3), 100).unwrap();
    let g = Gateway::new_read_only(
        profile,
        Policy {
            revision: 1,
            evidence: "OFFLINE read-only demo port fixture".into(),
            execution: Level::Unknown,
            origin: Origin::Testnet,
            expiry_ms: 3000,
            credits: 10000,
            cleanup_reserve: 100,
            read_cost: cinder_pacifica::reads::MIN_READ_COST,
        },
        Zeroizing::new([10; 32]),
        1,
    )
    .unwrap();
    demo_tests::qualify(&mut j, &c, &g);
    (j, c, g)
}
fn next(
    a: &Authorization,
    j: &mut support::Store,
    c: &Controller,
    g: &Gateway,
    at: u64,
) -> Result<Step, Error> {
    a.accept_next(j, c, g, &demo_tests::policy(), at)
}
fn flat_template() -> lifecycle::Action {
    let temp = support::Temp::new();
    let (mut j, c, g) = workflow(&temp, true);
    assert_eq!(
        next(&bind(&c, &g, allocation()), &mut j, &c, &g, 2100).unwrap(),
        Step::Accepted
    );
    let Control::Funds(action) = &j.transactions().last().unwrap().controls[0] else {
        panic!("funds")
    };
    action.clone()
}

#[test]
fn flat_certificate_cannot_change_receipt_route_amount_owner_or_terms() {
    let template = flat_template();
    for case in 0..16 {
        let temp = support::Temp::new();
        let (mut j, _, _) = workflow(&temp, true);
        let lifecycle::Action::FlatAccept {
            mut intent,
            mut approval,
            mut allocation,
        } = template.clone()
        else {
            panic!("flat")
        };
        match case {
            0 => allocation.authorization = [0; 32],
            1 => allocation.original.event = EconomicEventId::new(&[99]).unwrap(),
            2 => allocation.original_amount = atoms(99_999_999),
            3 => allocation.deposit.net = atoms(19_000_000),
            4 => allocation.deposit.request = allocation.release.request,
            5 => allocation.deposit.source = Location::Vault,
            6 => allocation.release.maximum_fee = atoms(1),
            7 => allocation.release.allow_partial = true,
            8 => allocation.release.destination = Destination::Recipient([1; 32]),
            9 => allocation.deposit.request.account = support::user(2),
            10 => allocation.release.policy = PolicyVersion::new(99).unwrap(),
            11 => {
                allocation.release.expires_at = 2100;
                allocation.deposit.expires_at = 2100;
            }
            12 => {
                allocation.deposit.request.domain.deployment = DeploymentId::new([99; 32]).unwrap()
            }
            13 => {
                allocation.release.net = atoms(19_000_000);
                allocation.deposit.net = atoms(19_000_000);
            }
            14 => allocation.deposit.authority_epoch = 2,
            _ => allocation.original.scope = config().sources[0].scope,
        }
        // Even a matching trusted intent digest cannot legitimize an invalid
        // certificate. Case 13 instead tests an intent outside its fixed grant.
        if case != 13 {
            *intent = allocation.release.clone();
        }
        approval.intent_hash = intent.digest().unwrap();
        let mut t = tx(
            &j,
            95,
            vec![],
            vec![Control::Funds(lifecycle::Action::FlatAccept {
                intent,
                approval,
                allocation,
            })],
        );
        t.at = 2100;
        let state = j.state().unwrap().clone();
        assert!(
            j.commit(t).unwrap().receipt.controls.is_some(),
            "substitution {case}"
        );
        assert_eq!(j.state().unwrap().ledger(), state.ledger());
        assert!(j.state().unwrap().funds().is_empty());
        assert!(j.state().unwrap().attempts().is_empty());
    }
}

#[test]
fn flat_prepare_rechecks_cash_exposure_reservations_and_containment() {
    for case in 0..7 {
        let temp = support::Temp::new();
        let (mut j, c, g) = workflow(&temp, true);
        assert_eq!(
            next(&bind(&c, &g, allocation()), &mut j, &c, &g, 2100).unwrap(),
            Step::Accepted
        );
        let (events, controls) = match case {
            0 => (vec![], vec![Control::Funds(lifecycle::Action::Freeze)]),
            1 => (
                vec![],
                vec![Control::Order(orders::Action::AdvanceAuthority {
                    account: support::user(1),
                    epoch: 2,
                })],
            ),
            2 => (
                vec![(
                    Location::Venue,
                    Change::Fill {
                        target: FillTarget::House,
                        quantity: support::q(1),
                        price: support::p(100),
                    },
                )],
                vec![],
            ),
            3 => (
                vec![(
                    Location::Vault,
                    Change::Receipt {
                        owner: Owner::Customer(support::user(1)),
                        location: Location::Vault,
                        amount: atoms(1),
                    },
                )],
                vec![],
            ),
            4 => (
                vec![],
                vec![Control::Reserve {
                    request: support::request(94),
                    reservations: vec![Reservation {
                        resource: Resource::Customer(support::user(1)),
                        amount: atoms(1),
                    }],
                }],
            ),
            5 => (
                vec![],
                vec![Control::Funds(lifecycle::Action::NativeCreditReady(true))],
            ),
            _ => (vec![], vec![]),
        };
        let mut change = tx(&j, 94, events, controls);
        change.at = if case == 6 { 10_000 } else { 2100 };
        assert!(j.commit(change).unwrap().receipt.controls.is_none());
        if case == 6 {
            assert_eq!(j.state().unwrap().logical_time(), 10_000);
        }
        let state = j.state().unwrap().clone();
        let mut prepare = tx(
            &j,
            95,
            vec![],
            vec![Control::Funds(lifecycle::Action::Prepare {
                attempt: support::attempt(10),
                net: atoms(20_000_000),
            })],
        );
        prepare.at = state.logical_time();
        assert!(
            j.commit(prepare).unwrap().receipt.controls.is_some(),
            "prepare {case}"
        );
        assert_eq!(j.state().unwrap().ledger(), state.ledger());
        assert!(j.state().unwrap().attempts().is_empty());
        assert!(j.state().unwrap().funds()[0].attempt.is_none());
    }
}

#[test]
fn flat_exposure_rechecks_after_preparation_without_releasing_original_holds() {
    for freeze in [false, true] {
        let temp = support::Temp::new();
        let (mut j, c, g) = workflow(&temp, true);
        assert_eq!(
            next(&bind(&c, &g, allocation()), &mut j, &c, &g, 2100).unwrap(),
            Step::Accepted
        );
        c.prepare_demo_ingress(
            &mut j,
            &g,
            &demo_tests::policy(),
            Dispatch {
                attempt: support::attempt(10),
                commit: id(90),
                at: 2100,
            },
        )
        .unwrap();
        let control = if freeze {
            Control::Funds(lifecycle::Action::Freeze)
        } else {
            Control::Order(orders::Action::AdvanceAuthority {
                account: support::user(1),
                epoch: 2,
            })
        };
        let mut t = tx(&j, 91, vec![], vec![control]);
        t.at = 2100;
        assert!(j.commit(t).unwrap().receipt.controls.is_none());
        let f = fake(10);
        f.0.lock().unwrap().at = 2100;
        let mut port = loaded(&c, vec![original()])
            .with_transport(f.clone(), Arc::new(Advancing(AtomicU64::new(2100))))
            .unwrap();
        assert!(
            port.issue_demo_ingress(&mut j, &c, &g, &demo_tests::policy(), support::attempt(10))
                .is_err()
        );
        assert!(
            c.retained_wire(&mut j, support::attempt(10), 2100)
                .unwrap()
                .is_none()
        );
        assert!(
            f.0.lock()
                .unwrap()
                .requests
                .iter()
                .all(|r| r["method"] != "sendTransaction")
        );
        assert_eq!(
            j.state()
                .unwrap()
                .reserved(Resource::Customer(support::user(1)))
                .unwrap(),
            atoms(20_000_000)
        );
    }
}

#[test]
fn flat_allocation_cannot_authorize_ordinary_payout_withdrawal_or_recovery() {
    for case in 0..3 {
        let temp = support::Temp::new();
        let (mut j, c, g) = workflow(&temp, true);
        assert_eq!(
            next(&bind(&c, &g, allocation()), &mut j, &c, &g, 2100).unwrap(),
            Step::Accepted
        );
        let mut intent = j.state().unwrap().funds()[0].intent.clone();
        intent.request = support::request(92);
        if case == 0 {
            intent.destination = Destination::Recipient(route().beneficiaries[0].tokens);
        } else {
            intent.source = Location::Venue;
            intent.destination = Destination::Location(Location::Broker);
        }
        let approval = orders::Approval {
            account: intent.request.account,
            authority_epoch: intent.authority_epoch,
            intent_hash: intent.digest().unwrap(),
        };
        let action = if case == 2 {
            lifecycle::Action::RecoveryAccept {
                intent: Box::new(intent),
                approval,
            }
        } else {
            lifecycle::Action::Accept {
                intent: Box::new(intent),
                approval,
            }
        };
        let mut t = tx(&j, 92, vec![], vec![Control::Funds(action)]);
        t.at = 2100;
        assert_eq!(
            j.commit(t).unwrap().receipt.controls,
            Some(ControlError::Unqualified)
        );
        assert_eq!(j.state().unwrap().funds().len(), 1);
        assert!(j.state().unwrap().attempts().is_empty());
        assert!(!j.state().unwrap().native_funding_ready());
    }
}
#[test]
fn fresh_boot_flat_authority_does_not_enable_ordinary_funding() {
    let temp = support::Temp::new();
    let mut j = Journal::create(
        SqliteBackend::create(&temp.db).unwrap(),
        support::FixtureProtection,
        config(),
    )
    .unwrap();
    let c = controller();
    let f = fake(10);
    let mut port = deposit(&f, &c);
    assert!(port.observe_deposit(&mut j, &c, 0).unwrap() == Outcome::Settled);
    let authority = tx(
        &j,
        2,
        vec![],
        vec![Control::Order(orders::Action::AdvanceAuthority {
            account: support::user(1),
            epoch: 1,
        })],
    );
    assert_eq!(j.commit(authority).unwrap().receipt.controls, None);
    c.bind(&mut j, id(3), 100).unwrap();
    let g = demo_tests::gateway();
    demo_tests::qualify(&mut j, &c, &g);
    assert!(
        c.initial_demo_ingress_ready(&mut j, &g, &demo_tests::policy(), 2100)
            .unwrap()
    );
    // Exactly the fresh boot prerequisites, no synthetic complete NativeCheck,
    // mark/collateral cut, trading readiness or imagined settled native credit.
    let Control::Funds(lifecycle::Action::FlatAccept {
        intent, approval, ..
    }) = Control::Funds(flat_template())
    else {
        panic!("flat authority")
    };
    let mut ordinary = tx(
        &j,
        45,
        vec![],
        vec![Control::Funds(lifecycle::Action::Accept {
            intent,
            approval,
        })],
    );
    ordinary.at = 2100;
    assert_eq!(
        j.commit(ordinary).unwrap().receipt.controls,
        Some(ControlError::Unqualified)
    );
    assert_eq!(
        j.transactions_with_receipts().last().unwrap().1.controls,
        Some(ControlError::Unqualified)
    );
    assert!(j.state().unwrap().funds().is_empty());
    assert!(j.state().unwrap().attempts().is_empty());
    assert!(!j.state().unwrap().native_funding_ready());
}
#[test]
fn fixed_allocation_accepts_once_without_credit_signing_or_attempt_creation() {
    let temp = support::Temp::new();
    let (mut j, c, g) = workflow(&temp, true);
    let a = bind(&c, &g, allocation());
    let book = j
        .state()
        .unwrap()
        .ledger()
        .book(Owner::Customer(support::user(1)))
        .unwrap()
        .clone();
    let vault = j.state().unwrap().ledger().vault();
    assert_eq!(next(&a, &mut j, &c, &g, 2100).unwrap(), Step::Accepted);
    let head = j.head();
    let s = j.state().unwrap();
    assert_eq!(s.funds().len(), 1);
    assert_eq!(s.funds()[0].intent.request, support::attempt(10).request);
    assert_eq!(s.funds()[0].intent.net, atoms(20_000_000));
    assert!(!s.funds()[0].intent.allow_partial);
    assert!(s.attempts().is_empty());
    assert_eq!(s.ledger().vault(), vault);
    assert_eq!(
        s.ledger().book(Owner::Customer(support::user(1))).unwrap(),
        &book
    );
    assert!(!s.native_funding_ready());
    assert_eq!(next(&a, &mut j, &c, &g, 2100).unwrap(), Step::Waiting);
    assert_eq!(j.head(), head);
    drop(j);
    let mut j = Journal::open(
        SqliteBackend::open(&temp.db, Migration::None).unwrap(),
        support::FixtureProtection,
        config(),
    )
    .unwrap();
    assert_eq!(
        next(&bind(&c, &g, allocation()), &mut j, &c, &g, 2100).unwrap(),
        Step::Waiting
    );
    assert_eq!(j.head(), head);
}
#[test]
fn two_fixed_legs_require_actual_release_completion_and_never_recredit_the_customer() {
    let temp = support::Temp::new();
    let (mut j, c, g) = workflow(&temp, true);
    let a = bind(&c, &g, allocation());
    let cash = j
        .state()
        .unwrap()
        .ledger()
        .book(Owner::Customer(support::user(1)))
        .unwrap()
        .cash();
    let f = fake(10);
    f.0.lock().unwrap().at = 2100;
    let mut port = loaded(&c, vec![original()])
        .with_transport(f.clone(), Arc::new(Advancing(AtomicU64::new(2100))))
        .unwrap();
    assert_eq!(next(&a, &mut j, &c, &g, 2100).unwrap(), Step::Accepted);
    for (n, rail) in [(10, Rail::Release), (11, Rail::Deposit)] {
        let attempt = support::attempt(n);
        assert_eq!(
            c.prepare_demo_ingress(
                &mut j,
                &g,
                &demo_tests::policy(),
                Dispatch {
                    attempt,
                    commit: id(70 + n),
                    at: 2100
                }
            )
            .unwrap(),
            rail
        );
        let head = j.head();
        assert_eq!(next(&a, &mut j, &c, &g, 2100).unwrap(), Step::Waiting);
        assert_eq!(j.head(), head);
        assert!(
            port.issue_demo_ingress(&mut j, &c, &g, &demo_tests::policy(), attempt)
                .unwrap()
                == Outcome::Submitted
        );
        f.0.lock().unwrap().contract = Some(
            c.original_chain_contract(&mut j, attempt, 2100)
                .unwrap()
                .as_bytes()
                .to_vec(),
        );
        let result = port.reconcile(&mut j, &c, attempt).unwrap();
        if n == 10 {
            assert!(result == Outcome::Settled);
            assert_eq!(next(&a, &mut j, &c, &g, 2100).unwrap(), Step::Accepted);
            let mut rpc = f.0.lock().unwrap();
            rpc.wire = None;
            rpc.contract = None;
            rpc.attempt = support::attempt(11);
        } else {
            assert!(result == Outcome::Pending);
        }
    }
    let head = j.head();
    assert_eq!(next(&a, &mut j, &c, &g, 2100).unwrap(), Step::Waiting);
    assert_eq!(j.head(), head);
    let s = j.state().unwrap();
    assert_eq!(s.funds().len(), 2);
    assert_eq!(s.ledger().in_transit().unwrap(), atoms(20_000_000));
    assert_eq!(s.ledger().venue().cash(), atoms(0));
    assert_eq!(
        s.ledger()
            .book(Owner::Customer(support::user(1)))
            .unwrap()
            .cash(),
        cash
    );
    assert!(!s.native_funding_ready());
    assert_eq!(
        f.0.lock()
            .unwrap()
            .requests
            .iter()
            .filter(|r| r["method"] == "sendTransaction")
            .count(),
        2
    );
    // Finish the original demo credit through its real retained-GET port. A
    // successful workflow, not only a pending one, must stay once-only.
    let signature = c
        .retained_wire(&mut j, support::attempt(11), 3100)
        .unwrap()
        .unwrap()
        .signature;
    let bodies = [
        json!({"success":true,"data":[{"amount":"20","transaction_id":chain::signature(signature),"created_at":2200}],"has_more":false}),
        json!({"success":true,"data":[{"amount":"20","balance":"20","pending_balance":"0","event_type":"deposit_release","created_at":2200}],"has_more":false}),
    ];
    for (i, body) in bodies.into_iter().enumerate() {
        let at = 3100 + i as u64 * 1000;
        let cinder_pacifica::funding::demo::Step::Request(request, completion) = c
            .prepare_demo_deposit_poll(
                &mut j,
                &g,
                &demo_tests::policy(),
                cinder_pacifica::funding::demo::Poll {
                    attempt: support::attempt(11),
                    reservation: id(100 + i as u8 * 2),
                    evidence: id(101 + i as u8 * 2),
                    at,
                },
            )
            .unwrap()
        else {
            panic!("original demo GET");
        };
        request.consume(at).unwrap();
        cinder_pacifica::reads::complete(
            &mut j,
            &g,
            *completion,
            Reply::Response {
                status: 200,
                received_at: at,
                retry_after_ms: None,
                body: PrivateBytes::new(serde_json::to_vec(&body).unwrap()).unwrap(),
            },
        )
        .unwrap();
    }
    assert!(
        c.confirm_demo_deposit(
            &mut j,
            &g,
            &demo_tests::policy(),
            support::attempt(11),
            id(104),
            4100
        )
        .unwrap()
    );
    assert_eq!(next(&a, &mut j, &c, &g, 4100).unwrap(), Step::Complete);
    let head = j.head();
    assert_eq!(
        j.state().unwrap().ledger().venue().cash(),
        atoms(20_000_000)
    );
    assert_eq!(j.state().unwrap().ledger().in_transit().unwrap(), atoms(0));
    assert_eq!(
        j.state()
            .unwrap()
            .ledger()
            .book(Owner::Customer(support::user(1)))
            .unwrap()
            .cash(),
        cash
    );
    assert!(!j.state().unwrap().native_funding_ready());
    drop(j);
    let mut j = Journal::open(
        SqliteBackend::open(&temp.db, Migration::None).unwrap(),
        support::FixtureProtection,
        config(),
    )
    .unwrap();
    // Expiry does not renew the completed grant or produce a third intent.
    assert_eq!(
        next(&bind(&c, &g, allocation()), &mut j, &c, &g, 10_000).unwrap(),
        Step::Complete
    );
    assert_eq!(j.head(), head);
    assert_eq!(j.state().unwrap().funds().len(), 2);
}
#[test]
fn seeded_cash_or_missing_original_cannot_authorize_the_release() {
    let temp = support::Temp::new();
    let (mut j, c, g) = workflow(&temp, false);
    let a = bind(&c, &g, allocation());
    let head = j.head();
    assert_eq!(next(&a, &mut j, &c, &g, 2100).unwrap(), Step::Waiting);
    assert_eq!(j.head(), head);
    assert!(j.state().unwrap().funds().is_empty());
}
#[test]
fn changed_grant_cannot_create_another_allocation_even_after_restart() {
    let temp = support::Temp::new();
    let (mut j, c, g) = workflow(&temp, true);
    assert_eq!(
        next(&bind(&c, &g, allocation()), &mut j, &c, &g, 2100).unwrap(),
        Step::Accepted
    );
    drop(j);
    let mut j = Journal::open(
        SqliteBackend::open(&temp.db, Migration::None).unwrap(),
        support::FixtureProtection,
        config(),
    )
    .unwrap();
    let head = j.head();
    for case in 0..8 {
        let mut p = allocation();
        match case {
            0 => p.revision += 1,
            1 => p.amount -= 1,
            2 => p.release_request = [12; 32],
            3 => p.deposit_request = [13; 32],
            4 => p.authority_epoch += 1,
            5 => p.expires_at += 1,
            6 => p.original.operation = [44; 32],
            _ => p.original.signature[0] ^= 1,
        }
        let chain = loaded(&c, vec![p.original.clone()]);
        let a = Authorization::bind(p, &config(), &c, &g, &demo_tests::policy(), &chain).unwrap();
        assert!(
            next(&a, &mut j, &c, &g, 2100).is_err(),
            "changed grant {case}"
        );
        assert_eq!(j.head(), head);
        assert_eq!(j.state().unwrap().funds().len(), 1);
    }
}
#[test]
fn invalid_scope_or_missing_explicit_setup_cannot_bind_authority() {
    let c = controller();
    let g = demo_tests::gateway();
    let chain = loaded(&c, vec![original()]);
    for case in 0..10 {
        let mut p = allocation();
        match case {
            0 => p.revision = 0,
            1 => p.amount = 0,
            2 => p.amount = route().maximum_movement + 1,
            3 => p.release_request = [0; 32],
            4 => p.deposit_request = p.release_request,
            5 => p.authority_epoch = 0,
            6 => p.expires_at = 0,
            7 => p.original.signature.pop().map(|_| ()).unwrap(),
            8 => p.original.account = [3; 32],
            _ => p.original.operation = [0; 32],
        }
        assert!(Authorization::bind(p, &config(), &c, &g, &demo_tests::policy(), &chain).is_err());
    }
    assert!(
        Authorization::bind(
            allocation(),
            &config(),
            &c,
            &g,
            &demo_tests::policy(),
            &loaded(&c, vec![])
        )
        .is_err()
    );
    let mut p = demo_tests::policy();
    p.initial_setup = None;
    assert!(Authorization::bind(allocation(), &config(), &c, &g, &p, &chain).is_err());
    let mut mainnet = profile();
    mainnet.environment = Origin::Mainnet.url().into();
    let policy = Policy {
        revision: 1,
        evidence: "synthetic mainnet refusal only".into(),
        execution: Level::Qualified,
        origin: Origin::Mainnet,
        expiry_ms: 3000,
        credits: 10000,
        cleanup_reserve: 100,
        read_cost: cinder_pacifica::reads::MIN_READ_COST,
    };
    let gateway = Gateway::new(mainnet, policy, Zeroizing::new([10; 32]), 1).unwrap();
    assert!(
        Authorization::bind(
            allocation(),
            &config(),
            &c,
            &gateway,
            &demo_tests::policy(),
            &chain
        )
        .is_err()
    );
    let p = allocation();
    assert_eq!(format!("{p:?}"), "DemoAllocationPolicy([PRIVATE])");
}
#[test]
fn expiry_rotation_freeze_and_stale_setup_refuse_without_a_movement() {
    for case in 0..4 {
        let temp = support::Temp::new();
        let (mut j, c, g) = workflow(&temp, true);
        let a = bind(&c, &g, allocation());
        if case == 1 || case == 2 {
            let mut t = tx(
                &j,
                90,
                vec![],
                vec![if case == 1 {
                    Control::Order(orders::Action::AdvanceAuthority {
                        account: support::user(1),
                        epoch: 2,
                    })
                } else {
                    Control::Funds(lifecycle::Action::Freeze)
                }],
            );
            t.at = 2100;
            assert!(j.commit(t).unwrap().receipt.controls.is_none());
        }
        let at = if case == 0 {
            10_000
        } else if case == 3 {
            9100
        } else {
            2100
        };
        let head = j.head();
        if case == 3 {
            assert_eq!(next(&a, &mut j, &c, &g, at).unwrap(), Step::Waiting);
        } else {
            assert!(next(&a, &mut j, &c, &g, at).is_err());
        }
        assert_eq!(j.head(), head);
        assert!(j.state().unwrap().funds().is_empty());
    }
}
#[test]
fn cancelled_or_capacity_rejected_release_cannot_authorize_the_second_leg() {
    for reject in [false, true] {
        let temp = support::Temp::new();
        let (mut j, c, g) = workflow(&temp, true);
        let a = bind(&c, &g, allocation());
        if reject {
            let vault = j.state().unwrap().ledger().vault();
            let mut t = tx(
                &j,
                90,
                vec![],
                vec![Control::Reserve {
                    request: support::request(90),
                    reservations: vec![Reservation {
                        resource: Resource::Location(Location::Vault),
                        amount: vault,
                    }],
                }],
            );
            t.at = 2100;
            assert!(j.commit(t).unwrap().receipt.controls.is_none());
            assert!(next(&a, &mut j, &c, &g, 2100).is_err());
            assert!(j.state().unwrap().funds().is_empty());
        } else {
            assert_eq!(next(&a, &mut j, &c, &g, 2100).unwrap(), Step::Accepted);
            let mut t = tx(
                &j,
                90,
                vec![],
                vec![Control::Funds(lifecycle::Action::CancelUnexposed(
                    support::attempt(10).request,
                ))],
            );
            t.at = 2100;
            assert!(j.commit(t).unwrap().receipt.controls.is_none());
        }
        let head = j.head();
        assert!(next(&a, &mut j, &c, &g, 2100).is_err());
        assert_eq!(j.head(), head);
        assert!(j.state().unwrap().attempts().is_empty());
    }
}
