//! Joined offline tests of the shipping controller. Only RPC and clock are
//! synthetic; journal, mandate, codec, signatures and receipt recognition are real.
use super::*;
use crate::chain_rpc::{Account, Response};
use crate::runtime::initialization_support as support;
use base64::{Engine, engine::general_purpose::STANDARD};
use cinder_journal::{collateral, funds as lifecycle, model::*, orders, sqlite::*};
use cinder_kernel::{
    amounts::QuoteAtoms,
    identity::*,
    ledger::{evidence::*, *},
};
use cinder_pacifica::{
    funding::{Beneficiary, Route, Setup},
    profile::*,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

struct Fixed;
impl Clock for Fixed {
    fn now(&self) -> Result<u64, Error> {
        Ok(100)
    }
}
struct Advancing(AtomicU64);
impl Clock for Advancing {
    fn now(&self) -> Result<u64, Error> {
        Ok(self.0.load(Ordering::SeqCst))
    }
}
struct BrokenClock;
impl Clock for BrokenClock {
    fn now(&self) -> Result<u64, Error> {
        Err(Error)
    }
}
fn vector(i: usize) -> Value {
    serde_json::from_str::<Value>(include_str!(
        "../../pacifica/tests/fixtures/funding-codec-public.json"
    ))
    .unwrap()["vectors"][i]["contract"]
        .clone()
}
fn field(v: &Value, k: &str) -> [u8; 32] {
    serde_json::from_value(v[k].clone()).unwrap()
}
fn config() -> Config {
    let mut c = support::config();
    c.markets[0] = cinder_kernel::position::Market::new(c.markets[0].unit(), 1_000_000, 1).unwrap();
    for (tag, location) in [(8, Location::Vault), (9, Location::Broker)] {
        c.sources.push(Source {
            scope: EventScope {
                namespace: NamespaceId::new([tag; 32]).unwrap(),
                ..c.sources[0].scope
            },
            location,
        });
    }
    c
}
fn route() -> Route {
    let v = vector(6);
    Route {
        domain: [2; 32],
        pool: field(&v, "pool"),
        funds: field(&v, "funds"),
        beneficiaries: vec![
            Beneficiary {
                account: [1; 32],
                wallet: field(&v, "customer"),
                tokens: field(&v, "recipient_tokens"),
            },
            Beneficiary {
                account: [2; 32],
                wallet: [12; 32],
                tokens: [35; 32],
            },
        ],
        program: field(&v, "program"),
        config: field(&v, "config"),
        vault: field(&v, "vault"),
        mint: field(&v, "mint"),
        broker: field(&v, "broker"),
        broker_tokens: field(&v, "broker_tokens"),
        venue_program: field(&v, "venue_program"),
        venue_vault: field(&v, "venue_vault"),
        epoch: 1,
        decimals: 6,
        withdrawal: Level::Qualified,
        chain: Level::Qualified,
        settings: Level::Qualified,
        withdrawal_cost: 120,
        maximum_movement: 100_000_000,
        maximum_fee: 1_000_000,
        setup_max_age: 10_000,
    }
}
fn controller() -> Controller {
    Controller::new(
        Profile {
            config: config(),
            source: config().sources[0].scope,
            account: chain::address(route().broker),
            environment: cinder_pacifica::execution::Origin::Testnet.url().into(),
            revision: 1,
            evidence: "synthetic joined RPC fixture, not live qualification".into(),
            precision: Level::Qualified,
            fills: Level::Qualified,
            quote_places: 6,
            perp_tag: 0,
            markets: vec![Mapping {
                symbol: "BTC".into(),
                market: 0,
                size: Grid { places: 0, step: 1 },
                price: Grid { places: 0, step: 1 },
            }],
        },
        route(),
        Zeroizing::new([9; 32]),
    )
    .unwrap()
}
fn id(n: u8) -> CommitId {
    CommitId::new([n; 32]).unwrap()
}
fn atoms(n: i128) -> QuoteAtoms {
    QuoteAtoms::new(config().quote, n)
}
fn tx<B: Backend, P: Protection>(
    j: &Journal<B, P>,
    n: u8,
    events: Vec<(Location, Change)>,
    controls: Vec<Control>,
) -> Transaction {
    Transaction {
        id: id(n),
        expected: j.head(),
        at: 100,
        evidence: vec![],
        inputs: events
            .into_iter()
            .enumerate()
            .map(|(i, (location, change))| {
                let source = config()
                    .sources
                    .into_iter()
                    .find(|s| s.location == location)
                    .unwrap()
                    .scope;
                Input {
                    source,
                    source_cut: Some(100),
                    authority_epoch: 1,
                    observed_at: 100,
                    raw: PrivateBytes::new(b"OFFLINE synthetic prerequisite".to_vec()).unwrap(),
                    event: Some(Event {
                        key: RecordKey::Economic(EventKey {
                            scope: source,
                            event: EconomicEventId::new(&[n]).unwrap(),
                            leg: i as u32,
                        }),
                        policy: config().policy,
                        change,
                    }),
                }
            })
            .collect(),
        order_observations: vec![],
        funds_observations: vec![],
        controls,
    }
}
fn setup(temp: &support::Temp) -> (support::Store, Controller) {
    seed(
        Journal::create(
            SqliteBackend::create(&temp.db).unwrap(),
            support::FixtureProtection,
            config(),
        )
        .unwrap(),
    )
}
fn seed<B: Backend, P: Protection>(mut j: Journal<B, P>) -> (Journal<B, P>, Controller) {
    let c = controller();
    let t = tx(
        &j,
        1,
        vec![
            (
                Location::Vault,
                Change::Receipt {
                    owner: Owner::Customer(support::user(1)),
                    location: Location::Vault,
                    amount: atoms(20_000_000),
                },
            ),
            (
                Location::Vault,
                Change::Receipt {
                    owner: Owner::House,
                    location: Location::Vault,
                    amount: atoms(100_000_000),
                },
            ),
        ],
        vec![Control::Order(orders::Action::AdvanceAuthority {
            account: support::user(1),
            epoch: 1,
        })],
    );
    assert_eq!(j.commit(t).unwrap().receipt.controls, None);
    let l = j.state().unwrap().ledger();
    let t = tx(
        &j,
        2,
        vec![(
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
                revision: config().policy,
                markets: vec![collateral::MarginRule {
                    market: config().markets[0].unit(),
                    private_bps: 1000,
                    native_bps: 1000,
                }],
                evidence: EvidencePolicy {
                    max_issue_age: 10_000,
                    max_mark_age: 10_000,
                    max_check_age: 10_000,
                },
            },
            marks: vec![MarkObservation {
                price: support::p(100),
                evidence: EventKey {
                    scope: config().sources[0].scope,
                    event: EconomicEventId::new(&[2]).unwrap(),
                    leg: 0,
                },
                observed_at: 100,
                valid_until: 10_100,
                qualified: true,
            }],
        })],
    );
    assert_eq!(j.commit(t).unwrap().receipt.controls, None);
    c.bind(&mut j, id(3), 100).unwrap();
    assert!(
        c.observe_setup(
            &mut j,
            id(4),
            100,
            Setup {
                account: route().broker,
                observed_at: 100,
                lending_disabled: true,
                borrowed: "0".into(),
                interest: "0".into(),
                complete: true
            }
        )
        .unwrap()
    );
    (j, c)
}
fn prepare<B: Backend, P: Protection>(j: &mut Journal<B, P>, n: u8, rail: Rail) {
    let (source, destination) = match rail {
        Rail::Release => (Location::Vault, Destination::Location(Location::Broker)),
        Rail::Deposit => (Location::Broker, Destination::Location(Location::Venue)),
        Rail::Return => (Location::Broker, Destination::Location(Location::Vault)),
        Rail::Payout => (
            Location::Vault,
            Destination::Recipient(route().beneficiaries[0].tokens),
        ),
        _ => panic!("not a chain rail"),
    };
    let intent = lifecycle::Intent {
        request: support::attempt(n).request,
        source,
        destination,
        net: atoms(20_000_000),
        maximum_fee: atoms(0),
        fee_payer: Owner::House,
        allow_partial: false,
        policy: config().policy,
        authority_epoch: 1,
        expires_at: 10_000,
    };
    let approval = orders::Approval {
        account: support::user(1),
        intent_hash: intent.digest().unwrap(),
        authority_epoch: 1,
    };
    let t = tx(
        j,
        n,
        vec![],
        vec![
            Control::Funds(lifecycle::Action::Accept {
                intent: Box::new(intent),
                approval,
            }),
            Control::Funds(lifecycle::Action::Prepare {
                attempt: support::attempt(n),
                net: atoms(20_000_000),
            }),
        ],
    );
    assert_eq!(j.commit(t).unwrap().receipt.controls, None);
}
struct Rpc {
    requests: Vec<Value>,
    accounts: BTreeMap<[u8; 32], Value>,
    wire: Option<Vec<u8>>,
    contract: Option<Vec<u8>>,
    attempt: AttemptKey,
    lose_ack: bool,
    fail_sim: bool,
    fee: u64,
    missing: bool,
    bad_deposit: bool,
    frozen: bool,
    failed_deposit: bool,
    fail_rpc: bool,
    bad_deltas: bool,
}
#[derive(Clone)]
struct Fake(Arc<Mutex<Rpc>>);
fn account_json(a: &Account) -> Value {
    json!({"owner":chain::address(a.owner),"executable":a.executable,"lamports":a.lamports,"data":[STANDARD.encode(a.data.as_bytes()),"base64"]})
}
fn case_from_vector(i: usize) -> chain_receipt::tests::Case {
    let data: Value = serde_json::from_str(include_str!(
        "../../pacifica/tests/fixtures/funding-codec-public.json"
    ))
    .unwrap();
    let wire = serde_json::from_value::<Vec<u8>>(data["vectors"][i]["wire"].clone()).unwrap();
    let attempt = AttemptKey {
        request: RequestKey {
            domain: config().domain,
            account: AccountId::new([11; 32]).unwrap(),
            request: RequestId::new([12; 32]).unwrap(),
        },
        attempt: AttemptId::new([13; 32]).unwrap(),
    };
    chain_receipt::tests::fixture_from(serde_json::to_vec(&vector(i)).unwrap(), attempt, wire)
}
fn fake(n: u8) -> Fake {
    let c = case_from_vector(0);
    let mut accounts = BTreeMap::new();
    for a in c.accounts.values.into_iter().flatten() {
        accounts.insert(a.key, account_json(&a));
    }
    for a in case_from_vector(6).accounts.values.into_iter().flatten() {
        if let std::collections::btree_map::Entry::Vacant(entry) = accounts.entry(a.key) {
            let mut v = account_json(&a);
            if a.data.as_bytes().len() == 97 {
                let mut b = a.data.as_bytes().to_vec();
                b[81..97].fill(0);
                v["data"][0] = json!(STANDARD.encode(b));
            }
            entry.insert(v);
        }
    }
    for mut a in case_from_vector(2).accounts.values.into_iter().flatten() {
        if a.key == [60; 32] {
            a.key = [62; 32];
        }
        if a.key == route().venue_program {
            let mut b = a.data.as_bytes().to_vec();
            b[4..36].copy_from_slice(&[62; 32]);
            a.data = PrivateBytes::new(b).unwrap();
        }
        if a.key == route().venue_program || a.key == [62; 32] || a.key == route().venue_vault {
            accounts.insert(a.key, account_json(&a));
        }
    }
    // The initial fixture is BEFORE the newly prepared funding operation.
    let bytes = STANDARD
        .decode(accounts[&route().config]["data"][0].as_str().unwrap())
        .unwrap();
    assert_eq!(bytes.len(), 333);
    Fake(Arc::new(Mutex::new(Rpc {
        requests: vec![],
        accounts,
        wire: None,
        contract: None,
        attempt: support::attempt(n),
        lose_ack: false,
        fail_sim: false,
        fee: 5000,
        missing: false,
        bad_deposit: false,
        frozen: false,
        failed_deposit: false,
        fail_rpc: false,
        bad_deltas: false,
    })))
}
impl Transport for Fake {
    fn post(&mut self, body: PrivateBytes) -> Result<Response, Error> {
        let r: Value = serde_json::from_slice(body.as_bytes()).unwrap();
        let mut s = self.0.lock().unwrap();
        s.requests.push(r.clone());
        if s.fail_rpc {
            return Err(Error);
        }
        let result = match r["method"].as_str().unwrap() {
            "getGenesisHash" => json!(chain::address([1; 32])),
            "getSlot" => json!(200),
            "getLatestBlockhash" => {
                json!({"context":{"slot":200},"value":{"blockhash":chain::address([51;32]),"lastValidBlockHeight":400}})
            }
            "getBlockHeight" => json!(200),
            "getFeeForMessage" => json!({"context":{"slot":200},"value":s.fee}),
            "simulateTransaction" => {
                assert_eq!(r["params"][1]["replaceRecentBlockhash"], false);
                assert_eq!(r["params"][1]["sigVerify"], false);
                json!({"context":{"slot":200},"value":{"err":if s.fail_sim{json!({"InstructionError":[0,"Custom"]})}else{Value::Null}}})
            }
            "sendTransaction" => {
                assert!(s.wire.is_none(), "no second signed submission");
                assert_eq!(r["params"][1]["maxRetries"], 0);
                let w = STANDARD.decode(r["params"][0].as_str().unwrap()).unwrap();
                let sig: [u8; 64] = w[1..65].try_into().unwrap();
                s.wire = Some(w);
                if s.lose_ack {
                    return Err(Error);
                }
                json!(chain::signature(sig))
            }
            "getSignatureStatuses" => {
                json!({"context":{"slot":201},"value":[if s.missing{Value::Null}else{json!({"slot":200,"confirmations":null,"err":if s.failed_deposit {json!({"InstructionError":[0,{"Custom":1}]})} else {Value::Null},"confirmationStatus":"finalized"})}]})
            }
            "getTransaction" => {
                let w = s.wire.clone().unwrap();
                if s.failed_deposit {
                    let result = json!({"slot":200,"version":"legacy","transaction":[STANDARD.encode(w),"base64"],"meta":{"err":{"InstructionError":[0,{"Custom":1}]},"fee":5000}});
                    return Ok(Response {
                        status: 200,
                        at: 100,
                        body: PrivateBytes::new(
                            serde_json::to_vec(
                                &json!({"jsonrpc":"2.0","id":r["id"],"result":result}),
                            )
                            .unwrap(),
                        )
                        .unwrap(),
                    });
                }
                let c = if let Some(contract) = &s.contract {
                    chain_receipt::tests::fixture_from(contract.clone(), s.attempt, w.clone())
                } else {
                    let deposit =
                        chain::inspect_customer_deposit(&route(), [1; 32], [1; 32], [41; 32], &w)
                            .unwrap();
                    chain_receipt::tests::fixture_target(
                        deposit.into_target(),
                        vec![],
                        s.attempt,
                        w.clone(),
                    )
                };
                for mut a in c.accounts.values.into_iter().flatten() {
                    if s.frozen && a.data.as_bytes().len() == 333 {
                        let mut b = a.data.as_bytes().to_vec();
                        b[268..276].copy_from_slice(&2u64.to_le_bytes());
                        b[276] = 1;
                        a.data = PrivateBytes::new(b).unwrap();
                    }
                    if s.bad_deposit && a.data.as_bytes().len() == 161 {
                        let mut b = a.data.as_bytes().to_vec();
                        b[145] ^= 1;
                        a.data = PrivateBytes::new(b).unwrap();
                    }
                    if a.key == route().venue_program {
                        let mut b = a.data.as_bytes().to_vec();
                        b[4..36].copy_from_slice(&[62; 32]);
                        a.data = PrivateBytes::new(b).unwrap();
                    }
                    s.accounts.insert(a.key, account_json(&a));
                }
                json!({"slot":200,"version":"legacy","transaction":[STANDARD.encode(w),"base64"],"meta":{"err":null,"fee":s.fee,"preTokenBalances":c.tx.meta["preTokenBalances"],"postTokenBalances":if s.bad_deltas {json!([])} else {c.tx.meta["postTokenBalances"].clone()}}})
            }
            "getMultipleAccounts" => {
                let values = r["params"][0]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|k| {
                        let key = chain::parse_address(k.as_str().unwrap()).unwrap();
                        let mut a = s.accounts.get(&key).cloned().unwrap_or(Value::Null);
                        if let Some(slice) = r["params"][1].get("dataSlice") {
                            let bytes = STANDARD.decode(a["data"][0].as_str().unwrap()).unwrap();
                            let offset = slice["offset"].as_u64().unwrap() as usize;
                            let length = slice["length"].as_u64().unwrap() as usize;
                            let tail = bytes.get(offset..).unwrap_or_default();
                            a["data"][0] = json!(STANDARD.encode(&tail[..tail.len().min(length)]));
                        }
                        a
                    })
                    .collect::<Vec<_>>();
                json!({"context":{"slot":201},"value":values})
            }
            _ => panic!("unexpected RPC method"),
        };
        Ok(Response {
            status: 200,
            at: 100,
            body: PrivateBytes::new(
                serde_json::to_vec(&json!({"jsonrpc":"2.0","id":r["id"],"result":result})).unwrap(),
            )
            .unwrap(),
        })
    }
}
fn loaded(c: &Controller, deposits: Vec<crate::customer_deposit::Locator>) -> Loaded {
    // Parser-valid public CA only; Fake never opens TLS or claims this is the
    // provider's production trust anchor.
    let root = openssl::x509::X509::from_pem(include_bytes!("aws-root.pem"))
        .unwrap()
        .to_der()
        .unwrap();
    let hash = openssl::sha::sha256(&root);
    let d = case_from_vector(0).deployment;
    let mut native = d.clone();
    native.program = route().venue_program;
    native.data = [62; 32];
    Loaded::new(
        Configuration {
            path: "/?api-key=SYNTHETIC-NOT-A-CREDENTIAL".into(),
            custody: d,
            native,
            limits: Limits {
                maximum_age_ms: 1000,
                maximum_fee_lamports: 6000,
                compute_units: 0,
            },
            maximum_calls: 1000,
            expiry_slots: 100,
            poll_ms: 1000,
            deposits,
        },
        &Peer {
            host: "devnet.helius-rpc.com".into(),
            port: 9050,
            root,
            root_hash: hash,
            network: [1; 32],
        },
        Zeroizing::new([7; 32]),
        c,
        [1; 32],
    )
    .unwrap()
}
fn port(c: &Controller, f: &Fake) -> Port<Fake> {
    loaded(c, vec![])
        .with_transport(f.clone(), Arc::new(Fixed))
        .unwrap()
}
#[test]
fn loaded_chain_binds_actual_policy_and_rejects_substituted_seed_network_and_locator() {
    let c = controller();
    let base = loaded(&c, vec![]);
    let peer = Peer {
        host: base.endpoint.host.clone(),
        port: base.endpoint.port,
        root: base.endpoint.root.clone(),
        root_hash: base.endpoint.root_hash,
        network: [1; 32],
    };
    let load = |conf, key, p: &Peer| Loaded::new(conf, p, Zeroizing::new(key), &c, [1; 32]);
    let mut changed = base.configuration.clone();
    changed.path = "/?api-key=OTHER-SYNTHETIC".into();
    assert_ne!(
        base.commitment(),
        load(changed, [7; 32], &peer).unwrap().commitment()
    );
    let mut changed = base.configuration.clone();
    changed.limits.maximum_fee_lamports += 1;
    assert_ne!(
        base.commitment(),
        load(changed, [7; 32], &peer).unwrap().commitment()
    );
    for seed in [[0; 32], [9; 32], [11; 32]] {
        assert!(load(base.configuration.clone(), seed, &peer).is_err());
    }
    let mut wrong = peer.clone();
    wrong.network = [2; 32];
    assert!(load(base.configuration.clone(), [7; 32], &wrong).is_err());
    wrong = peer.clone();
    wrong.root_hash = [0; 32];
    assert!(load(base.configuration.clone(), [7; 32], &wrong).is_err());
    let mut changed = base.configuration.clone();
    changed.deposits = vec![crate::customer_deposit::Locator {
        account: [3; 32],
        operation: [4; 32],
        signature: vec![5; 64],
    }];
    assert!(load(changed, [7; 32], &peer).is_err());
    let mut changed = base.configuration.clone();
    let locator = crate::customer_deposit::Locator {
        account: [1; 32],
        operation: [4; 32],
        signature: vec![5; 64],
    };
    changed.deposits = vec![locator.clone(), locator];
    assert!(load(changed, [7; 32], &peer).is_err());
}
fn deposit(f: &Fake, c: &Controller) -> Port<Fake> {
    let v: Value = serde_json::from_str(include_str!(
        "../../pacifica/tests/fixtures/customer-deposit-public.json"
    ))
    .unwrap();
    let wire: Vec<u8> = serde_json::from_value(v["deposits"][0]["wire"].clone()).unwrap();
    let locator = crate::customer_deposit::Locator {
        account: [1; 32],
        operation: [41; 32],
        signature: wire[1..65].to_vec(),
    };
    f.0.lock().unwrap().wire = Some(wire);
    let l = loaded(c, vec![locator]);
    l.with_transport(f.clone(), Arc::new(Fixed)).unwrap()
}
#[test]
fn customer_deposit_original_receipt_credits_once_without_seeded_customer_funds() {
    let temp = support::Temp::new();
    let mut j = Journal::create(
        SqliteBackend::create(&temp.db).unwrap(),
        support::FixtureProtection,
        config(),
    )
    .unwrap();
    let c = controller();
    let f = fake(10);
    // The owner deposit completed before the later public vault freeze.
    f.0.lock().unwrap().frozen = true;
    let mut p = deposit(&f, &c);
    assert!(p.observe_deposit(&mut j, &c, 0).unwrap() == Outcome::Settled);
    let state = j.state().unwrap().clone();
    assert_eq!(j.transactions().count(), 1);
    let event = j.transactions().next().unwrap().inputs[0]
        .event
        .as_ref()
        .unwrap();
    assert_eq!(j.transactions().next().unwrap().inputs[0].source_cut, None);
    assert!(
        matches!(event.change, Change::Receipt { owner:Owner::Customer(a), amount,.. } if a==support::user(1) && amount.atoms()==9007199254740993)
    );
    assert!(!state.native_funding_ready());
    drop(j);
    drop(p);
    let count = f.0.lock().unwrap().requests.len();
    let mut j = Journal::open(
        SqliteBackend::open(&temp.db, Migration::None).unwrap(),
        support::FixtureProtection,
        config(),
    )
    .unwrap();
    let mut p = deposit(&f, &c);
    assert!(p.observe_deposit(&mut j, &c, 0).unwrap() == Outcome::Settled);
    assert_eq!(j.state().unwrap(), &state);
    assert_eq!(j.transactions().count(), 1);
    assert_eq!(f.0.lock().unwrap().requests.len(), count);
    assert!(
        f.0.lock()
            .unwrap()
            .requests
            .iter()
            .all(|r| r["method"] != "sendTransaction" && r["method"] != "getLatestBlockhash")
    );
}
#[test]
fn collected_deposit_rejoins_later_writer_cut_without_repoll_or_duplicate_credit() {
    let temp = support::Temp::new();
    let mut j = Journal::create(
        SqliteBackend::create(&temp.db).unwrap(),
        support::FixtureProtection,
        config(),
    )
    .unwrap();
    let c = controller();
    let f = fake(10);
    let mut p = deposit(&f, &c);
    let first = p.collect_deposit(c.route(), 0).unwrap();
    let second = p.collect_deposit(c.route(), 0).unwrap();
    let calls = f.0.lock().unwrap().requests.len();
    let mut unrelated = tx(&j, 200, vec![], vec![]);
    unrelated.at = 500;
    unrelated.evidence = vec![PrivateBytes::new(b"unrelated accepted command".to_vec()).unwrap()];
    j.commit(unrelated).unwrap();
    assert!(first.complete(&mut j).unwrap() == Outcome::Settled);
    assert_eq!(j.transactions().last().unwrap().at, 500);
    assert_eq!(j.transactions().last().unwrap().inputs[0].observed_at, 100);
    let state = j.state().unwrap().clone();
    let head = j.head();
    assert!(second.complete(&mut j).unwrap() == Outcome::Settled);
    assert_eq!(j.head(), head);
    assert_eq!(j.state().unwrap(), &state);
    assert_eq!(f.0.lock().unwrap().requests.len(), calls);
    assert!(
        f.0.lock()
            .unwrap()
            .requests
            .iter()
            .all(|r| r["method"] != "sendTransaction")
    );
}
#[test]
fn customer_deposit_missing_history_and_bad_receipt_never_credit() {
    for missing in [true, false] {
        let temp = support::Temp::new();
        let mut j = Journal::create(
            SqliteBackend::create(&temp.db).unwrap(),
            support::FixtureProtection,
            config(),
        )
        .unwrap();
        let c = controller();
        let f = fake(10);
        let mut p = deposit(&f, &c);
        {
            let mut r = f.0.lock().unwrap();
            r.missing = missing;
            r.bad_deposit = !missing;
        }
        let before = j.state().unwrap().clone();
        if missing {
            assert!(p.observe_deposit(&mut j, &c, 0).unwrap() == Outcome::Pending);
        } else {
            assert!(p.observe_deposit(&mut j, &c, 0).is_err());
        }
        assert_eq!(j.state().unwrap(), &before);
        assert_eq!(j.transactions().count(), 0);
    }
}
#[test]
fn rejected_deposit_poll_advances_to_valid_locator_without_crediting_failure() {
    let temp = support::Temp::new();
    let mut j = Journal::create(
        SqliteBackend::create(&temp.db).unwrap(),
        support::FixtureProtection,
        config(),
    )
    .unwrap();
    let c = controller();
    let f = fake(10);
    let original = deposit(&f, &c);
    let valid = original.loaded.configuration.deposits[0].clone();
    let wire = f.0.lock().unwrap().wire.clone().unwrap();
    let mut failed = wire.clone();
    failed[1..65].fill(5);
    let rejected = crate::customer_deposit::Locator {
        account: [1; 32],
        operation: [42; 32],
        signature: failed[1..65].to_vec(),
    };
    let mut p = loaded(&c, vec![rejected, valid])
        .with_transport(f.clone(), Arc::new(Fixed))
        .unwrap();
    {
        let mut r = f.0.lock().unwrap();
        r.wire = Some(failed);
        r.failed_deposit = true;
    }
    let before = j.state().unwrap().clone();
    let mut index = 0;
    crate::runtime::poll_deposit(&mut p, &mut j, &c, &mut index).unwrap();
    assert_eq!(index, 1);
    assert_eq!(j.state().unwrap(), &before);
    assert_eq!(j.transactions().count(), 0);
    assert!(
        f.0.lock()
            .unwrap()
            .requests
            .iter()
            .all(|r| r["method"] != "getMultipleAccounts")
    );
    {
        let mut r = f.0.lock().unwrap();
        r.wire = Some(wire);
        r.failed_deposit = false;
    }
    crate::runtime::poll_deposit(&mut p, &mut j, &c, &mut index).unwrap();
    assert_eq!(index, 0);
    assert_eq!(j.transactions().count(), 1);
    assert!(!j.state().unwrap().native_funding_ready());
}
#[test]
fn immutable_deposit_ineligibility_is_rejected_but_port_and_current_receipt_errors_propagate() {
    for case in 0..5 {
        let temp = support::Temp::new();
        let mut j = Journal::create(
            SqliteBackend::create(&temp.db).unwrap(),
            support::FixtureProtection,
            config(),
        )
        .unwrap();
        let c = controller();
        let f = fake(10);
        let mut p = deposit(&f, &c);
        {
            let mut r = f.0.lock().unwrap();
            match case {
                0 => r.fee = 6001,
                1 => r.bad_deltas = true,
                2 => r.fail_rpc = true,
                3 => r.bad_deposit = true,
                _ => p.clock = Arc::new(BrokenClock),
            }
        }
        let state = j.state().unwrap().clone();
        let mut index = 0;
        if case < 2 {
            assert!(p.observe_deposit(&mut j, &c, 0).unwrap() == Outcome::Rejected);
            crate::runtime::poll_deposit(&mut p, &mut j, &c, &mut index).unwrap();
        } else {
            assert!(crate::runtime::poll_deposit(&mut p, &mut j, &c, &mut index).is_err());
        }
        assert_eq!(index, 0);
        assert_eq!(j.state().unwrap(), &state);
        assert_eq!(j.transactions().count(), 0);
    }
}
#[derive(Default)]
struct StorageFault {
    countdown: AtomicU64,
    durable: AtomicBool,
    fenced: AtomicBool,
}
struct Guarded {
    inner: SqliteBackend,
    fault: Arc<StorageFault>,
}
impl Backend for Guarded {
    fn load(&mut self) -> Result<Vec<cinder_journal::Frame>, cinder_journal::Error> {
        self.inner.load()
    }
    fn append(
        &mut self,
        expected: Option<cinder_journal::Head>,
        frame: &cinder_journal::Frame,
    ) -> Result<(), cinder_journal::Error> {
        let remaining = self.fault.countdown.load(Ordering::SeqCst);
        if remaining > 0 {
            self.fault.countdown.fetch_sub(1, Ordering::SeqCst);
        }
        if remaining == 1 {
            if self.fault.durable.load(Ordering::SeqCst) {
                self.inner.append(expected, frame)?;
            }
            return Err(cinder_journal::Error::Storage);
        }
        self.inner.append(expected, frame)
    }
    fn check_current(
        &mut self,
        expected: cinder_journal::Head,
    ) -> Result<(), cinder_journal::Error> {
        if self.fault.fenced.load(Ordering::SeqCst) {
            return Err(cinder_journal::Error::Stale);
        }
        self.inner.check_current(expected)
    }
}
#[test]
fn wire_persistence_failure_requires_verified_restart_and_retained_wire_never_closes_unsent() {
    for durable in [false, true] {
        let temp = support::Temp::new();
        let fault = Arc::new(StorageFault::default());
        let (mut j, c) = seed(
            Journal::create(
                Guarded {
                    inner: SqliteBackend::create(&temp.db).unwrap(),
                    fault: fault.clone(),
                },
                support::FixtureProtection,
                config(),
            )
            .unwrap(),
        );
        prepare(&mut j, 10, Rail::Release);
        let ledger = j.state().unwrap().ledger().clone();
        let f = fake(10);
        let mut p = port(&c, &f);
        fault.durable.store(durable, Ordering::SeqCst);
        // First append exposes the plan; second retains the signed wire.
        fault.countdown.store(2, Ordering::SeqCst);
        assert!(p.issue(&mut j, &c, support::attempt(10)).is_err());
        assert!(j.state().is_err());
        assert!(
            c.close_unsent_chain(&mut j, id(50), 10_000, support::attempt(10))
                .is_err()
        );
        assert!(
            f.0.lock()
                .unwrap()
                .requests
                .iter()
                .all(|r| r["method"] != "sendTransaction")
        );
        drop(j);
        let mut j = Journal::open(
            Guarded {
                inner: SqliteBackend::open(&temp.db, Migration::None).unwrap(),
                fault: fault.clone(),
            },
            support::FixtureProtection,
            config(),
        )
        .unwrap();
        assert_eq!(
            c.retained_wire(&mut j, support::attempt(10), 10_000)
                .unwrap()
                .is_some(),
            durable
        );
        assert_eq!(
            c.close_unsent_chain(&mut j, id(50), 10_000, support::attempt(10))
                .unwrap(),
            !durable
        );
        assert_eq!(j.state().unwrap().ledger(), &ledger);
        assert_eq!(
            j.state()
                .unwrap()
                .holds()
                .iter()
                .find(|h| h.request == support::attempt(10).request)
                .unwrap()
                .active,
            durable
        );
        assert!(p.issue(&mut j, &c, support::attempt(10)).is_err());
    }
}
#[test]
fn verified_head_failure_fences_deposit_poll_and_unsent_closure() {
    let temp = support::Temp::new();
    let fault = Arc::new(StorageFault::default());
    let (mut j, c) = seed(
        Journal::create(
            Guarded {
                inner: SqliteBackend::create(&temp.db).unwrap(),
                fault: fault.clone(),
            },
            support::FixtureProtection,
            config(),
        )
        .unwrap(),
    );
    prepare(&mut j, 10, Rail::Release);
    let f = fake(10);
    f.0.lock().unwrap().fail_sim = true;
    let mut p = port(&c, &f);
    assert!(p.issue(&mut j, &c, support::attempt(10)).is_err());
    fault.fenced.store(true, Ordering::SeqCst);
    assert!(
        c.close_unsent_chain(&mut j, id(50), 10_000, support::attempt(10))
            .is_err()
    );
    let mut p = deposit(&f, &c);
    let mut index = 0;
    let calls = f.0.lock().unwrap().requests.len();
    assert!(crate::runtime::poll_deposit(&mut p, &mut j, &c, &mut index).is_err());
    assert_eq!(index, 0);
    assert_eq!(f.0.lock().unwrap().requests.len(), calls);
}
fn retain(f: &Fake, j: &mut support::Store, c: &Controller, n: u8) {
    f.0.lock().unwrap().contract = Some(
        c.original_chain_contract(j, support::attempt(n), 100)
            .unwrap()
            .as_bytes()
            .to_vec(),
    );
}
#[test]
fn lost_send_ack_restart_reconciles_original_without_second_signature_or_submission() {
    let temp = support::Temp::new();
    let (mut j, c) = setup(&temp);
    prepare(&mut j, 10, Rail::Release);
    let f = fake(10);
    f.0.lock().unwrap().lose_ack = true;
    let mut p = port(&c, &f);
    assert!(p.issue(&mut j, &c, support::attempt(10)).unwrap() == Outcome::Unknown);
    retain(&f, &mut j, &c, 10);
    let original = c
        .retained_wire(&mut j, support::attempt(10), 100)
        .unwrap()
        .unwrap()
        .wire
        .as_bytes()
        .to_vec();
    assert_eq!(original, f.0.lock().unwrap().wire.clone().unwrap());
    assert!(p.issue(&mut j, &c, support::attempt(10)).is_err());
    let state = j.state().unwrap().clone();
    drop(j);
    drop(p);
    let mut j = Journal::open(
        SqliteBackend::open(&temp.db, Migration::None).unwrap(),
        support::FixtureProtection,
        config(),
    )
    .unwrap();
    assert_eq!(j.state().unwrap(), &state);
    let mut p = port(&c, &f);
    assert!(p.reconcile(&mut j, &c, support::attempt(10)).unwrap() == Outcome::Settled);
    assert!(
        j.state()
            .unwrap()
            .funds()
            .iter()
            .find(|o| o.attempt == Some(support::attempt(10)))
            .unwrap()
            .terminal
    );
    let s = f.0.lock().unwrap();
    assert_eq!(
        s.requests
            .iter()
            .filter(|r| r["method"] == "sendTransaction")
            .count(),
        1
    );
    assert_eq!(
        s.requests
            .iter()
            .filter(|r| r["method"] == "getLatestBlockhash")
            .count(),
        1
    );
}
#[test]
fn failed_simulation_or_excess_fee_exposes_no_signed_wire_and_no_delivery() {
    for simulation in [true, false] {
        let temp = support::Temp::new();
        let (mut j, c) = setup(&temp);
        prepare(&mut j, 10, Rail::Release);
        let f = fake(10);
        {
            let mut s = f.0.lock().unwrap();
            s.fail_sim = simulation;
            if !simulation {
                s.fee = 6001;
            }
        }
        let clock = Arc::new(Advancing(AtomicU64::new(100)));
        let mut p = loaded(&c, vec![])
            .with_transport(f.clone(), clock.clone())
            .unwrap();
        let ledger = j.state().unwrap().ledger().clone();
        assert!(p.issue(&mut j, &c, support::attempt(10)).is_err());
        assert!(
            c.retained_wire(&mut j, support::attempt(10), 100)
                .unwrap()
                .is_none()
        );
        assert!(p.reconcile(&mut j, &c, support::attempt(10)).unwrap() == Outcome::Pending);
        assert!(
            j.state()
                .unwrap()
                .holds()
                .iter()
                .find(|h| h.request == support::attempt(10).request)
                .unwrap()
                .active
        );
        let calls = f.0.lock().unwrap().requests.len();
        clock.0.store(10_000, Ordering::SeqCst);
        assert!(p.reconcile(&mut j, &c, support::attempt(10)).unwrap() == Outcome::Settled);
        let state = j.state().unwrap().clone();
        assert_eq!(state.ledger(), &ledger);
        assert!(
            state
                .funds()
                .iter()
                .find(|o| o.attempt == Some(support::attempt(10)))
                .unwrap()
                .terminal
        );
        assert!(
            !state
                .holds()
                .iter()
                .find(|h| h.request == support::attempt(10).request)
                .unwrap()
                .active
        );
        assert!(
            state
                .attempts()
                .iter()
                .find(|a| a.key == support::attempt(10))
                .unwrap()
                .possibly_exposed
        );
        assert_eq!(f.0.lock().unwrap().requests.len(), calls);
        drop(j);
        let mut j = Journal::open(
            SqliteBackend::open(&temp.db, Migration::None).unwrap(),
            support::FixtureProtection,
            config(),
        )
        .unwrap();
        assert_eq!(j.state().unwrap(), &state);
        assert!(p.reconcile(&mut j, &c, support::attempt(10)).unwrap() == Outcome::Settled);
        assert_eq!(j.state().unwrap(), &state);
        assert!(p.issue(&mut j, &c, support::attempt(10)).is_err());
        assert_eq!(f.0.lock().unwrap().requests.len(), calls);
        assert!(
            c.retained_wire(&mut j, support::attempt(10), 10_000)
                .unwrap()
                .is_none()
        );
        assert!(p.reconcile(&mut j, &c, support::attempt(10)).unwrap() == Outcome::Settled);
        assert!(
            f.0.lock()
                .unwrap()
                .requests
                .iter()
                .all(|r| r["method"] != "sendTransaction")
        );
        assert!(p.issue(&mut j, &c, support::attempt(10)).is_err());
    }
}
#[test]
fn finalized_missing_history_keeps_original_reservation_and_cannot_retry() {
    let temp = support::Temp::new();
    let (mut j, c) = setup(&temp);
    prepare(&mut j, 10, Rail::Release);
    let f = fake(10);
    let mut p = port(&c, &f);
    assert!(p.issue(&mut j, &c, support::attempt(10)).unwrap() == Outcome::Submitted);
    retain(&f, &mut j, &c, 10);
    f.0.lock().unwrap().missing = true;
    let state = j.state().unwrap().clone();
    assert!(p.reconcile(&mut j, &c, support::attempt(10)).unwrap() == Outcome::Pending);
    assert_eq!(j.state().unwrap(), &state);
    assert!(p.issue(&mut j, &c, support::attempt(10)).is_err());
    assert!(
        !c.close_unsent_chain(&mut j, id(50), 10_000, support::attempt(10))
            .unwrap()
    );
    assert_eq!(j.state().unwrap(), &state);
}
#[test]
fn release_return_and_registered_payout_join_one_ledger_but_native_deposit_is_only_a_debit() {
    for next in [Rail::Return, Rail::Deposit] {
        let temp = support::Temp::new();
        let (mut j, c) = setup(&temp);
        prepare(&mut j, 10, Rail::Release);
        let f = fake(10);
        let mut p = port(&c, &f);
        assert!(p.issue(&mut j, &c, support::attempt(10)).unwrap() == Outcome::Submitted);
        retain(&f, &mut j, &c, 10);
        assert!(p.reconcile(&mut j, &c, support::attempt(10)).unwrap() == Outcome::Settled);
        prepare(&mut j, 11, next);
        {
            let mut s = f.0.lock().unwrap();
            s.attempt = support::attempt(11);
            s.wire = None;
            s.contract = None;
        }
        assert!(p.issue(&mut j, &c, support::attempt(11)).unwrap() == Outcome::Submitted);
        retain(&f, &mut j, &c, 11);
        let result = p
            .reconcile(&mut j, &c, support::attempt(11))
            .unwrap_or_else(|_| panic!("reconcile {next:?}"));
        if next == Rail::Deposit {
            assert!(result == Outcome::Pending);
            assert!(
                !j.state()
                    .unwrap()
                    .funds()
                    .iter()
                    .find(|o| o.attempt == Some(support::attempt(11)))
                    .unwrap()
                    .terminal
            );
            assert!(!j.state().unwrap().native_funding_ready());
            assert!(p.issue(&mut j, &c, support::attempt(11)).is_err());
        } else {
            assert!(result == Outcome::Settled);
            prepare(&mut j, 12, Rail::Payout);
            {
                let mut s = f.0.lock().unwrap();
                s.attempt = support::attempt(12);
                s.wire = None;
                s.contract = None;
            }
            assert!(p.issue(&mut j, &c, support::attempt(12)).unwrap() == Outcome::Submitted);
            retain(&f, &mut j, &c, 12);
            assert!(p.reconcile(&mut j, &c, support::attempt(12)).unwrap() == Outcome::Settled);
        }
    }
}
