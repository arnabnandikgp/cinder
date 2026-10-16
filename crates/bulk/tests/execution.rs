//! Synthetic native qualification only. Trusted journal API-header fixtures are
//! NOT valid private API signatures; P33 tests own that separate verification.
//! Actual Ed25519 native signatures are verified independently below. No I/O.
#[path = "../../journal/tests/support/mod.rs"]
mod support;
use cinder_bulk::{
    Error, Network, Scope,
    execution::*,
    financial::{self, Mapping, Target},
    observation::{self, Kind, Message},
    precision::Market as NativeMarket,
};
use cinder_journal::{
    Backend, Frame, Head, collateral,
    controller::Controller,
    model::*,
    orders, risk,
    sqlite::{Migration, SqliteBackend},
    units,
};
use cinder_kernel::{
    identity::*,
    ledger::{evidence::*, *},
};
use ed25519_dalek::{Signature, SigningKey, Verifier, VerifyingKey};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use zeroize::Zeroizing;

fn uid(n: u8) -> UnitId {
    UnitId::new([n; 32]).unwrap()
}
fn account() -> AccountId {
    cinder_journal::onboarding::account(support::config().domain, [70; 32]).unwrap()
}
fn request(n: u8) -> RequestKey {
    RequestKey {
        domain: support::config().domain,
        account: account(),
        request: RequestId::new([n; 32]).unwrap(),
    }
}
fn config() -> units::Configuration {
    units::Configuration::new(
        support::config().domain,
        [40_u8, 50]
            .into_iter()
            .map(|n| {
                let mut ledger = support::config();
                ledger.customers.clear();
                ledger.venue = VenueId::new([n + 4; 32]).unwrap();
                ledger.venue_account = VenueAccountId::new([n + 5; 32]).unwrap();
                ledger.sources[0].scope.venue = ledger.venue;
                ledger.sources[0].scope.account = ledger.venue_account;
                let mut vault = ledger.sources[0];
                vault.location = Location::Vault;
                vault.scope.namespace = NamespaceId::new([8; 32]).unwrap();
                ledger.sources.push(vault);
                units::Descriptor {
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
            .collect(),
    )
    .unwrap()
}
struct Storage {
    inner: SqliteBackend,
    unknown: Arc<AtomicBool>,
}
impl Backend for Storage {
    fn load(&mut self) -> Result<Vec<Frame>, cinder_journal::Error> {
        self.inner.load()
    }
    fn append(
        &mut self,
        expected: Option<Head>,
        frame: &Frame,
    ) -> Result<(), cinder_journal::Error> {
        self.inner.append(expected, frame)?;
        if self.unknown.swap(false, Ordering::SeqCst) {
            Err(cinder_journal::Error::Storage)
        } else {
            Ok(())
        }
    }
}
type Store = units::Journal<Storage, support::FixtureProtection>;
fn proposal(j: &Store, n: u8, controls: Vec<Control>, events: Vec<Event>) -> Transaction {
    Transaction {
        id: CommitId::new([n; 32]).unwrap(),
        expected: j.head(),
        at: 10,
        evidence: vec![],
        inputs: events
            .into_iter()
            .map(|e| Input {
                source: match &e.key {
                    RecordKey::Economic(k) => k.scope,
                    _ => panic!("economic fixture"),
                },
                source_cut: Some(u64::from(n)),
                authority_epoch: 1,
                observed_at: 10,
                raw: support::raw(b"synthetic native fixture"),
                event: Some(e),
            })
            .collect(),
        order_observations: vec![],
        funds_observations: vec![],
        controls,
    }
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
fn commit(j: &mut Store, n: u8, controls: Vec<Control>, events: Vec<Event>) {
    let p = proposal(j, n, controls, events);
    let r = j.select(uid(40)).unwrap().commit(p).unwrap();
    assert_eq!(r.receipt.controls, None);
}
fn fixture(t: &support::Temp) -> (Store, Arc<AtomicBool>) {
    let unknown = Arc::new(AtomicBool::new(false));
    let mut j = Store::create(
        Storage {
            inner: SqliteBackend::create(&t.db).unwrap(),
            unknown: unknown.clone(),
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
            recipients: vec![
                units::Recipient {
                    unit: uid(40),
                    tokens: [71; 32],
                },
                units::Recipient {
                    unit: uid(50),
                    tokens: [72; 32],
                },
            ],
        },
    })
    .unwrap();
    let cfg = config();
    let c = &cfg.units()[0].ledger;
    commit(
        &mut j,
        2,
        vec![],
        vec![
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
        ],
    );
    let ledger = j.state().unwrap().unit(uid(40)).unwrap().ledger();
    let check = event(
        c,
        4,
        0,
        Change::Reconcile(NativeCheck {
            expected_version: ledger.version(),
            cash: Some(ledger.venue().cash()),
            funding: Some(ledger.venue().funding()),
            positions: Some(ledger.venue().positions().to_vec()),
            complete: true,
            resolves: vec![],
        }),
    );
    commit(&mut j, 3, vec![], vec![check]);
    let expected_version = j.state().unwrap().unit(uid(40)).unwrap().ledger().version();
    commit(
        &mut j,
        4,
        vec![
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
                        max_mark_age: 1000,
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
                    valid_until: 1000,
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
                    valid_until: 1000,
                    paths: vec![risk::Path {
                        id: [1; 32],
                        steps: vec![risk::Step {
                            after_ms: 1,
                            marks: vec![support::p(100)],
                            events: vec![],
                            liquidity: [Location::Venue, Location::Vault]
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
        ],
        vec![],
    );
    (j, unknown)
}
fn admit(j: &mut Store) -> AttemptKey {
    let cfg = config();
    let c = &cfg.units()[0].ledger;
    let intent = orders::Intent {
        request: request(20),
        quantity: support::q(2),
        minimum: support::p(90),
        maximum: support::p(110),
        maximum_fee_per_lot: support::cash(1),
        time_in_force: orders::TimeInForce::GoodTilCancelled,
        reduce_only: false,
        policy: c.policy,
        authority_epoch: 1,
        expires_at: 1000,
    };
    let controls = vec![Control::Order(orders::Action::Accept {
        approval: orders::Approval {
            account: account(),
            intent_hash: intent.digest().unwrap(),
            authority_epoch: 1,
        },
        intent: Box::new(intent),
        reservations: vec![
            Reservation {
                resource: Resource::Customer(account()),
                amount: support::cash(100),
            },
            Reservation {
                resource: Resource::Location(Location::Venue),
                amount: support::cash(100),
            },
        ],
    })];
    let proposal = proposal(j, 5, controls, vec![]);
    let record = units::api::Record {
        unit: uid(40),
        request: request(20),
        policy: c.policy,
        digest: [20; 32],
        epoch: 1,
        signer: [70; 32],
        contract: [90; 32],
        effect: units::api::Effect::Recorded,
        wire: support::raw(b"trusted journal header fixture only"),
    };
    let r = j
        .commit(units::Transaction {
            id: proposal.id,
            expected: proposal.expected,
            at: 10,
            change: units::Change::Admit(Box::new(units::api::Admission {
                record,
                kind: units::api::Kind::Order,
                proposal,
            })),
        })
        .unwrap();
    assert!(matches!(
        r.receipt,
        units::Receipt::Admitted {
            receipt: Receipt { controls: None, .. },
            ..
        }
    ));
    let key = AttemptKey {
        request: request(20),
        attempt: AttemptId::new([30; 32]).unwrap(),
    };
    commit(
        j,
        6,
        vec![Control::Order(orders::Action::Prepare { attempt: key })],
        vec![],
    );
    key
}
fn profile() -> financial::Profile {
    let cfg = config();
    let c = &cfg.units()[0].ledger;
    financial::Profile::new(
        Scope::new(c.sources[0].scope, Network::Testnet, [41; 32]).unwrap(),
        c.quote,
        0,
        Target::Master,
        vec![Mapping::new(
            NativeMarket::new("BTC-USD", "1", "1", 0).unwrap(),
            c.markets[0].unit(),
        )],
    )
    .unwrap()
}
fn policy() -> Policy {
    Policy {
        unit: uid(40),
        revision: 1,
        evidence: [88; 32],
        qualification: Level::Qualified,
        expiry_ms: 500,
        credits: 3,
        cleanup_reserve: 1,
    }
}
fn gateway(epoch: u64) -> Gateway {
    Gateway::new(
        config().units()[0].ledger.clone(),
        profile(),
        policy(),
        Zeroizing::new([10; 32]),
        epoch,
    )
    .unwrap()
}
fn dispatch(key: AttemptKey, n: u8) -> Dispatch {
    Dispatch {
        attempt: key,
        commit: CommitId::new([n; 32]).unwrap(),
        nonce: u64::from(n),
        at: 10,
    }
}

#[test]
fn native_agent_signs_only_after_unit_exposure_and_ack_never_posts_a_fill() {
    let t = support::Temp::new();
    let (mut j, _) = fixture(&t);
    let key = admit(&mut j);
    let g = gateway(1);
    g.initialize(
        &mut j.select(uid(40)).unwrap(),
        CommitId::new([7; 32]).unwrap(),
        10,
    )
    .unwrap();
    let (out, c) = g
        .prepare_dispatch(&mut j.select(uid(40)).unwrap(), dispatch(key, 8))
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(out.body()).unwrap();
    let nonce = value["nonce"].as_str().unwrap().parse::<u64>().unwrap();
    assert_eq!(nonce, 8);
    let action = cinder_bulk::wire::Action::limit(
        &NativeMarket::new("BTC-USD", "1", "1", 0).unwrap(),
        true,
        "110",
        "2",
        cinder_bulk::wire::TimeInForce::Gtc,
    )
    .unwrap();
    let scope = Scope::new(
        config().units()[0].ledger.sources[0].scope,
        Network::Testnet,
        [41; 32],
    )
    .unwrap();
    let bytes = bs58::decode(value["signature"].as_str().unwrap())
        .into_vec()
        .unwrap();
    VerifyingKey::from_bytes(&g.agent_key())
        .unwrap()
        .verify(
            &action.message(&scope, 8).unwrap(),
            &Signature::from_slice(&bytes).unwrap(),
        )
        .unwrap();
    assert_ne!(g.agent_key(), [41; 32]);
    assert_eq!(out.path(), "/api/v1/order");
    assert_eq!(out.network(), Network::Testnet);
    let oid = bs58::encode(action.order_id(&scope, 8).unwrap()).into_string();
    let body = serde_json::json!({"status":"ok","response":{"type":"order","data":{"statuses":[{"filled":{"oid":oid,"totalSz":2,"avgPx":100}}]}}});
    assert_eq!(
        g.complete_dispatch(
            &mut j.select(uid(40)).unwrap(),
            c,
            Reply::Response {
                status: 200,
                body: PrivateBytes::new(serde_json::to_vec(&body).unwrap()).unwrap(),
                received_at: 11,
                retry_after_ms: None
            },
            11
        )
        .unwrap(),
        Outcome::Acknowledged
    );
    let state = j.state().unwrap().unit(uid(40)).unwrap();
    assert!(state.orders()[0].acknowledged);
    assert_eq!(state.orders()[0].filled.lots(), 0);
    assert!(
        state
            .ledger()
            .book(Owner::Customer(account()))
            .unwrap()
            .positions()
            .iter()
            .all(|p| p.quantity().lots() == 0)
    );
    assert_eq!(
        state
            .ledger()
            .book(Owner::Customer(account()))
            .unwrap()
            .cash()
            .atoms(),
        2000
    );
    assert!(state.holds()[0].active);
    assert!(
        g.prepare_dispatch(&mut j.select(uid(40)).unwrap(), dispatch(key, 8))
            .is_err()
    );
    assert!(
        g.prepare_dispatch(&mut j.select(uid(50)).unwrap(), dispatch(key, 9))
            .is_err()
    );
}

#[test]
fn uncertain_exposure_delivers_nothing_and_restart_cannot_regenerate_a_signature() {
    let t = support::Temp::new();
    let (mut j, unknown) = fixture(&t);
    let key = admit(&mut j);
    let g = gateway(1);
    g.initialize(
        &mut j.select(uid(40)).unwrap(),
        CommitId::new([7; 32]).unwrap(),
        10,
    )
    .unwrap();
    unknown.store(true, Ordering::SeqCst);
    assert!(matches!(
        g.prepare_dispatch(&mut j.select(uid(40)).unwrap(), dispatch(key, 8)),
        Err(Error::Unavailable)
    ));
    assert!(j.state().is_err());
    drop(j);
    let mut j = Store::open(
        Storage {
            inner: SqliteBackend::open(&t.db, Migration::None).unwrap(),
            unknown,
        },
        support::FixtureProtection,
        config(),
    )
    .unwrap();
    assert!(j.state().unwrap().unit(uid(40)).unwrap().attempts()[0].possibly_exposed);
    assert!(
        g.prepare_dispatch(&mut j.select(uid(40)).unwrap(), dispatch(key, 9))
            .is_err()
    );
    assert!(j.state().unwrap().unit(uid(40)).unwrap().holds()[0].active);
}

fn stream(oid: [u8; 32], price: u64) -> PrivateBytes {
    PrivateBytes::new(serde_json::to_vec(&serde_json::json!({"type":"account","topic":format!("account.{}",bs58::encode([41;32]).into_string()),
        "data":{"type":"fill","tradeId":"100:1","symbol":"BTC-USD","orderId":bs58::encode(oid).into_string(),"price":price,"size":1,"fee":1,"isBuy":true,"maker":false,"reasonCode":0,"timestamp":10000000000_u64}})).unwrap()).unwrap()
}
#[test]
fn actual_fill_fee_post_once_to_original_owner_even_after_key_revoke_and_unknowns_stay_unresolved()
{
    let t = support::Temp::new();
    let (mut j, _) = fixture(&t);
    let key = admit(&mut j);
    let g = gateway(1);
    g.initialize(
        &mut j.select(uid(40)).unwrap(),
        CommitId::new([7; 32]).unwrap(),
        10,
    )
    .unwrap();
    let (_out, c) = g
        .prepare_dispatch(&mut j.select(uid(40)).unwrap(), dispatch(key, 8))
        .unwrap();
    g.complete_dispatch(&mut j.select(uid(40)).unwrap(), c, Reply::Unknown, 10)
        .unwrap();
    g.deactivate(
        &mut j.select(uid(40)).unwrap(),
        CommitId::new([9; 32]).unwrap(),
        10,
    )
    .unwrap();
    let action = cinder_bulk::wire::Action::limit(
        &NativeMarket::new("BTC-USD", "1", "1", 0).unwrap(),
        true,
        "110",
        "2",
        cinder_bulk::wire::TimeInForce::Gtc,
    )
    .unwrap();
    let scope = Scope::new(
        config().units()[0].ledger.sources[0].scope,
        Network::Testnet,
        [41; 32],
    )
    .unwrap();
    let oid = action.order_id(&scope, 8).unwrap();
    let other = j.state().unwrap().unit(uid(50)).unwrap().clone();
    for n in [10, 11] {
        observation::ingest(
            &g,
            &mut j.select(uid(40)).unwrap(),
            Message {
                kind: Kind::TradeStream,
                body: stream(oid, 100),
                received_at: 10,
            },
            CommitId::new([n; 32]).unwrap(),
            10,
        )
        .unwrap();
    }
    let state = j.state().unwrap().unit(uid(40)).unwrap();
    let book = state.ledger().book(Owner::Customer(account())).unwrap();
    assert_eq!(book.cash().atoms(), 1999);
    assert_eq!(book.positions()[0].quantity().lots(), 1);
    assert_eq!(book.positions()[0].basis().atoms(), 100);
    assert_eq!(j.state().unwrap().unit(uid(50)).unwrap(), &other);
    observation::ingest(
        &g,
        &mut j.select(uid(40)).unwrap(),
        Message {
            kind: Kind::Funding,
            body: PrivateBytes::new(b"{}".to_vec()).unwrap(),
            received_at: 10,
        },
        CommitId::new([12; 32]).unwrap(),
        10,
    )
    .unwrap();
    assert!(j.state().unwrap().unit(uid(40)).unwrap().unresolved_raw() > 0);
    assert_eq!(j.state().unwrap().unit(uid(50)).unwrap(), &other);
}

#[test]
fn unsupported_qualification_wrong_conversion_and_master_key_never_activate() {
    let c = config().units()[0].ledger.clone();
    let mut p = policy();
    p.qualification = Level::Unknown;
    assert!(Gateway::new(c.clone(), profile(), p, Zeroizing::new([10; 32]), 1).is_err());
    let mut c2 = c.clone();
    c2.markets[0] = cinder_kernel::position::Market::new(c2.markets[0].unit(), 2, 1).unwrap();
    assert!(Gateway::new(c2, profile(), policy(), Zeroizing::new([10; 32]), 1).is_err());
    let master = SigningKey::from_bytes(&[10; 32]).verifying_key().to_bytes();
    let profile = financial::Profile::new(
        Scope::new(c.sources[0].scope, Network::Testnet, master).unwrap(),
        c.quote,
        0,
        Target::Master,
        vec![Mapping::new(
            NativeMarket::new("BTC-USD", "1", "1", 0).unwrap(),
            c.markets[0].unit(),
        )],
    )
    .unwrap();
    assert!(Gateway::new(c, profile, policy(), Zeroizing::new([10; 32]), 1).is_err());
}
