//! Actual measured constructors, RecordCipher, Handler and Ed25519 user/native
//! keys, with SYNTHETIC account/fill responses. Not native or Nitro evidence.
use super::*;
use cinder_bulk::{
    observation::{Kind, Message},
    paper, reads,
};
use cinder_kernel::ledger::Owner as BookOwner;
use serde_json::{Value, json};
fn input() -> (Manifest, BTreeMap<Role, Zeroizing<Vec<u8>>>) {
    let (mut m, mut keys) = worker_input(true);
    m.version = 9;
    let worker = m.units.as_mut().unwrap().worker.as_mut().unwrap();
    worker.pacifica_dispatches = 0;
    worker.bulk_dispatches = 5;
    worker.bulk_reads = 2;
    let mut p: Pacifica = serde_cbor::from_slice(&keys[&Role::PacificaConfiguration]).unwrap();
    p.execution.execution = pp::Level::Unknown;
    p.fills = Some(pp::Level::Unknown);
    keys.insert(
        Role::PacificaConfiguration,
        Zeroizing::new(serde_cbor::to_vec(&p).unwrap()),
    );
    let b: Bulk = serde_cbor::from_slice(&keys[&Role::BulkConfiguration]).unwrap();
    let c = cinder_journal::wire::decode_config(&b.unit.ledger).unwrap();
    let profile = bf::Profile::new(
        cinder_bulk::Scope::new(
            c.sources[b.source].scope,
            cinder_bulk::Network::Testnet,
            b.unit.native_account,
        )
        .unwrap(),
        c.quote,
        b.quote_places,
        bf::Target::Master,
        b.markets
            .iter()
            .map(|mapping| {
                bf::Mapping::new(
                    cinder_bulk::precision::Market::new(
                        &mapping.symbol,
                        &mapping.size_step,
                        &mapping.price_step,
                        b.quote_places,
                    )
                    .unwrap(),
                    c.markets[mapping.market].unit(),
                )
            })
            .collect(),
    )
    .unwrap();
    let mut global: Configuration = serde_cbor::from_slice(&keys[&Role::Configuration]).unwrap();
    global.paper = Some(paper::Policy {
        purpose: "closed-fresh-paper-1".into(),
        run: [90; 32],
        unit: m.units.as_ref().unwrap().bulk,
        account: b.unit.native_account,
        customer: public(9),
        profile: profile.commitment(),
        asset: c.quote.asset.bytes(),
        precision: c.quote.precision.get(),
        setup: [91; 32],
        quote: [92; 32],
        observed_at: 100,
        expires_at: 30000,
        customer_atoms: 20,
        notional_atoms: 20,
        buffer_atoms: 20,
        reference_ticks: 4,
        lower_ticks: 3,
        upper_ticks: 5,
        cleanup_ticks: 5,
    });
    keys.insert(
        Role::Configuration,
        Zeroizing::new(serde_cbor::to_vec(&global).unwrap()),
    );
    crate::application::prepare(&mut m, &keys).unwrap();
    (m, keys)
}
#[test]
fn paper_requires_its_own_measured_version_exact_scope_and_read_only_pacifica() {
    let (m, keys) = input();
    let loaded = loaded(&m, keys.clone());
    assert_eq!(loaded.commitment(), m.application);
    for (role, body) in &keys {
        assert!(m.wrap(*role, body).unwrap().starts_with(b"CKR9"));
    }
    for variant in 0..8 {
        let mut m = m.clone();
        let mut keys = keys.clone();
        let mut c: Configuration = serde_cbor::from_slice(&keys[&Role::Configuration]).unwrap();
        match variant {
            0 => m.version = 8,
            1 => c.paper = None,
            2 => c.paper.as_mut().unwrap().asset = [99; 32],
            3 => c.paper.as_mut().unwrap().account = [99; 32],
            4 => c.paper.as_mut().unwrap().purpose = "production".into(),
            5 => {
                m.units
                    .as_mut()
                    .unwrap()
                    .worker
                    .as_mut()
                    .unwrap()
                    .pacifica_dispatches = 1
            }
            6 => {
                m.units
                    .as_mut()
                    .unwrap()
                    .worker
                    .as_mut()
                    .unwrap()
                    .bulk_dispatches = 6
            }
            _ => {
                let mut b: Bulk = serde_cbor::from_slice(&keys[&Role::BulkConfiguration]).unwrap();
                b.credits = b.cleanup_reserve + 4;
                keys.insert(
                    Role::BulkConfiguration,
                    Zeroizing::new(serde_cbor::to_vec(&b).unwrap()),
                );
            }
        }
        keys.insert(
            Role::Configuration,
            Zeroizing::new(serde_cbor::to_vec(&c).unwrap()),
        );
        assert!(
            crate::application::prepare(&mut m, &keys).is_err(),
            "variant {variant}"
        );
    }
    // Existing profiles stay omitted/disabled, not implicitly upgraded by decode.
    let (m, keys) = worker_input(false);
    let value: serde_cbor::Value = serde_cbor::from_slice(&keys[&Role::Configuration]).unwrap();
    let serde_cbor::Value::Map(fields) = value else {
        panic!("map")
    };
    assert!(!fields.contains_key(&serde_cbor::Value::Text("paper".into())));
    assert!(loaded_paper_is_absent(&m, keys));
}
fn loaded_paper_is_absent(m: &Manifest, keys: BTreeMap<Role, Zeroizing<Vec<u8>>>) -> bool {
    loaded(m, keys).paper.is_none()
}
#[test]
fn paper_expiry_caps_the_real_startup_lease_before_native_io() {
    let (mut m, keys) = input();
    m.gates.maximum_boot_ms = 60_000;
    crate::application::prepare(&mut m, &keys).unwrap();
    let shared = shared();
    let time = Arc::new(Time(AtomicU64::new(100)));
    let stop = Arc::new(AtomicBool::new(false));
    let backend = Interrupted {
        memory: Memory(shared.clone()),
        time: time.clone(),
        stop: stop.clone(),
        deadline: 30_000,
        case: 0,
    };
    assert!(
        loaded(&m, keys)
            .attach(
                &m,
                backend,
                true,
                Arc::new(Fresh(shared.clone())),
                time.clone(),
                stop.clone()
            )
            .is_err()
    );
    assert_eq!(
        shared.frames.lock().unwrap().len(),
        1,
        "no enrollment or native reservation past paper expiry"
    );
    assert!(stop.load(Ordering::SeqCst));
}
struct Port {
    agent: [u8; 32],
    calls: usize,
}
impl reads::Transport for Port {
    fn query(&mut self, r: reads::Request) -> bulk::Reply {
        let q = r.consume(100).unwrap();
        let request: Value = serde_json::from_slice(q.body()).unwrap();
        let body = if request["type"] == "fullAccount" {
            json!([{"fullAccount":{"kind":"MasterEOA","authorizedAgentWallets":[cinder_pacifica::funding::chain::address(self.agent)],
                "margin":{"totalMargin":"1000","availableMargin":"1000","marginUsed":"0","notional":"0","realizedPnl":"0","unrealizedPnl":"0","fees":"0","funding":"0"},
                "positions":[],"openOrders":[]}}])
        } else {
            // Actual fresh native history shape: no synthetic positive frontier.
            json!({"data":[],"page":{"hasMore":false,"nextCursor":null,"coverage":"unknown","asOfSlot":0,"startSlot":0,"endSlot":0}})
        };
        self.calls += 1;
        bulk::Reply::Response {
            status: 200,
            body: PrivateBytes::new(serde_json::to_vec(&body).unwrap()).unwrap(),
            received_at: 100,
            retry_after_ms: None,
        }
    }
}
fn start() -> (App<Memory>, bulk::Gateway) {
    let (m, keys) = input();
    let l = loaded(&m, keys);
    let shared = shared();
    let time = Arc::new(Time(AtomicU64::new(100)));
    let mut j = units::Journal::create(
        Memory(shared.clone()),
        RecordCipher::new(l.storage, m.generation, m.stream).unwrap(),
        l.config,
    )
    .unwrap();
    initialize_owners(&mut j, &l.owners, m.units.as_ref().unwrap(), time.as_ref()).unwrap();
    let unit = UnitId::new(m.units.as_ref().unwrap().bulk).unwrap();
    let mut port = Port {
        agent: l.native.bulk.agent_key(),
        calls: 0,
    };
    l.native
        .bulk
        .initialize(
            &mut j.select(unit).unwrap(),
            CommitId::new([100; 32]).unwrap(),
            100,
        )
        .unwrap();
    paper::initialize(
        l.paper.as_ref().unwrap(),
        &l.native.bulk,
        &mut j.select(unit).unwrap(),
        &mut port,
        || Ok(100),
    )
    .unwrap();
    assert_eq!(port.calls, 4);
    let binding = Binding {
        stream: Stream {
            domain: j.configuration().domain(),
            id: m.stream,
        },
        epoch: m.epoch,
        api: l.api.commitment(j.configuration()).unwrap(),
    };
    (
        super::super::super::Runtime::open(j, l.api, binding, Arc::new(Fresh(shared)), time, 30000)
            .unwrap(),
        l.native.bulk,
    )
}
fn order(lots: i64, maximum: u64, reduce_only: bool) -> Command {
    Command::Order {
        market: test_support::config().markets[0].unit().market,
        lots,
        minimum: 3,
        maximum,
        fee: 0,
        tif: 2,
        reduce_only,
        good_until: 10000,
    }
}
#[test]
fn ordinary_release_or_changed_experiment_cannot_reuse_paper_backing() {
    let (app, _) = start();
    let mut j = app.journal.lock().unwrap();
    let (_, keys) = input();
    let c: Configuration = serde_cbor::from_slice(&keys[&Role::Configuration]).unwrap();
    validate_paper_history(&mut j, c.paper.as_ref()).unwrap();
    assert!(validate_paper_history(&mut j, None).is_err());
    let mut changed = c.paper.unwrap();
    changed.run = [99; 32];
    assert!(validate_paper_history(&mut j, Some(&changed)).is_err());
    assert_eq!(
        j.state()
            .unwrap()
            .unit(UnitId::new([40; 32]).unwrap())
            .unwrap()
            .ledger()
            .venue()
            .cash()
            .atoms(),
        1000
    );
}
struct NoPacifica;
impl cinder_pacifica::reads::Transport for NoPacifica {
    fn get(&mut self, _: cinder_pacifica::reads::Request) -> pacifica::Reply {
        panic!("Pacifica remains read-only and unpolled in this test")
    }
}
impl pacifica::Transport for NoPacifica {
    fn post(&mut self, _: pacifica::Outbound) -> pacifica::Reply {
        panic!("no Pacifica dispatch")
    }
}
struct NativePort(Arc<Mutex<Option<Value>>>);
impl reads::Transport for NativePort {
    fn query(&mut self, _: reads::Request) -> bulk::Reply {
        panic!("no extra read")
    }
}
impl bulk::Transport for NativePort {
    fn post(&mut self, r: bulk::Outbound) -> bulk::Reply {
        let mut wire = self.0.lock().unwrap();
        assert!(wire.is_none());
        *wire = Some(serde_json::from_slice(r.body()).unwrap());
        bulk::Reply::Unknown
    }
}
#[test]
fn accepted_but_unexposed_opening_cannot_authorize_an_oversized_native_cleanup() {
    let (app, g) = start();
    let session = session();
    let unit = UnitId::new([40; 32]).unwrap();
    for id in [140, 141] {
        app.handle(
            &session,
            request(40, id, 1, &session, order(1, 3, false), 100),
            100,
        )
        .unwrap();
    }
    let (opening, c) = {
        let j = app.journal.lock().unwrap();
        let state = j.state().unwrap().unit(unit).unwrap();
        assert_eq!(state.orders().len(), 2);
        (
            state.orders()[0].intent.request,
            j.configuration()
                .units()
                .iter()
                .find(|d| d.id == unit)
                .unwrap()
                .clone(),
        )
    };
    let attempt = app
        .prepare_order(
            unit,
            opening,
            AttemptId::new([142; 32]).unwrap(),
            CommitId::new([143; 32]).unwrap(),
        )
        .unwrap();
    let nonce = 1_000_000;
    let (_, completion) = app
        .prepare_bulk(unit, &g, attempt, CommitId::new([144; 32]).unwrap(), nonce)
        .unwrap();
    app.complete_bulk(unit, &g, completion, bulk::Reply::Unknown)
        .unwrap();
    let native = cinder_bulk::precision::Market::new("BTC-USD", "1", "1", 0).unwrap();
    let scope = cinder_bulk::Scope::new(
        c.ledger.sources[0].scope,
        cinder_bulk::Network::Testnet,
        c.native_account,
    )
    .unwrap();
    let action = cinder_bulk::wire::Action::limit(
        &native,
        true,
        "3",
        "1",
        cinder_bulk::wire::TimeInForce::Ioc,
    )
    .unwrap();
    let oid = action.order_id(&scope, nonce).unwrap();
    let body = json!({"data":[{"maker":cinder_pacifica::funding::chain::address(c.native_account),"taker":cinder_pacifica::funding::chain::address([1;32]),
        "orderIdMaker":cinder_pacifica::funding::chain::address(oid),"orderIdTaker":cinder_pacifica::funding::chain::address([2;32]),
        "isBuy":true,"symbol":"BTC-USD","amount":"1","price":"3","makerFee":"0","takerFee":"0","fee":"0","reasonCode":0,"iso":false,
        "slot":100,"timestamp":100,"sequence":1}],"page":{"hasMore":false,"nextCursor":null,"coverage":"unknown","asOfSlot":100,"startSlot":0,"endSlot":100}});
    app.observe_bulk(
        unit,
        &g,
        Message {
            kind: Kind::Trades,
            body: PrivateBytes::new(serde_json::to_vec(&body).unwrap()).unwrap(),
            received_at: 100,
        },
        CommitId::new([145; 32]).unwrap(),
    )
    .unwrap();
    // Both accepted opening quantities fit the paper's finite cumulative budget,
    // but only one ever escaped or filled. Budget admission is not authority to
    // sign a close for the unexposed quantity.
    app.handle(
        &session,
        request(40, 146, 1, &session, order(-2, 3, true), 100),
        100,
    )
    .unwrap();
    assert_eq!(
        app.journal
            .lock()
            .unwrap()
            .state()
            .unwrap()
            .unit(unit)
            .unwrap()
            .orders()
            .len(),
        3,
        "the oversized cleanup reaches the existing exposure gate"
    );
    let (m, keys) = input();
    let l = loaded(&m, keys);
    let native_wire = Arc::new(Mutex::new(None));
    let mut worker = crate::units::worker::Worker::new(
        crate::units::worker::Policy {
            poll_ms: 1000,
            pacifica_reads: 0,
            bulk_reads: 0,
            pacifica_dispatches: 0,
            bulk_dispatches: 2,
        },
        UnitId::new([50; 32]).unwrap(),
        unit,
        l.native.pacifica,
        l.native.bulk,
        NoPacifica,
        NativePort(native_wire.clone()),
    )
    .unwrap()
    .reserve_paper_cleanup();
    // The final-slot worker cannot expose the other opening, and the existing
    // private reduce-only gate rejects a two-lot close of one actual lot.
    assert_eq!(worker.tick(&app).unwrap(), crate::units::worker::Step::Idle);
    assert!(native_wire.lock().unwrap().is_none());
    let j = app.journal.lock().unwrap();
    let state = j.state().unwrap().unit(unit).unwrap();
    assert_eq!(state.ledger().venue().positions()[0].quantity().lots(), 1);
    assert_eq!(
        state
            .attempts()
            .iter()
            .filter(|a| a.possibly_exposed)
            .count(),
        1
    );
    assert!(state.orders()[1].attempt.is_none());
}
#[test]
fn signed_handler_native_exposure_fill_attribution_and_duplicate_share_the_paper_ledger() {
    let (app, g) = start();
    let session = session();
    let unit = UnitId::new([40; 32]).unwrap();
    let wire = request(40, 120, 1, &session, order(1, 3, false), 100);
    app.handle(&session, wire.clone(), 100).unwrap();
    app.handle(&session, wire, 100).unwrap();
    // A signature altered after signing and an order for the other unit cannot
    // gain financial acceptance. Neither changes either account's native cash.
    let mut bad = request(40, 121, 1, &session, order(1, 3, false), 100)
        .as_bytes()
        .to_vec();
    *bad.last_mut().unwrap() ^= 1;
    let _ = app.handle(&session, PrivateBytes::new(bad).unwrap(), 100);
    let _ = app.handle(
        &session,
        request(50, 122, 1, &session, order(1, 3, false), 100),
        100,
    );
    let request_key = {
        let j = app.journal.lock().unwrap();
        let s = j.state().unwrap();
        assert_eq!(s.unit(unit).unwrap().orders().len(), 1);
        assert_eq!(
            s.unit(UnitId::new([50; 32]).unwrap())
                .unwrap()
                .ledger()
                .venue()
                .cash()
                .atoms(),
            0
        );
        s.unit(unit).unwrap().orders()[0].intent.request
    };
    let attempt = app
        .prepare_order(
            unit,
            request_key,
            AttemptId::new([123; 32]).unwrap(),
            CommitId::new([124; 32]).unwrap(),
        )
        .unwrap();
    let nonce = 1000000;
    let (out, completion) = app
        .prepare_bulk(unit, &g, attempt, CommitId::new([125; 32]).unwrap(), nonce)
        .unwrap();
    assert_eq!(out.network(), cinder_bulk::Network::Testnet);
    assert!(!out.body().is_empty());
    app.complete_bulk(unit, &g, completion, bulk::Reply::Unknown)
        .unwrap();
    let c = {
        app.journal
            .lock()
            .unwrap()
            .configuration()
            .units()
            .iter()
            .find(|d| d.id == unit)
            .unwrap()
            .clone()
    };
    let native = cinder_bulk::precision::Market::new("BTC-USD", "1", "1", 0).unwrap();
    let scope = cinder_bulk::Scope::new(
        c.ledger.sources[0].scope,
        cinder_bulk::Network::Testnet,
        c.native_account,
    )
    .unwrap();
    let action = cinder_bulk::wire::Action::limit(
        &native,
        true,
        "3",
        "1",
        cinder_bulk::wire::TimeInForce::Ioc,
    )
    .unwrap();
    let oid = action.order_id(&scope, nonce).unwrap();
    let body = json!({"data":[{"maker":cinder_pacifica::funding::chain::address(c.native_account),"taker":cinder_pacifica::funding::chain::address([1;32]),
        "orderIdMaker":cinder_pacifica::funding::chain::address(oid),"orderIdTaker":cinder_pacifica::funding::chain::address([2;32]),
        "isBuy":true,"symbol":"BTC-USD","amount":"1","price":"3","makerFee":"0","takerFee":"0","fee":"0","reasonCode":0,"iso":false,
        "slot":100,"timestamp":100,"sequence":1}],"page":{"hasMore":false,"nextCursor":null,"coverage":"unknown","asOfSlot":100,"startSlot":0,"endSlot":100}});
    for id in [126, 127] {
        app.observe_bulk(
            unit,
            &g,
            Message {
                kind: Kind::Trades,
                body: PrivateBytes::new(serde_json::to_vec(&body).unwrap()).unwrap(),
                received_at: 100,
            },
            CommitId::new([id; 32]).unwrap(),
        )
        .unwrap();
    }
    let j = app.journal.lock().unwrap();
    let s = j.state().unwrap().unit(unit).unwrap();
    let customer = request_key.account;
    assert_eq!(
        s.ledger()
            .book(BookOwner::Customer(customer))
            .unwrap()
            .positions()[0]
            .quantity()
            .lots(),
        1
    );
    assert_eq!(s.ledger().venue().positions()[0].quantity().lots(), 1);
    assert_eq!(s.ledger().venue().cash().atoms(), 1000);
    assert_eq!(
        s.ledger().book(BookOwner::House).unwrap().cash().atoms(),
        980
    );
    assert_eq!(
        s.ledger()
            .book(BookOwner::Customer(customer))
            .unwrap()
            .cash()
            .atoms(),
        20
    );
    assert_eq!(s.ledger().vault().atoms(), 0);
    assert_eq!(s.ledger().broker().atoms(), 0);
    assert_eq!(s.orders()[0].executions.len(), 1);
    assert!(s.orders()[0].unknown);
    assert!(s.holds().iter().any(|h| h.active));
    assert_eq!(s.ledger().unresolved_attribution(), 0);
    assert!(s.native_funding_ready());
    drop(j);
    // A second opening would spend the cleanup reserve and is refused.
    let _ = app.handle(
        &session,
        request(40, 128, 1, &session, order(3, 3, false), 100),
        100,
    );
    assert_eq!(
        app.journal
            .lock()
            .unwrap()
            .state()
            .unwrap()
            .unit(unit)
            .unwrap()
            .orders()
            .len(),
        1
    );
    app.handle(
        &session,
        request(40, 129, 1, &session, order(-1, 5, true), 100),
        100,
    )
    .unwrap();
    // Even a pending cancel must not spend the last slot reserved for cleanup.
    app.handle(
        &session,
        request(
            40,
            130,
            1,
            &session,
            Command::Cancel {
                target: RequestId::new([120; 32]).unwrap(),
                attempt: AttemptId::new([131; 32]).unwrap(),
                good_until: 10000,
            },
            100,
        ),
        100,
    )
    .unwrap();
    let (m, keys) = input();
    let l = loaded(&m, keys);
    let native_wire = Arc::new(Mutex::new(None));
    let mut worker = crate::units::worker::Worker::new(
        crate::units::worker::Policy {
            poll_ms: 1000,
            pacifica_reads: 0,
            bulk_reads: 0,
            pacifica_dispatches: 0,
            bulk_dispatches: 2,
        },
        UnitId::new([50; 32]).unwrap(),
        unit,
        l.native.pacifica,
        l.native.bulk,
        NoPacifica,
        NativePort(native_wire.clone()),
    )
    .unwrap()
    .reserve_paper_cleanup();
    assert_eq!(
        worker.tick(&app).unwrap(),
        crate::units::worker::Step::Dispatched
    );
    assert_eq!(worker.tick(&app).unwrap(), crate::units::worker::Step::Idle);
    let wire = native_wire.lock().unwrap().take().unwrap();
    let nonce = wire["nonce"].as_str().unwrap().parse::<u64>().unwrap();
    let action = cinder_bulk::wire::Action::limit(
        &native,
        false,
        "3",
        "1",
        cinder_bulk::wire::TimeInForce::Ioc,
    )
    .unwrap();
    let oid = action.order_id(&scope, nonce).unwrap();
    let mut close = body;
    close["data"][0]["orderIdMaker"] = json!(cinder_pacifica::funding::chain::address(oid));
    close["data"][0]["isBuy"] = json!(false);
    close["data"][0]["sequence"] = json!(2);
    app.observe_bulk(
        unit,
        &g,
        Message {
            kind: Kind::Trades,
            body: PrivateBytes::new(serde_json::to_vec(&close).unwrap()).unwrap(),
            received_at: 100,
        },
        CommitId::new([132; 32]).unwrap(),
    )
    .unwrap();
    let j = app.journal.lock().unwrap();
    let s = j.state().unwrap().unit(unit).unwrap();
    assert_eq!(s.orders().len(), 2);
    assert!(
        s.ledger()
            .venue()
            .positions()
            .iter()
            .all(|p| p.quantity().lots() == 0)
    );
    assert_eq!(s.ledger().venue().cash().atoms(), 1000);
    assert_eq!(
        s.ledger()
            .book(BookOwner::Customer(customer))
            .unwrap()
            .cash()
            .atoms(),
        20
    );
    assert_eq!(
        s.orders().iter().map(|o| o.executions.len()).sum::<usize>(),
        2
    );
    assert!(
        s.holds().iter().any(|h| h.active),
        "flat is not a fabricated terminal certificate"
    );
    assert!(
        s.attempts()
            .iter()
            .filter(|a| a.kind == cinder_journal::model::AttemptKind::Cancel)
            .all(|a| !a.possibly_exposed)
    );
}
