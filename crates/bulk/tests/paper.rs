//! Offline synthetic paper responses only; no wallets, native requests or AWS.
#[path = "../../journal/tests/support/mod.rs"]
mod support;
use cinder_bulk::{
    Network, Scope,
    execution::{self, Gateway, Reply},
    financial::{Mapping, Profile, Target},
    paper::{self, Policy},
    precision::Market,
    reads,
};
use cinder_journal::{
    Backend, Frame, Head,
    model::*,
    sqlite::{Migration, SqliteBackend},
    units,
};
use cinder_kernel::{identity::*, ledger::*};
use serde_json::{Value, json};
use std::{
    cell::Cell,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use zeroize::Zeroizing;
fn uid() -> UnitId {
    UnitId::new([40; 32]).unwrap()
}
fn config() -> units::Configuration {
    let mut c = support::config();
    c.customers.clear();
    for (n, location) in [(80, Location::Vault), (81, Location::Broker)] {
        let mut s = c.sources[0];
        s.scope.namespace = NamespaceId::new([n; 32]).unwrap();
        s.location = location;
        c.sources.push(s);
    }
    units::Configuration::new(
        c.domain,
        vec![units::Descriptor {
            id: uid(),
            environment: Network::Testnet.commitment(),
            native_account: [41; 32],
            funding_wallet: [42; 32],
            custody_config: [43; 32],
            payout_vault: [44; 32],
            key_policy: [45; 32],
            ledger: c,
        }],
    )
    .unwrap()
}
fn profile(network: Network) -> Profile {
    let c = config().units()[0].ledger.clone();
    Profile::new(
        Scope::new(c.sources[0].scope, network, [41; 32]).unwrap(),
        c.quote,
        0,
        Target::Master,
        vec![Mapping::new(
            Market::new("BTC-USD", "1", "1", 0).unwrap(),
            c.markets[0].unit(),
        )],
    )
    .unwrap()
}
fn gateway() -> Gateway {
    Gateway::new(
        config().units()[0].ledger.clone(),
        profile(Network::Testnet),
        execution::Policy {
            unit: uid(),
            revision: 1,
            evidence: [88; 32],
            qualification: execution::Level::Qualified,
            expiry_ms: 3000,
            credits: 40,
            cleanup_reserve: 5,
        },
        Zeroizing::new([10; 32]),
        1,
    )
    .unwrap()
}
fn policy() -> Policy {
    Policy {
        purpose: "closed-fresh-paper-1".into(),
        run: [90; 32],
        unit: uid().bytes(),
        account: [41; 32],
        customer: [70; 32],
        profile: profile(Network::Testnet).commitment(),
        asset: support::cash(0).unit().asset.bytes(),
        precision: 1,
        setup: [91; 32],
        quote: [92; 32],
        observed_at: 1,
        expires_at: 1000,
        customer_atoms: 20,
        notional_atoms: 20,
        buffer_atoms: 20,
        reference_ticks: 10,
        lower_ticks: 8,
        upper_ticks: 12,
        cleanup_ticks: 12,
    }
}
struct Storage {
    inner: SqliteBackend,
    unknown: Arc<AtomicBool>,
}
impl Backend for Storage {
    fn load(&mut self) -> Result<Vec<Frame>, cinder_journal::Error> {
        self.inner.load()
    }
    fn append(&mut self, h: Option<Head>, f: &Frame) -> Result<(), cinder_journal::Error> {
        self.inner.append(h, f)?;
        if f.head.sequence == 11 && self.unknown.swap(false, Ordering::SeqCst) {
            Err(cinder_journal::Error::Storage)
        } else {
            Ok(())
        }
    }
}
type Store = units::Journal<Storage, support::FixtureProtection>;
fn store(t: &support::Temp, unknown: Arc<AtomicBool>) -> Store {
    let mut j = Store::create(
        Storage {
            inner: SqliteBackend::create(&t.db).unwrap(),
            unknown,
        },
        support::FixtureProtection,
        config(),
    )
    .unwrap();
    j.commit(units::Transaction {
        id: CommitId::new([1; 32]).unwrap(),
        expected: j.head(),
        at: 1,
        change: units::Change::Enroll {
            wallet: [70; 32],
            recipients: vec![units::Recipient {
                unit: uid(),
                tokens: [71; 32],
            }],
        },
    })
    .unwrap();
    gateway()
        .initialize(
            &mut j.select(uid()).unwrap(),
            CommitId::new([2; 32]).unwrap(),
            2,
        )
        .unwrap();
    j
}
fn snapshot() -> Value {
    json!([{"fullAccount":{"kind":"MasterEOA","authorizedAgentWallets":[bs58::encode(gateway().agent_key()).into_string()],"margin":{"totalMargin":"1000","availableMargin":"1000","marginUsed":"0",
        "notional":"0","realizedPnl":"0","unrealizedPnl":"0","fees":"0","funding":"0"},"positions":[],"openOrders":[],
        "pendingOnFill":[],"subAccounts":[],"lockedAmounts":[]}}])
}
fn page() -> Value {
    json!({"data":[],"page":{"hasMore":false,"nextCursor":null,"coverage":"complete","asOfSlot":100,"startSlot":0,"endSlot":100,"minAvailableSlot":0}})
}
struct Port {
    calls: usize,
    bodies: [Value; 4],
    status: u16,
    unknown_after: Option<Arc<AtomicBool>>,
}
impl Port {
    fn new() -> Self {
        Self {
            calls: 0,
            bodies: [snapshot(), page(), page(), page()],
            status: 200,
            unknown_after: None,
        }
    }
}
impl reads::Transport for Port {
    fn query(&mut self, r: reads::Request) -> Reply {
        let q = r.consume(10).unwrap();
        assert_eq!(q.network(), Network::Testnet);
        let v: Value = serde_json::from_slice(q.body()).unwrap();
        assert_eq!(
            v["type"],
            ["fullAccount", "fills", "fundingHistory", "riskHistory"][self.calls]
        );
        let body =
            PrivateBytes::new(serde_json::to_vec(&self.bodies[self.calls]).unwrap()).unwrap();
        self.calls += 1;
        if self.calls == 4
            && let Some(flag) = &self.unknown_after
        {
            flag.store(true, Ordering::SeqCst);
        }
        Reply::Response {
            status: self.status,
            body,
            received_at: 10,
            retry_after_ms: None,
        }
    }
}
fn init(p: &Policy, j: &mut Store, port: &mut Port) -> Result<(), cinder_bulk::Error> {
    paper::initialize(p, &gateway(), &mut j.select(uid()).unwrap(), port, || {
        Ok(10)
    })
}
fn assert_empty(j: &Store) {
    let s = j.state().unwrap().unit(uid()).unwrap();
    assert_eq!(s.ledger().venue().cash().atoms(), 0);
    assert!(!s.has_joined_risk());
}
#[test]
fn observed_cash_is_split_once_and_protected_reopen_never_requeries_or_recredits() {
    let t = support::Temp::new();
    let flag = Arc::new(AtomicBool::new(false));
    let mut j = store(&t, flag.clone());
    let mut port = Port::new();
    let result = init(&policy(), &mut j, &mut port);
    assert!(
        result.is_ok(),
        "{result:?}, calls {} head {}",
        port.calls,
        j.head().sequence
    );
    assert_eq!(port.calls, 4);
    let head = j.head();
    let s = j.state().unwrap().unit(uid()).unwrap();
    let l = s.ledger();
    let account = cinder_journal::onboarding::account(config().domain(), [70; 32]).unwrap();
    assert_eq!(l.venue().cash().atoms(), 1000);
    assert_eq!(l.book(Owner::Customer(account)).unwrap().cash().atoms(), 20);
    assert_eq!(l.book(Owner::House).unwrap().cash().atoms(), 980);
    assert_eq!(l.vault().atoms(), 0);
    assert_eq!(l.broker().atoms(), 0);
    assert!(s.has_joined_risk());
    assert!(s.raw_inputs().iter().all(|e| e.resolved.is_some()));
    init(&policy(), &mut j, &mut port).unwrap();
    assert_eq!(j.head(), head);
    assert_eq!(port.calls, 4);
    drop(j);
    let mut j = Store::open(
        Storage {
            inner: SqliteBackend::open(&t.db, Migration::None).unwrap(),
            unknown: flag,
        },
        support::FixtureProtection,
        config(),
    )
    .unwrap();
    init(&policy(), &mut j, &mut port).unwrap();
    assert_eq!(j.head(), head);
    assert_eq!(port.calls, 4);
    for variant in 0..4 {
        let mut p = policy();
        match variant {
            0 => p.run = [99; 32],
            1 => p.setup = [99; 32],
            2 => p.customer_atoms = 19,
            _ => p.quote = [99; 32],
        };
        assert!(init(&p, &mut j, &mut port).is_err());
        assert_eq!(j.head(), head);
    }
}
#[test]
fn insufficient_or_nonfresh_or_missing_evidence_never_posts_cash() {
    for variant in 0..12 {
        let t = support::Temp::new();
        let mut j = store(&t, Arc::new(AtomicBool::new(false)));
        let mut port = Port::new();
        let account = &mut port.bodies[0][0]["fullAccount"];
        match variant {
            0 => {
                account["margin"]["totalMargin"] = json!("39");
                account["margin"]["availableMargin"] = json!("39");
            }
            1 => account["margin"]["fees"] = json!("-1"),
            2 => account["margin"]["funding"] = json!("1"),
            3 => account["lockedAmounts"] = json!([{}]),
            4 => account["openOrders"] = json!([{}]),
            5 => account["pendingOnFill"] = json!([{}]),
            6 => account["subAccounts"] = json!([{"pubkey":"not-an-address"}]),
            7 => port.bodies[1]["page"]["coverage"] = json!("partial"),
            8 => port.bodies[2]["page"]["startSlot"] = json!(1),
            9 => port.bodies[3]["page"]["minAvailableSlot"] = json!(1),
            10 => port.status = 500,
            _ => account["margin"]["totalMargin"] = Value::Null,
        }
        assert!(
            init(&policy(), &mut j, &mut port).is_err(),
            "variant {variant}"
        );
        assert_empty(&j);
        let count = port.calls;
        assert!(init(&policy(), &mut j, &mut port).is_err());
        assert_eq!(port.calls, count);
        assert_empty(&j);
    }
}
#[test]
fn wrong_scope_purpose_asset_account_expiry_and_mainnet_are_refused_before_io() {
    let t = support::Temp::new();
    let mut j = store(&t, Arc::new(AtomicBool::new(false)));
    let mut port = Port::new();
    for variant in 0..10 {
        let mut p = policy();
        match variant {
            0 => p.unit = [1; 32],
            1 => p.account = [1; 32],
            2 => p.asset = [1; 32],
            3 => p.precision = 2,
            4 => p.profile = [1; 32],
            5 => p.purpose = "production".into(),
            6 => p.expires_at = 10,
            7 => p.setup = [0; 32],
            8 => p.customer = [1; 32],
            _ => p.customer = p.account,
        };
        assert!(init(&p, &mut j, &mut port).is_err());
        assert_eq!(port.calls, 0);
        assert_empty(&j);
    }
    let mut d = config().units()[0].clone();
    d.environment = Network::Mainnet.commitment();
    assert!(policy().validate(&gateway(), &d).is_err());
    let mut p = policy();
    p.profile = profile(Network::Mainnet).commitment();
    assert!(p.validate(&gateway(), &config().units()[0]).is_err());
}
#[test]
fn expiry_during_observation_never_posts_and_no_original_request_is_retried() {
    let t = support::Temp::new();
    let mut j = store(&t, Arc::new(AtomicBool::new(false)));
    let mut port = Port::new();
    let count = Cell::new(0);
    assert!(
        paper::initialize(
            &policy(),
            &gateway(),
            &mut j.select(uid()).unwrap(),
            &mut port,
            || {
                count.set(count.get() + 1);
                Ok(if count.get() > 2 { 1000 } else { 10 })
            }
        )
        .is_err()
    );
    assert_eq!(port.calls, 1);
    assert_empty(&j);
    assert!(init(&policy(), &mut j, &mut port).is_err());
    assert_eq!(port.calls, 1);
}
#[test]
fn unknown_bootstrap_commit_is_recovered_once_without_reexposure() {
    let t = support::Temp::new();
    let flag = Arc::new(AtomicBool::new(false));
    let mut j = store(&t, flag.clone());
    let mut port = Port::new();
    port.unknown_after = Some(flag.clone());
    assert_eq!(
        init(&policy(), &mut j, &mut port),
        Err(cinder_bulk::Error::Unavailable)
    );
    assert_eq!(port.calls, 4);
    drop(j);
    let mut j = Store::open(
        Storage {
            inner: SqliteBackend::open(&t.db, Migration::None).unwrap(),
            unknown: flag,
        },
        support::FixtureProtection,
        config(),
    )
    .unwrap();
    assert_eq!(
        j.state()
            .unwrap()
            .unit(uid())
            .unwrap()
            .ledger()
            .venue()
            .cash()
            .atoms(),
        1000
    );
    let head = j.head();
    init(&policy(), &mut j, &mut port).unwrap();
    assert_eq!(j.head(), head);
    assert_eq!(port.calls, 4);
}
#[test]
fn native_unknown_coverage_and_absent_optional_fields_stay_named_paper_assumptions() {
    let t = support::Temp::new();
    let mut j = store(&t, Arc::new(AtomicBool::new(false)));
    let mut port = Port::new();
    for name in ["lockedAmounts", "pendingOnFill", "subAccounts"] {
        port.bodies[0][0]["fullAccount"]
            .as_object_mut()
            .unwrap()
            .remove(name);
    }
    for body in &mut port.bodies[1..] {
        body["page"]["coverage"] = json!("unknown");
    }
    init(&policy(), &mut j, &mut port).unwrap();
    assert_eq!(
        j.state()
            .unwrap()
            .unit(uid())
            .unwrap()
            .ledger()
            .venue()
            .cash()
            .atoms(),
        1000
    );
    // Retained source coverage remains None, never a fabricated native frontier.
    let selected = j.select(uid()).unwrap();
    use cinder_journal::controller::Controller;
    assert!(
        selected
            .transactions()
            .flat_map(|t| &t.inputs)
            .all(|i| i.source_cut.is_none())
    );
}
#[test]
fn unexpected_funding_is_retained_as_unresolved_not_silently_ignored() {
    let t = support::Temp::new();
    let mut j = store(&t, Arc::new(AtomicBool::new(false)));
    let mut port = Port::new();
    init(&policy(), &mut j, &mut port).unwrap();
    let mut body = snapshot();
    body[0]["fullAccount"]["margin"]["funding"] = json!("1");
    cinder_bulk::observation::ingest(
        &gateway(),
        &mut j.select(uid()).unwrap(),
        cinder_bulk::observation::Message {
            kind: cinder_bulk::observation::Kind::Account,
            body: PrivateBytes::new(serde_json::to_vec(&body).unwrap()).unwrap(),
            received_at: 10,
        },
        CommitId::new([99; 32]).unwrap(),
        10,
    )
    .unwrap();
    let state = j.state().unwrap().unit(uid()).unwrap();
    assert_eq!(state.unresolved_raw(), 1);
    assert_eq!(state.ledger().venue().cash().atoms(), 1000);
    assert!(state.risk_report().is_err());
}
#[test]
fn fresh_empty_zero_slot_native_histories_initialize_without_inventing_a_frontier() {
    let t = support::Temp::new();
    let mut j = store(&t, Arc::new(AtomicBool::new(false)));
    let mut port = Port::new();
    // Exact empty-page shape returned by all three native history endpoints in
    // the October 21 frozen-image hardware attempt. No positive cut exists.
    for body in &mut port.bodies[1..] {
        *body = json!({"data":[],"page":{"hasMore":false,"nextCursor":null,
            "coverage":"unknown","asOfSlot":0,"startSlot":0,"endSlot":0}});
    }
    init(&policy(), &mut j, &mut port).unwrap();
    assert_eq!(port.calls, 4);
    let s = j.state().unwrap().unit(uid()).unwrap();
    assert_eq!(s.ledger().venue().cash().atoms(), 1000);
    assert_eq!(s.ledger().book(Owner::House).unwrap().cash().atoms(), 980);
    assert!(s.raw_inputs().is_empty());
    assert!(s.has_joined_risk());
    use cinder_journal::controller::Controller;
    let selected = j.select(uid()).unwrap();
    assert!(
        selected
            .transactions()
            .flat_map(|t| &t.inputs)
            .all(|i| i.source_cut.is_none())
    );
    assert!(
        selected
            .transactions()
            .flat_map(|t| &t.controls)
            .all(|c| !matches!(c, Control::ResolveRaw(_)))
    );
    let head = j.head();
    init(&policy(), &mut j, &mut port).unwrap();
    assert_eq!(j.head(), head);
    assert_eq!(port.calls, 4);
}
#[test]
fn zero_slot_pages_do_not_bypass_ordinary_history_containment_or_bad_bootstrap_evidence() {
    use cinder_bulk::observation::{self, Kind, Message};
    let empty = json!({"data":[],"page":{"hasMore":false,"nextCursor":null,
        "coverage":"unknown","asOfSlot":0,"startSlot":0,"endSlot":0}});
    let t = support::Temp::new();
    let mut j = store(&t, Arc::new(AtomicBool::new(false)));
    init(&policy(), &mut j, &mut Port::new()).unwrap();
    for (i, kind) in [Kind::Funding, Kind::Risk].into_iter().enumerate() {
        observation::ingest(
            &gateway(),
            &mut j.select(uid()).unwrap(),
            Message {
                kind,
                body: PrivateBytes::new(serde_json::to_vec(&empty).unwrap()).unwrap(),
                received_at: 10,
            },
            CommitId::new([100 + i as u8; 32]).unwrap(),
            10,
        )
        .unwrap();
    }
    let s = j.state().unwrap().unit(uid()).unwrap();
    assert_eq!(s.unresolved_raw(), 2);
    assert_eq!(s.ledger().venue().cash().atoms(), 1000);
    assert!(s.risk_report().is_err());
    for variant in 0..6 {
        let t = support::Temp::new();
        let mut j = store(&t, Arc::new(AtomicBool::new(false)));
        let mut port = Port::new();
        for body in &mut port.bodies[1..] {
            *body = empty.clone();
        }
        let body = &mut port.bodies[2];
        match variant {
            0 => body["page"]["coverage"] = json!("partial"),
            1 => {
                body["page"]["hasMore"] = json!(true);
                body["page"]["nextCursor"] = json!("more");
            }
            2 => body["page"]["minAvailableSlot"] = json!(1),
            3 => body["data"] = json!([{}]),
            4 => {
                body.as_object_mut().unwrap().remove("page");
            }
            _ => {
                body["page"]["startSlot"] = json!(1);
                body["page"]["endSlot"] = json!(2);
                body["page"]["asOfSlot"] = json!(2);
            }
        }
        assert!(
            init(&policy(), &mut j, &mut port).is_err(),
            "variant {variant}"
        );
        assert_empty(&j);
        let calls = port.calls;
        assert!(init(&policy(), &mut j, &mut port).is_err());
        assert_eq!(port.calls, calls);
    }
}
fn intent(n: u8, lots: i64, maximum: u64, reduce: bool) -> cinder_journal::orders::Intent {
    let c = config().units()[0].ledger.clone();
    let m = c.markets[0].unit();
    cinder_journal::orders::Intent {
        time_in_force: cinder_journal::orders::TimeInForce::ImmediateOrCancel,
        request: RequestKey {
            domain: c.domain,
            account: cinder_journal::onboarding::account(c.domain, [70; 32]).unwrap(),
            request: RequestId::new([n; 32]).unwrap(),
        },
        quantity: cinder_kernel::amounts::QuantityLots::new(m, lots),
        minimum: cinder_kernel::amounts::PriceTicks::new(m, 8).unwrap(),
        maximum: cinder_kernel::amounts::PriceTicks::new(m, maximum).unwrap(),
        maximum_fee_per_lot: support::cash(0),
        reduce_only: reduce,
        policy: c.policy,
        authority_epoch: 1,
        expires_at: 900,
    }
}
#[test]
fn original_order_budget_reserves_cleanup_and_never_counts_cancellation_as_capacity() {
    let t = support::Temp::new();
    let mut j = store(&t, Arc::new(AtomicBool::new(false)));
    let mut port = Port::new();
    init(&policy(), &mut j, &mut port).unwrap();
    let opening = intent(100, 1, 8, false);
    let state = j.state().unwrap().unit(uid()).unwrap();
    assert!(policy().order(state, &opening).is_ok());
    assert!(policy().order(state, &intent(101, 2, 8, false)).is_err());
    assert!(policy().order(state, &intent(101, 1, 9, false)).is_err());
    // This is a trusted ledger-level authority fixture; the service test checks
    // the actual Ed25519 user request rather than treating this as a signature.
    let holds = j
        .state()
        .unwrap()
        .unit(uid())
        .unwrap()
        .order_reservations(&opening)
        .unwrap();
    let approval = cinder_journal::orders::Approval {
        account: opening.request.account,
        intent_hash: opening.digest().unwrap(),
        authority_epoch: 1,
    };
    let record = units::api::Record {
        unit: uid(),
        request: opening.request,
        policy: opening.policy,
        digest: [100; 32],
        epoch: 1,
        signer: [70; 32],
        contract: [90; 32],
        effect: units::api::Effect::Recorded,
        wire: PrivateBytes::new(b"trusted header fixture; NOT an API signature".to_vec()).unwrap(),
    };
    let tx = Transaction {
        id: CommitId::new([100; 32]).unwrap(),
        expected: j.head(),
        at: 10,
        evidence: vec![],
        inputs: vec![],
        order_observations: vec![],
        funds_observations: vec![],
        controls: vec![Control::Order(cinder_journal::orders::Action::Accept {
            intent: Box::new(opening),
            approval,
            reservations: holds,
        })],
    };
    let receipt = j
        .commit(units::Transaction {
            id: tx.id,
            expected: tx.expected,
            at: tx.at,
            change: units::Change::Admit(Box::new(units::api::Admission {
                record,
                kind: units::api::Kind::Order,
                proposal: tx,
            })),
        })
        .unwrap();
    assert!(matches!(
        receipt.receipt,
        units::Receipt::Admitted {
            receipt: Receipt { controls: None, .. },
            ..
        }
    ));
    let state = j.state().unwrap().unit(uid()).unwrap();
    assert!(policy().order(state, &intent(101, 1, 8, false)).is_err());
    assert!(policy().order(state, &intent(101, -1, 12, true)).is_ok());
    assert!(policy().order(state, &intent(101, -2, 12, true)).is_err());
    assert!(policy().order(state, &intent(101, 1, 12, true)).is_err());
    let mut gtc = intent(101, -1, 12, true);
    gtc.time_in_force = cinder_journal::orders::TimeInForce::GoodTilCancelled;
    assert!(policy().order(state, &gtc).is_err());
}
