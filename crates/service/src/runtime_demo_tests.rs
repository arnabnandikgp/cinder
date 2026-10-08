//! Real private Runtime/API/AEAD journal, synthetic prerequisites and GETs only.
//! No real chain signature, AWS call, native transfer or economic qualification.
use super::*;
use cinder_journal::{collateral, funds};
use cinder_kernel::{
    amounts::{PriceTicks, QuoteAtoms},
    ledger::{Change, Event, Owner, evidence::*, funds::Destination},
};
use cinder_pacifica::{
    execution::{Dispatch, Reply},
    funding::{ChainReceipt, Counters, Rail, Setup, VerifiedWire, demo},
    reads,
};
use serde_json::{Value, json};

fn policy() -> demo::Policy {
    demo::Policy {
        revision: 1,
        maximum_reads: 4,
        maximum_pages: 2,
        interval_ms: 1000,
        maximum_backoff_ms: 4000,
        lifetime_ms: 9000,
        initial_setup: None,
    }
}
fn atoms(n: i128) -> QuoteAtoms {
    QuoteAtoms::new(configuration().quote, n)
}
fn attempt() -> AttemptKey {
    AttemptKey {
        request: RequestKey {
            domain: configuration().domain,
            account: initialization_support::user(1),
            request: RequestId::new([203; 32]).unwrap(),
        },
        attempt: AttemptId::new([204; 32]).unwrap(),
    }
}
fn tx(store: &Store, n: u8, inputs: Vec<Input>, controls: Vec<Control>) -> Transaction {
    Transaction {
        id: CommitId::new([n; 32]).unwrap(),
        expected: store.head(),
        at: NOW,
        evidence: vec![],
        inputs,
        controls,
        order_observations: vec![],
        funds_observations: vec![],
    }
}
fn input(n: u8, location: Location, change: Change) -> Input {
    let source = configuration()
        .sources
        .into_iter()
        .find(|s| s.location == location)
        .unwrap()
        .scope;
    Input {
        source,
        source_cut: Some(1),
        authority_epoch: 1,
        observed_at: NOW,
        raw: PrivateBytes::new(b"OFFLINE synthetic prerequisite only".to_vec()).unwrap(),
        event: Some(Event {
            key: RecordKey::Economic(EventKey {
                scope: source,
                event: EconomicEventId::new(&[n]).unwrap(),
                leg: 0,
            }),
            policy: configuration().policy,
            change,
        }),
    }
}
fn application_base(p: demo::Policy) -> (Application, Arc<AdjustableTime>) {
    let metrics = Arc::new(Metrics::default());
    let mut app = independent(metrics);
    let time = Arc::new(AdjustableTime(AtomicU64::new(NOW)));
    app.clock = time.clone();
    app.gates.native_reads = true;
    {
        let mut active = app.active.lock().unwrap();
        let Active {
            store,
            funding,
            gateway,
            poll_ms,
            demo_deposit,
            ..
        } = &mut *active;
        *poll_ms = Some(1000);
        *demo_deposit = Some(p);
        gateway
            .initialize_reads(store, CommitId::new([200; 32]).unwrap(), NOW)
            .unwrap();
        funding
            .bind(store, CommitId::new([201; 32]).unwrap(), NOW)
            .unwrap();
    }
    (app, time)
}
fn application_with_original(finalized: bool) -> (Application, Arc<AdjustableTime>) {
    let (app, time) = application_base(policy());
    {
        let mut active = app.active.lock().unwrap();
        let Active { store, funding, .. } = &mut *active;
        assert!(
            funding
                .observe_setup(
                    store,
                    CommitId::new([202; 32]).unwrap(),
                    NOW,
                    Setup {
                        account: funding.route().broker,
                        observed_at: NOW,
                        lending_disabled: true,
                        borrowed: "0".into(),
                        interest: "0".into(),
                        complete: true,
                    }
                )
                .unwrap()
        );
        let seed = tx(
            store,
            203,
            vec![input(
                203,
                Location::Broker,
                Change::Receipt {
                    owner: Owner::Customer(initialization_support::user(1)),
                    location: Location::Broker,
                    amount: atoms(20),
                },
            )],
            vec![],
        );
        assert!(
            store
                .commit(seed)
                .unwrap()
                .receipt
                .inputs
                .iter()
                .all(|i| matches!(i, InputResult::Normalized(Disposition::Applied)))
        );
        let l = store.state().unwrap().ledger();
        let seed = tx(
            store,
            204,
            vec![input(
                204,
                Location::Venue,
                Change::Reconcile(NativeCheck {
                    expected_version: l.version(),
                    cash: Some(l.venue().cash()),
                    funding: Some(l.venue().funding()),
                    positions: Some(l.venue().positions().to_vec()),
                    complete: true,
                    resolves: vec![],
                }),
            )],
            vec![Control::Collateral(collateral::Cut {
                expected_version: l.version() + 1,
                policy: collateral::Policy {
                    revision: configuration().policy,
                    markets: vec![collateral::MarginRule {
                        market: configuration().markets[0].unit(),
                        private_bps: 1000,
                        native_bps: 1000,
                    }],
                    evidence: EvidencePolicy {
                        max_issue_age: 10000,
                        max_mark_age: 10000,
                        max_check_age: 10000,
                    },
                },
                marks: vec![MarkObservation {
                    price: PriceTicks::new(configuration().markets[0].unit(), 100).unwrap(),
                    evidence: EventKey {
                        scope: configuration().sources[0].scope,
                        event: EconomicEventId::new(&[204]).unwrap(),
                        leg: 0,
                    },
                    observed_at: NOW,
                    valid_until: NOW + 10000,
                    qualified: true,
                }],
            })],
        );
        assert!(store.commit(seed).unwrap().receipt.controls.is_none());
        let intent = funds::Intent {
            request: attempt().request,
            source: Location::Broker,
            destination: Destination::Location(Location::Venue),
            net: atoms(20),
            maximum_fee: atoms(0),
            fee_payer: Owner::House,
            allow_partial: false,
            policy: configuration().policy,
            authority_epoch: 1,
            expires_at: NOW + 10000,
        };
        let approval = orders::Approval {
            account: intent.request.account,
            intent_hash: intent.digest().unwrap(),
            authority_epoch: 1,
        };
        let admit = tx(
            store,
            205,
            vec![],
            vec![
                Control::Funds(funds::Action::Accept {
                    intent: Box::new(intent),
                    approval,
                }),
                Control::Funds(funds::Action::Prepare {
                    attempt: attempt(),
                    net: atoms(20),
                }),
            ],
        );
        assert!(store.commit(admit).unwrap().receipt.controls.is_none());
        let action = funding
            .expose_chain(
                store,
                Dispatch {
                    attempt: attempt(),
                    commit: CommitId::new([206; 32]).unwrap(),
                    at: NOW,
                },
                Rail::Deposit,
                Counters {
                    paid: 0,
                    sequence: 0,
                    recipient_tokens: [0; 32],
                    expires_at_slot: 500,
                },
            )
            .unwrap();
        let binding = openssl::sha::sha256(action.encode().unwrap().as_bytes());
        funding
            .persist_wire(
                store,
                CommitId::new([207; 32]).unwrap(),
                NOW,
                action,
                VerifiedWire {
                    attempt: attempt(),
                    binding,
                    signature: [61; 64],
                    wire: PrivateBytes::new(b"OFFLINE synthetic verified wire".to_vec()).unwrap(),
                },
            )
            .unwrap();
        if finalized {
            let r = funding.route();
            let receipt = ChainReceipt {
                network: configuration().domain.network.bytes(),
                attempt: attempt(),
                mint: r.mint,
                program: r.venue_program,
                source: r.broker_tokens,
                destination: r.venue_vault,
                amount: 20,
                succeeded: true,
                signature: [61; 64],
                slot: 200,
                operation: None,
                epoch: None,
                config: None,
                paid: None,
                sequence: None,
                raw: PrivateBytes::new(b"OFFLINE synthetic finalized receipt".to_vec()).unwrap(),
            };
            funding
                .observe_chain(store, CommitId::new([208; 32]).unwrap(), NOW, receipt)
                .unwrap();
        }
    }
    (app, time)
}
fn page(data: Value) -> Value {
    json!({"success":true,"data":data,"has_more":false})
}
fn deposit() -> Value {
    page(
        // Canonical base58 encoding of the synthetic retained [61; 64] signature.
        json!([{"amount":"20","transaction_id":"2E1muM4Xjqzyd5Fc7R5yCoF6cehhHoMqQbGAd2cs8V1uAVYQyZEWvPFUboZSs7YJwj8czm7o6qpGvmkhpmbssZA4","created_at":NOW}]),
    )
}
fn balance() -> Value {
    page(
        json!([{"amount":"20","balance":"20","pending_balance":"0","event_type":"deposit_release","created_at":NOW}]),
    )
}
struct Response {
    at: u64,
    status: u16,
    body: Value,
    calls: usize,
    target: String,
}
fn unqualify_setup(app: &Application) {
    let mut active = app.active.lock().unwrap();
    let Active { store, funding, .. } = &mut *active;
    assert!(
        !funding
            .observe_setup(
                store,
                CommitId::new([209; 32]).unwrap(),
                NOW,
                Setup {
                    account: funding.route().broker,
                    observed_at: NOW,
                    lending_disabled: false,
                    borrowed: "0".into(),
                    interest: "0".into(),
                    complete: false,
                }
            )
            .unwrap()
    );
}
fn setup_bodies() -> [Value; 3] {
    [
        json!({"success":true,"data":{"auto_lend_disabled":true,"margin_settings":[],"spot_settings":[]}}),
        json!({"success":true,"data":{"borrowed":"0","pending_interest":"0","spot_balances":[],"updated_at":NOW}}),
        json!({"success":true,"data":{
            "balance":"0","account_equity":"0","available_to_spend":"0","available_to_withdraw":"0",
            "pending_balance":"0","pending_interest":"0","total_margin_used":"0","cross_mmr":"0",
            "spot_collateral":"0","spot_market_value":"0","cross_account_equity":null,
            "positions_count":0,"orders_count":0,"stop_orders_count":0,"spot_balances":[],"updated_at":NOW
        }}),
    ]
}
#[test]
fn scheduler_archives_setup_reads_without_native_credit_or_complete_certificate() {
    let (app, time) = application_with_original(true);
    unqualify_setup(&app);
    for (i, body) in setup_bodies().into_iter().enumerate() {
        let at = NOW + 1000 * i as u64;
        time.0.store(at, Ordering::SeqCst);
        let mut reply = response(at, body);
        app.poll_native(&mut reply).unwrap();
        assert_eq!(reply.calls, 1);
        assert!(!reply.target.contains("/deposit/"));
        assert_pending(&app);
    }
    let head = app.active.lock().unwrap().store.head();
    time.0.store(NOW + 3000, Ordering::SeqCst);
    let mut reply = response(NOW + 3000, deposit());
    app.poll_native(&mut reply).unwrap();
    assert_eq!(reply.calls, 0);
    assert_eq!(app.active.lock().unwrap().store.head(), head);
}
#[test]
fn initial_demo_preflight_reaches_the_same_runtime_without_financial_activation() {
    let p = demo::Policy {
        initial_setup: Some(demo::InitialSetup {
            revision: 1,
            exclusive_control: true,
        }),
        ..policy()
    };
    let (app, time) = application_base(p.clone());
    for (i, body) in setup_bodies().into_iter().enumerate() {
        let at = NOW + 1000 * i as u64;
        time.0.store(at, Ordering::SeqCst);
        let mut reply = response(at, body);
        app.poll_native(&mut reply).unwrap();
        assert_eq!(reply.calls, 1);
    }
    time.0.store(NOW + 3000, Ordering::SeqCst);
    let mut unused = response(NOW + 3000, deposit());
    assert!(app.poll_setup(&mut unused).unwrap());
    assert_eq!(unused.calls, 0);
    let active = app.active.lock().unwrap();
    let s = active.store.state().unwrap();
    assert!(!s.native_funding_ready());
    assert!(s.attempts().is_empty() && s.funds().is_empty());
    assert_eq!(s.ledger().venue().cash(), atoms(0));
    assert!(!app.gates.funding && !app.gates.trading);
    let head = active.store.head();
    drop(active);
    app.poll_native(&mut unused).unwrap();
    assert_eq!(unused.calls, 0);
    assert_eq!(app.active.lock().unwrap().store.head(), head);
}
#[test]
fn funding_scheduler_preserves_existing_original_instead_of_preparing_a_replacement() {
    let (app, _) = application_with_original(true);
    let mut active = app.active.lock().unwrap();
    let Active { store, funding, .. } = &mut *active;
    let head = store.head();
    super::super::prepare_next_ingress(store, funding, None, NOW).unwrap();
    assert_eq!(store.head(), head);
    assert_eq!(store.state().unwrap().attempts().len(), 1);
    assert_eq!(store.state().unwrap().attempts()[0].key, attempt());
}
#[test]
fn setup_socket_releases_writer_and_rejoins_after_private_command() {
    struct Stalled {
        entered: mpsc::SyncSender<()>,
        resume: mpsc::Receiver<()>,
    }
    impl reads::Transport for Stalled {
        fn get(&mut self, request: reads::Request) -> Reply {
            assert!(
                request
                    .consume(NOW)
                    .unwrap()
                    .target()
                    .starts_with("/api/v1/account/settings?")
            );
            self.entered.send(()).unwrap();
            self.resume.recv_timeout(Duration::from_secs(3)).unwrap();
            Reply::Response {
                status: 200,
                body: PrivateBytes::new(setup_bodies()[0].to_string().into_bytes()).unwrap(),
                received_at: NOW,
                retry_after_ms: None,
            }
        }
    }
    let (app, _) = application_with_original(true);
    unqualify_setup(&app);
    let (entered, observed) = mpsc::sync_channel(1);
    let (release, resume) = mpsc::sync_channel(1);
    let mut transport = Stalled { entered, resume };
    std::thread::scope(|scope| {
        let work = scope.spawn(|| app.poll_native(&mut transport).unwrap());
        observed.recv_timeout(Duration::from_secs(3)).unwrap();
        assert!(app.active.try_lock().is_ok());
        success(&app.handle(&session(), grant(40), NOW).unwrap(), 1);
        let head = app.active.lock().unwrap().store.head();
        release.send(()).unwrap();
        work.join().unwrap();
        assert!(app.active.lock().unwrap().store.head().sequence > head.sequence);
    });
    assert_pending(&app);
}
impl reads::Transport for Response {
    fn get(&mut self, request: reads::Request) -> Reply {
        let prepared = request.consume(self.at).unwrap();
        assert_eq!(prepared.origin(), Origin::Testnet);
        self.target = prepared.target().to_owned();
        self.calls += 1;
        Reply::Response {
            status: self.status,
            body: PrivateBytes::new(self.body.to_string().into_bytes()).unwrap(),
            received_at: self.at,
            retry_after_ms: Some(3000),
        }
    }
}
fn response(at: u64, body: Value) -> Response {
    Response {
        at,
        status: 200,
        body,
        calls: 0,
        target: String::new(),
    }
}
fn assert_pending(app: &Application) {
    let active = app.active.lock().unwrap();
    let s = active.store.state().unwrap();
    assert_eq!(s.ledger().venue().cash(), atoms(0));
    assert_eq!(s.unresolved_raw(), 0);
    assert!(!s.native_funding_ready());
    assert!(
        !s.funds()
            .iter()
            .find(|o| o.attempt == Some(attempt()))
            .unwrap()
            .terminal
    );
}
#[test]
fn original_deposit_is_confirmed_once_without_generic_diagnostics_or_strong_readiness() {
    let (app, time) = application_with_original(true);
    let mut first = response(NOW, deposit());
    app.poll_native(&mut first).unwrap();
    assert_eq!(first.calls, 1);
    assert!(first.target.starts_with("/api/v1/account/deposit/history?"));
    assert_pending(&app);
    // Cadence excludes another GET at the same clock cut.
    app.poll_native(&mut first).unwrap();
    assert_eq!(first.calls, 1);
    time.0.store(NOW + 1000, Ordering::SeqCst);
    let mut second = response(NOW + 1000, balance());
    app.poll_native(&mut second).unwrap();
    assert_eq!(second.calls, 1);
    assert!(
        second.target.contains("/api/v1/account/balance/history?")
            && second.target.contains("include_trades=true")
    );
    let head = {
        let active = app.active.lock().unwrap();
        let s = active.store.state().unwrap();
        assert_eq!(s.ledger().venue().cash(), atoms(20));
        assert_eq!(s.ledger().in_transit().unwrap(), atoms(0));
        assert_eq!(s.unresolved_raw(), 0);
        assert!(!s.native_funding_ready());
        let o = s
            .funds()
            .iter()
            .find(|o| o.attempt == Some(attempt()))
            .unwrap();
        assert!(o.terminal && !o.faulted && o.demo.is_some() && o.proof.is_none());
        active.store.head()
    };
    time.0.store(NOW + 2000, Ordering::SeqCst);
    app.poll_native(&mut second).unwrap();
    assert_eq!(second.calls, 1);
    assert_eq!(app.active.lock().unwrap().store.head(), head);
}
#[test]
fn absent_finality_waits_and_missing_setup_only_spends_a_setup_read() {
    for finalized in [false, true] {
        let (app, _) = application_with_original(finalized);
        if finalized {
            let mut active = app.active.lock().unwrap();
            let Active { store, funding, .. } = &mut *active;
            assert!(
                !funding
                    .observe_setup(
                        store,
                        CommitId::new([209; 32]).unwrap(),
                        NOW,
                        Setup {
                            account: funding.route().broker,
                            observed_at: NOW,
                            lending_disabled: false,
                            borrowed: "0".into(),
                            interest: "0".into(),
                            complete: false,
                        }
                    )
                    .unwrap()
            );
        }
        let head = app.active.lock().unwrap().store.head();
        let mut reply = response(NOW, deposit());
        app.poll_native(&mut reply).unwrap();
        assert_eq!(reply.calls, usize::from(finalized));
        if finalized {
            assert!(reply.target.starts_with("/api/v1/account/settings?"));
            assert!(app.active.lock().unwrap().store.head().sequence > head.sequence);
        } else {
            assert_eq!(app.active.lock().unwrap().store.head(), head);
        }
        assert_pending(&app);
    }
    let (app, time) = application_with_original(true);
    let mut reply = response(NOW, json!({"error":"limited"}));
    reply.status = 429;
    app.poll_native(&mut reply).unwrap();
    time.0.store(NOW + 1000, Ordering::SeqCst);
    app.poll_native(&mut reply).unwrap();
    assert_eq!(reply.calls, 1);
    assert_pending(&app);
}
#[test]
fn delayed_demo_read_allows_private_commands_and_rejoins_the_new_head() {
    struct Stalled {
        entered: mpsc::SyncSender<()>,
        resume: mpsc::Receiver<()>,
    }
    impl reads::Transport for Stalled {
        fn get(&mut self, request: reads::Request) -> Reply {
            assert!(
                request
                    .consume(NOW)
                    .unwrap()
                    .target()
                    .starts_with("/api/v1/account/deposit/history?")
            );
            self.entered.send(()).unwrap();
            self.resume.recv_timeout(Duration::from_secs(3)).unwrap();
            Reply::Response {
                status: 200,
                body: PrivateBytes::new(deposit().to_string().into_bytes()).unwrap(),
                received_at: NOW,
                retry_after_ms: None,
            }
        }
    }
    let (app, _) = application_with_original(true);
    let (entered, observed) = mpsc::sync_channel(1);
    let (release, resume) = mpsc::sync_channel(1);
    let mut transport = Stalled { entered, resume };
    std::thread::scope(|scope| {
        let work = scope.spawn(|| {
            let _io = app.io.lock().unwrap();
            app.poll_native(&mut transport).unwrap();
        });
        observed.recv_timeout(Duration::from_secs(3)).unwrap();
        assert!(app.active.try_lock().is_ok());
        success(&app.handle(&session(), grant(40), NOW).unwrap(), 1);
        success(&app.handle(&session(), read(1, 11, 1), NOW).unwrap(), 3);
        let head = app.active.lock().unwrap().store.head();
        app.tick().unwrap();
        assert_eq!(app.active.lock().unwrap().store.head(), head);
        release.send(()).unwrap();
        work.join().unwrap();
        assert!(app.active.lock().unwrap().store.head().sequence > head.sequence);
    });
    assert_pending(&app);
}

#[test]
fn lost_replies_exhaust_durable_budget_without_an_economic_resend_or_policy_reset() {
    struct Lost {
        at: u64,
        calls: usize,
    }
    impl reads::Transport for Lost {
        fn get(&mut self, request: reads::Request) -> Reply {
            request.consume(self.at).unwrap();
            self.calls += 1;
            Reply::Unknown
        }
    }
    let (app, time) = application_with_original(true);
    app.active
        .lock()
        .unwrap()
        .demo_deposit
        .as_mut()
        .unwrap()
        .maximum_reads = 2;
    let mut lost = Lost { at: NOW, calls: 0 };
    app.poll_native(&mut lost).unwrap();
    for elapsed in [1000, 2000, 3000, 6000, 7000] {
        time.0.store(NOW + elapsed, Ordering::SeqCst);
        lost.at = NOW + elapsed;
        app.poll_native(&mut lost).unwrap();
    }
    assert_eq!(lost.calls, 2);
    assert_pending(&app);
    // Resetting volatile scheduling state is not a renewed durable GET budget.
    app.active.lock().unwrap().next_poll = NOW;
    app.poll_native(&mut lost).unwrap();
    assert_eq!(lost.calls, 2);
    // Nor may rebinding the policy for the same original attempt renew it.
    {
        let mut active = app.active.lock().unwrap();
        active.next_poll = NOW;
        active.demo_deposit.as_mut().unwrap().maximum_reads = 4;
    }
    assert!(app.poll_native(&mut lost).is_err());
    assert_eq!(lost.calls, 2);
    assert_pending(&app);
}

#[test]
fn scheduler_resumes_from_retained_reads_without_another_get() {
    let (app, time) = application_with_original(true);
    for (n, at, body) in [(220, NOW, deposit()), (222, NOW + 1000, balance())] {
        let mut active = app.active.lock().unwrap();
        let Active {
            store,
            funding,
            gateway,
            demo_deposit,
            ..
        } = &mut *active;
        let demo::Step::Request(request, completion) = funding
            .prepare_demo_deposit_poll(
                store,
                gateway,
                demo_deposit.as_ref().unwrap(),
                demo::Poll {
                    attempt: attempt(),
                    reservation: CommitId::new([n; 32]).unwrap(),
                    evidence: CommitId::new([n + 1; 32]).unwrap(),
                    at,
                },
            )
            .unwrap()
        else {
            panic!("expected reserved GET")
        };
        let reply = reads::Transport::get(&mut response(at, body), request);
        reads::complete(store, gateway, *completion, reply).unwrap();
    }
    assert_pending(&app);
    time.0.store(NOW + 1000, Ordering::SeqCst);
    let mut reply = response(NOW + 1000, balance());
    app.poll_native(&mut reply).unwrap();
    assert_eq!(reply.calls, 0);
    let active = app.active.lock().unwrap();
    assert_eq!(
        active.store.state().unwrap().ledger().venue().cash(),
        atoms(20)
    );
    assert!(
        active
            .store
            .state()
            .unwrap()
            .funds()
            .iter()
            .find(|o| o.attempt == Some(attempt()))
            .unwrap()
            .demo
            .is_some()
    );
}

#[test]
fn completion_refuses_future_or_expired_observations_and_bounded_response_overflow() {
    struct Invalid {
        time: Arc<AdjustableTime>,
        case: u8,
    }
    impl reads::Transport for Invalid {
        fn get(&mut self, request: reads::Request) -> Reply {
            request.consume(NOW).unwrap();
            if self.case == 1 {
                self.time.0.store(NOW + 120000, Ordering::SeqCst);
            }
            let bytes = if self.case == 2 {
                vec![b' '; demo::MAX_BODY + 1]
            } else {
                deposit().to_string().into_bytes()
            };
            Reply::Response {
                status: 200,
                body: PrivateBytes::new(bytes).unwrap(),
                received_at: if self.case == 0 { NOW + 1 } else { NOW },
                retry_after_ms: None,
            }
        }
    }
    for case in 0..3 {
        let (app, time) = application_with_original(true);
        let before = app.active.lock().unwrap().store.head();
        let mut reply = Invalid { time, case };
        assert!(app.poll_native(&mut reply).is_err());
        if case == 1 {
            assert!(app.health() == Health::Fenced);
            // Closing the read publication fences state access too. Only the
            // prior GET reservation was committed, not its late reply/credit.
            let active = app.active.lock().unwrap();
            assert!(active.store.state().is_err());
            assert_eq!(active.store.head().sequence, before.sequence + 1);
            assert!(
                active
                    .store
                    .transactions()
                    .last()
                    .unwrap()
                    .inputs
                    .is_empty()
            );
        } else {
            assert_pending(&app);
        }
    }
}

#[test]
fn concurrent_freeze_retains_the_balance_reply_without_posting_demo_credit() {
    struct Freeze<'a> {
        app: &'a Application,
    }
    impl reads::Transport for Freeze<'_> {
        fn get(&mut self, request: reads::Request) -> Reply {
            request.consume(NOW + 1000).unwrap();
            {
                let mut active = self.app.active.lock().unwrap();
                let mut freeze = tx(
                    &active.store,
                    225,
                    vec![],
                    vec![Control::Funds(funds::Action::Freeze)],
                );
                freeze.at = NOW + 1000;
                assert!(
                    active
                        .store
                        .commit(freeze)
                        .unwrap()
                        .receipt
                        .controls
                        .is_none()
                );
            }
            Reply::Response {
                status: 200,
                body: PrivateBytes::new(balance().to_string().into_bytes()).unwrap(),
                received_at: NOW + 1000,
                retry_after_ms: None,
            }
        }
    }
    let (app, time) = application_with_original(true);
    app.poll_native(&mut response(NOW, deposit())).unwrap();
    time.0.store(NOW + 1000, Ordering::SeqCst);
    assert!(app.poll_native(&mut Freeze { app: &app }).is_err());
    assert_pending(&app);
    let active = app.active.lock().unwrap();
    assert!(active.store.state().unwrap().frozen());
    assert!(
        active
            .store
            .transactions()
            .last()
            .unwrap()
            .evidence
            .iter()
            .any(|e| e
                .as_bytes()
                .starts_with(b"CINDER-DEMO-DEPOSIT-RESPONSE-1\0"))
    );
}
