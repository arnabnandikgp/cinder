//! Actual Runtime mutation owner, encrypted replicated journal and real signing;
//! synthetic external I/O only. Never a hardware or economic qualification.
use super::*;
use crate::{chain_funding::tests as chain_fixture, chain_rpc};
use cinder_kernel::ledger::Owner;
use cinder_pacifica::{
    execution::{Outbound, Reply, Transport},
    funding::Rail,
};
use serde_json::{Value, json};
use std::sync::atomic::AtomicU64;

struct EscrowExpiryTime(AtomicU64);

#[test]
fn actual_runtime_bootstraps_a_fresh_wallet_into_its_same_encrypted_journal_without_funding_activation()
 {
    use cinder_api::onboarding;
    use cinder_pacifica::funding::{chain, escrow::Routing};
    let metrics = Arc::new(Metrics::default());
    let mut app = application(metrics.clone());
    let mut config = configuration();
    config.customers.clear();
    let mut route = app.active.lock().unwrap().funding.route().clone();
    route.beneficiaries.clear();
    route.config = chain::pda(
        route.program,
        &[b"cinder_vault", &route.domain, &route.pool, &route.mint],
    )
    .unwrap();
    route.vault = chain::pda(route.program, &[b"tokens", &route.config]).unwrap();
    let routing = Routing {
        route: chain::pda(route.program, &[b"venue_route", &route.config]).unwrap(),
        venue: config.venue.bytes(),
    };
    let profile = Profile {
        config: config.clone(),
        source: config.sources[0].scope,
        account: chain::address(route.broker),
        environment: Origin::Testnet.url().into(),
        revision: 1,
        evidence: "offline zero-customer Runtime test".into(),
        precision: Level::Qualified,
        fills: Level::Qualified,
        quote_places: 0,
        perp_tag: 0,
        markets: vec![Mapping {
            symbol: "BTC".into(),
            market: 0,
            size: Grid { places: 0, step: 1 },
            price: Grid { places: 0, step: 1 },
        }],
    };
    let gateway = Gateway::new(
        profile.clone(),
        Policy {
            revision: 1,
            evidence: "offline Runtime test".into(),
            execution: Level::Qualified,
            origin: Origin::Testnet,
            expiry_ms: 3000,
            credits: 600,
            cleanup_reserve: 120,
            read_cost: cinder_pacifica::reads::MIN_READ_COST,
        },
        Zeroizing::new([7; 32]),
        1,
    )
    .unwrap();
    let funding = Controller::new_escrow(
        profile,
        route.clone(),
        routing.clone(),
        Zeroizing::new([9; 32]),
    )
    .unwrap();
    let policy = onboarding::Policy {
        maximum_accounts: 8,
        custody_domain: route.domain,
        pool: route.pool,
        venue: routing.venue,
        program: route.program,
        config: route.config,
        route: routing.route,
        mint: route.mint,
        decimals: route.decimals,
        epoch: route.epoch,
        maximum_deposit: route.maximum_movement,
    };
    // Selected-unit assembly must compare discovery economics with the loaded
    // controller, not merely accept canonical PDAs with arbitrary scales/limits.
    let unit = cinder_kernel::identity::UnitId::new([40; 32]).unwrap();
    let registry = cinder_journal::units::Configuration::new(
        config.domain,
        vec![cinder_journal::units::Descriptor {
            id: unit,
            environment: Origin::Testnet.commitment(),
            native_account: route.broker,
            funding_wallet: route.broker,
            custody_config: route.config,
            payout_vault: route.vault,
            key_policy: funding.release_commitment(&config).unwrap(),
            ledger: config.clone(),
        }],
    )
    .unwrap();
    for change in 0..4 {
        let mut p = policy.clone();
        match change {
            0 => p.decimals += 1,
            1 => p.epoch += 1,
            2 => p.maximum_deposit += 1,
            _ => {}
        }
        let contract =
            cinder_api::units::onboarding::Contract::new(registry.clone(), vec![(unit, p)])
                .unwrap();
        let api = cinder_api::units::service::Service::new(
            cinder_api::units::Authorizer::new(120_000, 120_000).unwrap(),
            RiskAdmission { enabled: false },
        )
        .with_onboarding(contract);
        let result =
            crate::units::custody::validate_onboarding(&registry, &api, &[(unit, &funding)]);
        assert_eq!(result.is_ok(), change == 3);
        assert!(crate::units::custody::validate_onboarding(&registry, &api, &[]).is_err());
    }
    let api = Service::new_onboarding(
        Contract {
            owners: vec![],
            maximum_auth_lifetime: 120_000,
            maximum_grant_lifetime: 120_000,
        },
        RiskAdmission { enabled: false },
        policy.clone(),
    )
    .unwrap();
    ApplicationContract::derive(&config, &api, &gateway, &funding).unwrap();
    let mut store = store_for(metrics, config.clone());
    api.initialize(&mut store, NOW).unwrap();
    funding
        .bind(&mut store, CommitId::new([88; 32]).unwrap(), NOW)
        .unwrap();
    {
        let mut active = app.active.lock().unwrap();
        active.store = store;
        active.gateway = gateway;
        active.funding = funding;
    }
    app.api = api;
    let mut r = onboarding::Request {
        domain: config.domain,
        policy: config.policy,
        wallet: public(33),
        session: [8; 32],
        expires_at: NOW + 60_000,
        command: onboarding::Command::Open,
        signature: [0; 64],
    };
    r.signature = Signer::new_without_digest(&key(33))
        .unwrap()
        .sign_oneshot_to_vec(&r.message())
        .unwrap()
        .try_into()
        .unwrap();
    let result = app.handle(&session(), r.encode().unwrap(), NOW).unwrap();
    let account = cinder_journal::onboarding::account(config.domain, public(33)).unwrap();
    let expected = cinder_api::Response::Discovery(Box::new(onboarding::Discovery {
        account,
        epoch: 1,
        recipient: policy.recipient(public(33)).unwrap(),
        custody: policy,
        pending_deposits: vec![],
    }))
    .encode()
    .unwrap();
    assert_eq!(result, expected);
    let active = app.active.lock().unwrap();
    let state = active.store.state().unwrap();
    assert_eq!(state.registrations().len(), 1);
    assert_eq!(
        state
            .ledger()
            .book(Owner::Customer(account))
            .unwrap()
            .cash()
            .atoms(),
        0
    );
    assert!(!state.native_funding_ready());
    assert!(!app.gates.funding);
    assert!(!app.stop.load(Ordering::SeqCst));
}
impl Clock for EscrowExpiryTime {
    fn now(&self) -> Result<u64, Error> {
        Ok(self.0.load(Ordering::SeqCst))
    }
}
struct EscrowExpiryRpc<'a> {
    app: &'a Application,
    inner: chain_fixture::Fake,
    clock: Arc<EscrowExpiryTime>,
}
impl chain_rpc::Transport for EscrowExpiryRpc<'_> {
    fn post(&mut self, body: PrivateBytes) -> Result<chain_rpc::Response, Error> {
        let v: Value = serde_json::from_slice(body.as_bytes()).unwrap();
        let advance = v["method"] == "getBlockHeight";
        let response = self.inner.post(body)?;
        if advance {
            let _writer = self
                .app
                .active
                .try_lock()
                .expect("slow escrow precheck cannot hold writer");
            self.clock.0.store(101, Ordering::SeqCst);
        }
        Ok(response)
    }
}

#[test]
fn offline_runtime_cancels_escrow_expiring_during_prechecks_without_exposure_or_fencing() {
    use chain_fixture::escrow_tests as escrow;
    let metrics = Arc::new(Metrics::default());
    let mut app = application(metrics.clone());
    let clock = Arc::new(EscrowExpiryTime(AtomicU64::new(100)));
    app.clock = clock.clone();
    app.deadline = 10_000;
    app.gates.funding = false;
    let (mut journal, funding) = escrow::seed(store_for(metrics, chain_fixture::config()));
    let original = escrow::start_with_expiry(&mut journal, &funding, 101);
    let fake = chain_fixture::fake(10);
    escrow::pending(&fake, &original, &funding);
    let loaded = chain_fixture::loaded(&funding, vec![]);
    {
        let mut a = app.active.lock().unwrap();
        a.store = journal;
        a.funding = funding;
    }
    let mut port = loaded
        .with_transport(
            EscrowExpiryRpc {
                app: &app,
                inner: fake.clone(),
                clock: clock.clone(),
            },
            clock,
        )
        .unwrap();
    app.tick_funds(Some(&mut port), &mut NoNativePost).unwrap();
    assert!(!app.stop.load(Ordering::SeqCst));
    assert_eq!(fake.calls("simulateTransaction"), 0);
    assert_eq!(fake.calls("sendTransaction"), 0);
    let head = {
        let mut a = app.active.lock().unwrap();
        let Active { store, funding, .. } = &mut *a;
        assert_eq!(
            store.state().unwrap().escrows()[0].status,
            cinder_journal::escrow::Status::Failed
        );
        assert!(
            funding
                .retained_wire(store, initialization_support::attempt(10), 101)
                .unwrap()
                .is_none()
        );
        assert!(
            !store
                .state()
                .unwrap()
                .attempts()
                .iter()
                .find(|a| a.key == initialization_support::attempt(10))
                .unwrap()
                .possibly_exposed
        );
        store.head()
    };
    app.tick_funds(Some(&mut port), &mut NoNativePost).unwrap();
    assert_eq!(app.active.lock().unwrap().store.head(), head);
}

struct FlatTime;
impl Clock for FlatTime {
    fn now(&self) -> Result<u64, Error> {
        Ok(2100)
    }
}
struct NoNativePost;
impl Transport for NoNativePost {
    fn post(&mut self, _: Outbound) -> Reply {
        panic!("cash-only allocation cannot POST orders or withdrawals")
    }
}
struct EscrowRpc<'a> {
    app: &'a Application,
    inner: chain_fixture::Fake,
    refund: bool,
    changed: bool,
}
impl chain_rpc::Transport for EscrowRpc<'_> {
    fn post(&mut self, body: PrivateBytes) -> Result<chain_rpc::Response, Error> {
        let value: Value = serde_json::from_slice(body.as_bytes()).unwrap();
        if value["method"] == "simulateTransaction" && !self.changed {
            let mut active = self
                .app
                .active
                .try_lock()
                .expect("escrow RPC cannot hold the financial writer");
            let store = &mut active.store;
            let controls = if self.refund {
                vec![Control::Escrow(cinder_journal::escrow::Action::Refund(
                    initialization_support::request(10),
                ))]
            } else {
                vec![]
            };
            let tx = Transaction {
                id: CommitId::new([70; 32]).unwrap(),
                expected: store.head(),
                at: 100,
                evidence: vec![],
                inputs: vec![],
                order_observations: vec![],
                funds_observations: vec![],
                controls,
            };
            assert_eq!(store.commit(tx).unwrap().receipt.controls, None);
            self.changed = true;
        }
        self.inner.post(body)
    }
}
#[test]
fn offline_runtime_escrow_path_releases_writer_for_io_and_rechecks_refund_before_signing() {
    use chain_fixture::escrow_tests as escrow;
    for refund in [false, true] {
        let metrics = Arc::new(Metrics::default());
        let mut app = application(metrics.clone());
        app.clock = Arc::new(Fixed);
        app.deadline = 10_000;
        // Deliberately direct offline qualification. Shipping manifest funding
        // activation for this new mode remains closed; no old demo grant is used.
        app.gates.funding = false;
        let (mut journal, funding) = escrow::seed(store_for(metrics, chain_fixture::config()));
        let original = escrow::start(&mut journal, &funding);
        let fake = chain_fixture::fake(10);
        escrow::pending(&fake, &original, &funding);
        let loaded = chain_fixture::loaded(&funding, vec![]);
        {
            let mut a = app.active.lock().unwrap();
            a.store = journal;
            a.funding = funding;
        }
        let mut port = loaded
            .with_transport(
                EscrowRpc {
                    app: &app,
                    inner: fake.clone(),
                    refund,
                    changed: false,
                },
                Arc::new(Fixed),
            )
            .unwrap();
        let result = app.tick_funds(Some(&mut port), &mut NoNativePost);
        let mut a = app.active.lock().unwrap();
        let Active { store, funding, .. } = &mut *a;
        let wire = funding
            .retained_wire(store, initialization_support::attempt(10), 100)
            .unwrap();
        if refund {
            assert!(result.is_err());
            assert!(wire.is_none());
            assert_eq!(fake.calls("sendTransaction"), 0);
            continue;
        }
        result.unwrap();
        assert!(wire.is_some());
        assert_eq!(fake.calls("sendTransaction"), 1);
        chain_fixture::retain_at(
            &fake,
            store,
            funding,
            initialization_support::attempt(10),
            100,
        );
        let tx = Transaction {
            id: CommitId::new([71; 32]).unwrap(),
            expected: store.head(),
            at: 100,
            evidence: vec![],
            inputs: vec![],
            order_observations: vec![],
            funds_observations: vec![],
            controls: vec![Control::Funds(cinder_journal::funds::Action::Freeze)],
        };
        assert_eq!(store.commit(tx).unwrap().receipt.controls, None);
        drop(a);
        app.tick_funds(Some(&mut port), &mut NoNativePost).unwrap();
        let a = app.active.lock().unwrap();
        let s = a.store.state().unwrap();
        assert!(s.frozen());
        assert_eq!(s.ledger().broker().atoms(), 20_000_000);
        assert_eq!(
            s.reserved(Resource::Customer(initialization_support::user(1)))
                .unwrap()
                .atoms(),
            20_000_000
        );
        assert!(!s.native_funding_ready());
        assert_eq!(fake.calls("sendTransaction"), 1);
    }
}
struct FlatRpc<'a> {
    app: &'a Application,
    inner: chain_fixture::Fake,
    revoke: bool,
    changed: bool,
}
impl chain_rpc::Transport for FlatRpc<'_> {
    fn post(&mut self, body: PrivateBytes) -> Result<chain_rpc::Response, Error> {
        let v: Value = serde_json::from_slice(body.as_bytes()).unwrap();
        if self.revoke && !self.changed && v["method"] == "simulateTransaction" {
            let mut active = self
                .app
                .active
                .try_lock()
                .expect("simulation cannot hold writer");
            let store = &mut active.store;
            let tx = Transaction {
                id: CommitId::new([75; 32]).unwrap(),
                expected: store.head(),
                at: 2100,
                evidence: vec![],
                inputs: vec![],
                order_observations: vec![],
                funds_observations: vec![],
                controls: vec![Control::Order(orders::Action::AdvanceAuthority {
                    account: initialization_support::user(1),
                    epoch: 2,
                })],
            };
            assert!(store.commit(tx).unwrap().receipt.controls.is_none());
            self.changed = true;
        }
        self.inner.post(body)
    }
}
#[test]
fn actual_encrypted_runtime_fresh_allocation_uses_two_originals_and_rechecks_before_signing() {
    use chain_fixture::{allocation_tests as allocation, demo_tests as initial};
    for revoke in [false, true] {
        let metrics = Arc::new(Metrics::default());
        let mut app = application(metrics.clone());
        app.clock = Arc::new(FlatTime);
        app.deadline = 30_000;
        app.gates.funding = true;
        app.gates.native_reads = true;
        let (journal, funding, gateway) =
            allocation::fresh(store_for(metrics, chain_fixture::config()), true);
        let original_amount =
            crate::customer_deposit::recorded_amount(&journal, &allocation::original())
                .unwrap()
                .unwrap()
                .atoms();
        let loaded = chain_fixture::loaded(&funding, vec![allocation::original()]);
        let authority = crate::demo_funding::Authorization::bind(
            allocation::allocation(),
            journal.configuration(),
            &funding,
            &gateway,
            &initial::policy(),
            &loaded,
        )
        .unwrap();
        {
            let mut active = app.active.lock().unwrap();
            active.store = journal;
            active.funding = funding;
            active.gateway = gateway;
            active.demo_deposit = Some(initial::policy());
            active.demo_allocation = Some(authority);
        }
        let fake = chain_fixture::fake(10);
        fake.time(2100);
        let mut port = loaded
            .with_transport(
                FlatRpc {
                    app: &app,
                    inner: fake.clone(),
                    revoke,
                    changed: false,
                },
                Arc::new(FlatTime),
            )
            .unwrap();
        let result = app.tick_funds(Some(&mut port), &mut NoNativePost);
        {
            let mut active = app.active.lock().unwrap();
            let Active { store, funding, .. } = &mut *active;
            let attempt = store.state().unwrap().funds()[0].attempt.unwrap();
            assert_eq!(attempt.request, initialization_support::request(10));
            let wire = funding.retained_wire(store, attempt, 2100).unwrap();
            if revoke {
                assert!(result.is_err());
                assert!(wire.is_none());
                assert_eq!(fake.calls("sendTransaction"), 0);
                continue;
            }
            result.unwrap();
            assert!(wire.is_some());
            assert_eq!(store.state().unwrap().funds().len(), 1);
            assert_eq!(
                store
                    .state()
                    .unwrap()
                    .reserved(Resource::Customer(initialization_support::user(1)))
                    .unwrap()
                    .atoms(),
                20_000_000
            );
            chain_fixture::retain_at(&fake, store, funding, attempt, 2100);
        }
        app.tick_funds(Some(&mut port), &mut NoNativePost).unwrap();
        assert!(app.active.lock().unwrap().store.state().unwrap().funds()[0].terminal);
        fake.next(11);
        app.tick_funds(Some(&mut port), &mut NoNativePost).unwrap();
        {
            let mut active = app.active.lock().unwrap();
            let Active { store, funding, .. } = &mut *active;
            assert_eq!(store.state().unwrap().funds().len(), 2);
            let attempt = store.state().unwrap().funds()[1].attempt.unwrap();
            assert_eq!(attempt.request, initialization_support::request(11));
            chain_fixture::retain_at(&fake, store, funding, attempt, 2100);
        }
        app.tick_funds(Some(&mut port), &mut NoNativePost).unwrap();
        app.tick_funds(Some(&mut port), &mut NoNativePost).unwrap();
        let active = app.active.lock().unwrap();
        let s = active.store.state().unwrap();
        assert_eq!(s.ledger().vault().atoms(), original_amount - 20_000_000);
        assert_eq!(s.ledger().broker().atoms(), 0);
        assert_eq!(s.ledger().in_transit().unwrap().atoms(), 20_000_000);
        assert_eq!(s.ledger().venue().cash().atoms(), 0);
        assert_eq!(
            s.ledger()
                .book(Owner::Customer(initialization_support::user(1)))
                .unwrap()
                .cash()
                .atoms(),
            original_amount
        );
        assert!(!s.native_funding_ready());
        assert!(s.orders().is_empty());
        assert!(s.funds().iter().all(|o| o.flat.is_some()));
        assert_eq!(fake.calls("sendTransaction"), 2);
    }
}

struct Fixed;
impl Clock for Fixed {
    fn now(&self) -> Result<u64, Error> {
        Ok(100)
    }
}
fn chain_app() -> (Application, chain_fixture::Fake) {
    let metrics = Arc::new(Metrics::default());
    let mut app = application(metrics.clone());
    app.clock = Arc::new(Fixed);
    app.deadline = 30_000;
    let (mut journal, funding) = chain_fixture::seed(store_for(metrics, chain_fixture::config()));
    chain_fixture::prepare(&mut journal, 10, Rail::Release);
    {
        let mut active = app.active.lock().unwrap();
        active.store = journal;
        active.funding = funding;
    }
    (app, chain_fixture::fake(10))
}
fn intervening_write(app: &Application, revoke: bool, id: u8) {
    let mut active = app
        .active
        .try_lock()
        .expect("financial network I/O must not hold mutation owner");
    let store = &mut active.store;
    let controls = if revoke {
        vec![Control::Order(orders::Action::AdvanceAuthority {
            account: initialization_support::user(1),
            epoch: 2,
        })]
    } else {
        vec![]
    };
    let tx = Transaction {
        id: CommitId::new([id; 32]).unwrap(),
        expected: store.head(),
        at: 100,
        evidence: vec![PrivateBytes::new(b"OFFLINE intervening writer".to_vec()).unwrap()],
        inputs: vec![],
        controls,
        order_observations: vec![],
        funds_observations: vec![],
    };
    assert!(store.commit(tx).unwrap().receipt.controls.is_none());
}
struct InterleavedRpc<'a> {
    app: &'a Application,
    inner: chain_fixture::Fake,
    stage: &'static str,
    revoke: bool,
    writes: usize,
    sends: usize,
}
impl chain_rpc::Transport for InterleavedRpc<'_> {
    fn post(&mut self, body: PrivateBytes) -> Result<chain_rpc::Response, Error> {
        let value: Value = serde_json::from_slice(body.as_bytes()).unwrap();
        let method = value["method"].as_str().unwrap();
        if method == self.stage && self.writes == 0 {
            intervening_write(self.app, self.revoke, 70);
            self.writes += 1;
        }
        if method == "sendTransaction" {
            self.sends += 1;
        }
        self.inner.post(body)
    }
}
#[test]
fn actual_runtime_chain_io_allows_writer_and_revocation_during_context_or_simulation() {
    for stage in [
        "getLatestBlockhash",
        "simulateTransaction",
        "sendTransaction",
    ] {
        for revoke in [false, true] {
            let (app, fake) = chain_app();
            let loaded = {
                let active = app.active.lock().unwrap();
                chain_fixture::loaded(&active.funding, vec![])
            };
            let transport = InterleavedRpc {
                app: &app,
                inner: fake,
                stage,
                revoke,
                writes: 0,
                sends: 0,
            };
            let mut port = loaded.with_transport(transport, Arc::new(Fixed)).unwrap();
            let result = app.tick_chain(&mut port, initialization_support::attempt(10));
            let mut active = app.active.lock().unwrap();
            let Active { store, funding, .. } = &mut *active;
            let wire = funding
                .retained_wire(store, initialization_support::attempt(10), 100)
                .unwrap();
            assert!(
                store
                    .transaction(CommitId::new([70; 32]).unwrap())
                    .is_some()
            );
            if revoke && stage != "sendTransaction" {
                assert!(result.is_err(), "revoke during {stage}");
                assert!(wire.is_none());
            } else {
                result.unwrap();
                assert!(wire.is_some());
            }
        }
    }
}
#[test]
fn actual_runtime_original_reconciliation_allows_intervening_writer_without_new_send() {
    let (app, fake) = chain_app();
    let loaded = {
        let active = app.active.lock().unwrap();
        chain_fixture::loaded(&active.funding, vec![])
    };
    let mut port = loaded
        .with_transport(fake.clone(), Arc::new(Fixed))
        .unwrap();
    app.tick_chain(&mut port, initialization_support::attempt(10))
        .unwrap();
    {
        let mut active = app.active.lock().unwrap();
        let Active { store, funding, .. } = &mut *active;
        chain_fixture::retain(&fake, store, funding, 10);
    }
    let loaded = {
        let active = app.active.lock().unwrap();
        chain_fixture::loaded(&active.funding, vec![])
    };
    let transport = InterleavedRpc {
        app: &app,
        inner: fake,
        stage: "getTransaction",
        revoke: false,
        writes: 0,
        sends: 0,
    };
    let mut port = loaded.with_transport(transport, Arc::new(Fixed)).unwrap();
    app.tick_chain(&mut port, initialization_support::attempt(10))
        .unwrap();
    let active = app.active.lock().unwrap();
    assert!(
        active
            .store
            .transaction(CommitId::new([70; 32]).unwrap())
            .is_some()
    );
    assert!(active.store.state().unwrap().funds()[0].terminal);
}
struct InterleavedPost<'a> {
    app: &'a Application,
    calls: usize,
}
impl Transport for InterleavedPost<'_> {
    fn post(&mut self, _: Outbound) -> Reply {
        self.calls += 1;
        // The actual encrypted writer can mutate the owner while POST awaits.
        let mut active = self.app.active.try_lock().expect("POST cannot hold writer");
        let Active { store, .. } = &mut *active;
        let tx = Transaction {
            id: CommitId::new([72; 32]).unwrap(),
            expected: store.head(),
            at: NOW,
            evidence: vec![],
            inputs: vec![],
            controls: vec![Control::Order(orders::Action::AdvanceAuthority {
                account: initialization_support::user(1),
                epoch: 2,
            })],
            order_observations: vec![],
            funds_observations: vec![],
        };
        assert!(store.commit(tx).unwrap().receipt.controls.is_none());
        Reply::Response {
            status: 200,
            body: PrivateBytes::new(
                json!({"success":true,"data":{"order_id":123}})
                    .to_string()
                    .into_bytes(),
            )
            .unwrap(),
            received_at: NOW,
            retry_after_ms: None,
        }
    }
}
#[test]
fn actual_runtime_native_post_does_not_hold_writer_and_late_ack_never_resends() {
    let app = application(Arc::new(Metrics::default()));
    let request = initialization_support::request(10);
    {
        let mut active = app.active.lock().unwrap();
        let Active { store, gateway, .. } = &mut *active;
        gateway
            .initialize_reads(store, CommitId::new([68; 32]).unwrap(), NOW)
            .unwrap();
        let source = configuration().sources[0].scope;
        let receipt = Input {
            source,
            source_cut: Some(1),
            authority_epoch: 1,
            observed_at: NOW,
            raw: PrivateBytes::new(b"OFFLINE synthetic cash".to_vec()).unwrap(),
            event: Some(cinder_kernel::ledger::Event {
                key: RecordKey::Economic(EventKey {
                    scope: source,
                    event: EconomicEventId::new(&[1]).unwrap(),
                    leg: 0,
                }),
                policy: configuration().policy,
                change: cinder_kernel::ledger::Change::Receipt {
                    owner: Owner::Customer(request.account),
                    location: Location::Venue,
                    amount: cinder_kernel::amounts::QuoteAtoms::new(configuration().quote, 100_000),
                },
            }),
        };
        let intent = orders::Intent {
            request,
            quantity: initialization_support::q(2),
            minimum: initialization_support::p(90),
            maximum: initialization_support::p(110),
            maximum_fee_per_lot: initialization_support::cash(1),
            reduce_only: false,
            time_in_force: orders::TimeInForce::GoodTilCancelled,
            policy: configuration().policy,
            authority_epoch: 1,
            expires_at: NOW + 10_000,
        };
        let approval = orders::Approval {
            account: request.account,
            authority_epoch: 1,
            intent_hash: intent.digest().unwrap(),
        };
        let tx = Transaction {
            id: CommitId::new([69; 32]).unwrap(),
            expected: store.head(),
            at: NOW,
            evidence: vec![],
            inputs: vec![receipt],
            controls: vec![
                // Synthetic qualification for this order race ONLY. Shipping
                // demo credit deliberately never supplies this capability.
                Control::Funds(cinder_journal::funds::Action::NativeCreditReady(true)),
                Control::Order(orders::Action::Accept {
                    intent: Box::new(intent),
                    approval,
                    reservations: vec![
                        Reservation {
                            resource: Resource::Customer(request.account),
                            amount: initialization_support::cash(10),
                        },
                        Reservation {
                            resource: Resource::Location(Location::Venue),
                            amount: initialization_support::cash(10),
                        },
                    ],
                }),
                Control::Order(orders::Action::Prepare {
                    attempt: initialization_support::attempt(10),
                }),
            ],
            order_observations: vec![],
            funds_observations: vec![],
        };
        assert_eq!(store.commit(tx).unwrap().receipt.controls, None);
    }
    let mut transport = InterleavedPost {
        app: &app,
        calls: 0,
    };
    app.tick_orders(&mut transport).unwrap();
    app.tick_orders(&mut transport).unwrap();
    assert_eq!(transport.calls, 1);
    let active = app.active.lock().unwrap();
    assert_eq!(
        active
            .store
            .state()
            .unwrap()
            .authority_epoch(request.account),
        Some(2)
    );
    assert_eq!(
        active
            .store
            .transactions()
            .last()
            .unwrap()
            .order_observations[0]
            .status,
        cinder_journal::orders::Status::Acknowledged
    );
}
