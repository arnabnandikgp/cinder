//! Offline fake transport and public test keys only. Never fund these identities.
#[path = "../../journal/tests/support/mod.rs"]
mod support;
use cinder_journal::{model::*, orders::*, *};
use cinder_kernel::{identity::*, ledger::*};
use cinder_pacifica::{
    client_id,
    execution::*,
    observation::{self, Kind, Message},
    profile::*,
};
use ed25519_dalek::{Signature, VerifyingKey};
use serde_json::{Value, json};
use std::collections::VecDeque;
use support::*;
use zeroize::Zeroizing;

fn profile() -> Profile {
    Profile {
        config: config(),
        source: config().sources[0].scope,
        account: bs58::encode([42; 32]).into_string(),
        environment: Origin::Testnet.url().into(),
        revision: 1,
        evidence: "synthetic signing fixture only".into(),
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
    }
}
fn policy() -> Policy {
    Policy {
        revision: 1,
        evidence: "offline test policy".into(),
        execution: Level::Qualified,
        origin: Origin::Testnet,
        expiry_ms: 3000,
        credits: 100,
        cleanup_reserve: 20,
        read_cost: 10,
    }
}
fn gateway() -> Gateway {
    Gateway::new(profile(), policy(), Zeroizing::new([7; 32]), 1).unwrap()
}
fn id(n: u8) -> CommitId {
    CommitId::new([n; 32]).unwrap()
}
fn seed<B: Backend, P: Protection>(s: &mut Journal<B, P>, g: &Gateway) {
    s.commit(transaction(
        s.head(),
        1,
        vec![receipt(1, Owner::Customer(user(1)), 100_000)],
        vec![Control::Order(Action::AdvanceAuthority {
            account: user(1),
            epoch: 1,
        })],
    ))
    .unwrap();
    g.activate(s, id(2), 10).unwrap();
}
fn prepare<B: Backend, P: Protection>(
    s: &mut Journal<B, P>,
    n: u8,
    side: i64,
    tif: TimeInForce,
    at: u64,
) {
    let intent = Intent {
        time_in_force: tif,
        request: request(n),
        quantity: q(side * 2),
        minimum: p(90),
        maximum: p(110),
        maximum_fee_per_lot: cash(1),
        reduce_only: false,
        policy: config().policy,
        authority_epoch: 1,
        expires_at: 100_000,
    };
    let approval = Approval {
        account: user(1),
        intent_hash: intent.digest().unwrap(),
        authority_epoch: 1,
    };
    let mut tx = transaction(
        s.head(),
        n,
        vec![],
        vec![
            Control::Order(Action::Accept {
                intent: Box::new(intent),
                approval,
                reservations: vec![
                    Reservation {
                        resource: Resource::Customer(user(1)),
                        amount: cash(10),
                    },
                    Reservation {
                        resource: Resource::Location(Location::Venue),
                        amount: cash(10),
                    },
                ],
            }),
            Control::Order(Action::Prepare {
                attempt: attempt(n),
            }),
        ],
    );
    tx.at = at;
    assert_eq!(s.commit(tx).unwrap().receipt.controls, None);
}
fn dispatch(n: u8, at: u64) -> Dispatch {
    Dispatch {
        attempt: attempt(n),
        commit: id(n + 100),
        at,
    }
}
fn response(status: u16, body: Value, at: u64) -> Reply {
    Reply::Response {
        status,
        body: PrivateBytes::new(body.to_string().into_bytes()).unwrap(),
        received_at: at,
        retry_after_ms: None,
    }
}
#[derive(Default)]
struct Fake {
    requests: Vec<Outbound>,
    replies: VecDeque<Reply>,
}
impl Transport for Fake {
    fn post(&mut self, r: Outbound) -> Reply {
        self.requests.push(r);
        self.replies.pop_front().unwrap_or(Reply::Unknown)
    }
}
fn parsed(r: &Outbound) -> Value {
    serde_json::from_slice(r.body()).unwrap()
}
fn signature_valid(r: &Outbound) -> bool {
    let mut body = parsed(r).as_object().unwrap().clone();
    let account = body.remove("account").unwrap();
    assert_eq!(account, profile().account);
    let agent = body.remove("agent_wallet").unwrap();
    let key: [u8; 32] = bs58::decode(agent.as_str().unwrap())
        .into_vec()
        .unwrap()
        .try_into()
        .unwrap();
    let sig = body.remove("signature").unwrap();
    let sig =
        Signature::from_slice(&bs58::decode(sig.as_str().unwrap()).into_vec().unwrap()).unwrap();
    let timestamp = body.remove("timestamp").unwrap();
    let expiry = body.remove("expiry_window").unwrap();
    let kind = if r.path().ends_with("/cancel") {
        "cancel_order"
    } else {
        "create_order"
    };
    let bytes = serde_json::to_vec(
        &json!({"data":body,"expiry_window":expiry,"timestamp":timestamp,"type":kind}),
    )
    .unwrap();
    VerifyingKey::from_bytes(&key)
        .unwrap()
        .verify_strict(&bytes, &sig)
        .is_ok()
}
#[test]
fn recovery_close_is_a_bounded_signed_ioc_not_an_ordinary_or_reusable_capability() {
    use cinder_journal::{collateral, liquidation as lc, recovery, risk};
    use cinder_kernel::ledger::evidence::*;
    let t = Temp::new();
    let mut s = t.create();
    let g = gateway();
    seed(&mut s, &g);
    let mut tx = transaction(
        s.head(),
        3,
        vec![
            receipt(2, Owner::House, 10000),
            Event {
                key: RecordKey::Attempt(attempt(1)),
                policy: config().policy,
                change: Change::BindExecution {
                    market: q(0).unit(),
                    side: Side::Buy,
                },
            },
            event(
                4,
                Change::Fill {
                    target: FillTarget::Customer(attempt(1)),
                    quantity: q(2),
                    price: p(100),
                },
            ),
        ],
        vec![],
    );
    assert_eq!(s.commit(tx.clone()).unwrap().receipt.controls, None);
    let l = s.state().unwrap().ledger();
    tx = transaction(
        s.head(),
        4,
        vec![event(
            5,
            Change::Reconcile(NativeCheck {
                expected_version: l.version(),
                cash: Some(l.venue().cash()),
                funding: Some(l.venue().funding()),
                positions: Some(l.venue().positions().to_vec()),
                complete: true,
                resolves: vec![],
            }),
        )],
        vec![],
    );
    s.commit(tx).unwrap();
    let version = s.state().unwrap().ledger().version();
    let cut = collateral::Cut {
        expected_version: version,
        policy: collateral::Policy {
            revision: config().policy,
            markets: vec![collateral::MarginRule {
                market: q(0).unit(),
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
            price: p(100),
            evidence: key(99),
            observed_at: 10,
            valid_until: 1000,
            qualified: true,
        }],
    };
    let r = risk::Policy {
        revision: config().policy,
        markets: vec![risk::MarketRule {
            market: q(0).unit(),
            maximum_leverage: 100000,
            maintenance_bps: 500,
            native_maintenance_bps: 500,
            gross_limit: cash(100000),
            net_limit: cash(100000),
        }],
        buffer: cash(0),
        horizon_ms: 50,
        valid_until: 1000,
        paths: vec![risk::Path {
            id: [1; 32],
            steps: vec![risk::Step {
                after_ms: 1,
                marks: vec![p(100)],
                events: vec![],
                liquidity: vec![
                    risk::Liquidity {
                        location: Location::Vault,
                        accessible_bps: 10000,
                        due: cash(0),
                    },
                    risk::Liquidity {
                        location: Location::Venue,
                        accessible_bps: 10000,
                        due: cash(0),
                    },
                ],
            }],
        }],
    };
    let tx = transaction(
        s.head(),
        5,
        vec![],
        vec![
            Control::Collateral(cut),
            Control::Risk(risk::Action::Install {
                expected_version: version,
                policy: Box::new(r),
            }),
            Control::Liquidation(lc::Action::Install {
                expected_version: version,
                policy: Box::new(lc::Policy {
                    revision: config().policy,
                    authority_epoch: 1,
                    valid_until: 1000,
                    limits: vec![lc::Limit {
                        market: q(0).unit(),
                        maximum_lots: 2,
                        minimum: p(90),
                        maximum: p(110),
                        fee_per_lot: cash(1),
                        additional_per_lot: cash(5),
                    }],
                }),
            }),
            Control::Recovery(recovery::Action::Begin {
                expected_version: version,
                authority_epoch: 1,
                valid_until: 1000,
                fence: [41; 32],
            }),
        ],
    );
    assert_eq!(s.commit(tx).unwrap().receipt.controls, None);
    let p = lc::Proposal {
        attempt: attempt(20),
        kind: lc::Kind::RecoveryClose,
        market: q(0).unit(),
        expected_version: version,
        policy: config().policy,
        authority_epoch: 1,
        expires_at: 1000,
    };
    let tx = transaction(
        s.head(),
        6,
        vec![],
        vec![Control::Liquidation(lc::Action::Prepare {
            authenticated_digest: p.digest().unwrap(),
            proposal: Box::new(p),
        })],
    );
    assert_eq!(s.commit(tx).unwrap().receipt.controls, None);
    let mut fake = Fake::default();
    assert!(g.dispatch(&mut s, dispatch(20, 11), &mut fake).is_err());
    assert!(fake.requests.is_empty());
    assert_eq!(
        g.dispatch_recovery(&mut s, dispatch(20, 11), &mut fake)
            .unwrap(),
        Outcome::Unknown
    );
    assert_eq!(fake.requests.len(), 1);
    assert!(signature_valid(&fake.requests[0]));
    let body = parsed(&fake.requests[0]);
    assert_eq!(body["tif"], "IOC");
    assert_eq!(body["side"], "ask");
    assert_eq!(body["amount"], "2");
    assert_eq!(body["price"], "90");
    assert_eq!(body["reduce_only"], false);
    assert!(s.state().unwrap().frozen());
    assert!(s.state().unwrap().holds().iter().any(|h| h.active));
    assert_eq!(
        s.state()
            .unwrap()
            .ledger()
            .book(Owner::Customer(user(1)))
            .unwrap()
            .positions()[0]
            .quantity()
            .lots(),
        2
    );
    assert!(
        g.dispatch_recovery(&mut s, dispatch(20, 12), &mut fake)
            .is_err()
    );
    assert_eq!(fake.requests.len(), 1);
    let expected = s.state().unwrap().clone();
    drop(s);
    assert_eq!(t.open().state().unwrap(), &expected);
}
#[test]
fn gtc_alo_ioc_both_sides_are_real_signatures_over_exact_native_fields() {
    for tif in [
        TimeInForce::GoodTilCancelled,
        TimeInForce::AddLiquidityOnly,
        TimeInForce::ImmediateOrCancel,
    ] {
        for side in [-1, 1] {
            let t = Temp::new();
            let mut s = t.create();
            let g = gateway();
            seed(&mut s, &g);
            prepare(&mut s, 3, side, tif, 10);
            let mut fake = Fake::default();
            fake.replies
                .push_back(response(200, json!({"order_id":9007199254740993_u64}), 21));
            assert_eq!(
                g.dispatch(&mut s, dispatch(3, 20), &mut fake).unwrap(),
                Outcome::Acknowledged
            );
            let r = &fake.requests[0];
            assert!(signature_valid(r));
            assert_eq!(r.origin(), Origin::Testnet);
            let body = parsed(r);
            assert_eq!(body["price"], if side > 0 { "110" } else { "90" });
            assert_eq!(body["amount"], "2");
            assert_eq!(body["reduce_only"], false);
            assert_eq!(body["client_order_id"], client_id(attempt(3)));
            assert_eq!(body["expiry_window"], 3000);
            assert_eq!(
                body["tif"],
                match tif {
                    TimeInForce::GoodTilCancelled => "GTC",
                    TimeInForce::AddLiquidityOnly => "ALO",
                    TimeInForce::ImmediateOrCancel => "IOC",
                }
            );
            assert!(s.state().unwrap().orders()[0].acknowledged);
            assert!(!s.state().unwrap().orders()[0].complete());
            let tx = s.transaction(id(103)).unwrap();
            assert_eq!(tx.controls.len(), 1);
            assert!(
                tx.evidence[0]
                    .as_bytes()
                    .windows(8)
                    .any(|w| w == b"preimage")
            );
            assert!(!format!("{:?}", r).contains("signature"));
            assert!(!format!("{:?}", g).contains(&g.agent()));
        }
    }
}
#[test]
fn unknown_ack_never_resigns_or_releases_after_reopen() {
    let t = Temp::new();
    let mut s = t.create();
    let g = gateway();
    seed(&mut s, &g);
    prepare(&mut s, 3, 1, TimeInForce::ImmediateOrCancel, 10);
    let mut fake = Fake::default();
    assert_eq!(
        g.dispatch(&mut s, dispatch(3, 20), &mut fake).unwrap(),
        Outcome::Unknown
    );
    assert_eq!(
        s.state()
            .unwrap()
            .reserved(Resource::Customer(user(1)))
            .unwrap(),
        cash(10)
    );
    assert!(g.dispatch(&mut s, dispatch(3, 20), &mut fake).is_err());
    drop(s);
    let mut s = t.open();
    let retry = Dispatch {
        commit: id(200),
        ..dispatch(3, 22)
    };
    assert!(g.dispatch(&mut s, retry, &mut fake).is_err());
    assert_eq!(fake.requests.len(), 1);
    assert!(s.state().unwrap().orders()[0].unknown);
}
#[test]
fn scoped_cancel_and_late_fill_preserve_customer_ownership_and_hold() {
    let t = Temp::new();
    let mut s = t.create();
    let g = gateway();
    seed(&mut s, &g);
    prepare(&mut s, 3, 1, TimeInForce::GoodTilCancelled, 10);
    let mut fake = Fake::default();
    g.dispatch(&mut s, dispatch(3, 20), &mut fake).unwrap();
    let cancel = AttemptKey {
        request: request(3),
        attempt: AttemptId::new([99; 32]).unwrap(),
    };
    let mut tx = transaction(
        s.head(),
        4,
        vec![],
        vec![Control::Order(Action::PrepareCancel {
            attempt: cancel,
            authority_epoch: 1,
            expires_at: 1000,
        })],
    );
    tx.at = 21;
    assert_eq!(s.commit(tx).unwrap().receipt.controls, None);
    fake.replies
        .push_back(response(200, json!({"success":true}), 23));
    assert_eq!(
        g.dispatch(
            &mut s,
            Dispatch {
                attempt: cancel,
                commit: id(104),
                at: 22
            },
            &mut fake
        )
        .unwrap(),
        Outcome::CancelAcknowledged
    );
    let cancel_body = parsed(&fake.requests[1]);
    assert!(signature_valid(&fake.requests[1]));
    assert_eq!(cancel_body["client_order_id"], client_id(attempt(3)));
    assert!(cancel_body.get("order_id").is_none());
    assert!(cancel_body.get("amount").is_none());
    let message=Message{kind:Kind::Trades,account:profile().account,cursor:None,received_at:25,body:json!({"success":true,"has_more":false,"data":[{"history_id":1,"order_id":8,"client_order_id":client_id(attempt(3)),"symbol":"BTC","amount":"2","price":"100","entry_price":"100","fee":"1","pnl":"-1","event_type":"fulfill_taker","side":"open_long","cause":"normal","created_at":21}]}).to_string()};
    observation::ingest(&mut s, &profile(), message, id(5), 25).unwrap();
    assert_eq!(
        s.state()
            .unwrap()
            .ledger()
            .book(Owner::Customer(user(1)))
            .unwrap()
            .positions()[0]
            .quantity(),
        q(2)
    );
    assert!(s.state().unwrap().orders()[0].cancel_acknowledged);
    assert!(!s.state().unwrap().orders()[0].complete());
    assert_eq!(
        s.state()
            .unwrap()
            .reserved(Resource::Customer(user(1)))
            .unwrap(),
        cash(10)
    );
}
#[test]
fn api_credit_window_shared_with_reads_rotation_and_cleanup_reserve() {
    let t = Temp::new();
    let mut s = t.create();
    let mut p = policy();
    p.credits = 40;
    let g = Gateway::new(profile(), p.clone(), Zeroizing::new([7; 32]), 1).unwrap();
    seed(&mut s, &g);
    prepare(&mut s, 3, 1, TimeInForce::GoodTilCancelled, 10);
    prepare(&mut s, 4, 1, TimeInForce::GoodTilCancelled, 10);
    let mut fake = Fake::default();
    g.dispatch(&mut s, dispatch(3, 20), &mut fake).unwrap();
    assert_eq!(
        g.reserve_read(&mut s, id(10), 21, false)
            .unwrap()
            .consume(21)
            .unwrap(),
        (21, 10)
    );
    assert!(g.dispatch(&mut s, dispatch(4, 22), &mut fake).is_err());
    assert!(g.reserve_read(&mut s, id(11), 22, false).is_err());
    let rotated = Gateway::new(profile(), p, Zeroizing::new([8; 32]), 2).unwrap();
    rotated.activate(&mut s, id(12), 22).unwrap();
    assert!(g.reserve_read(&mut s, id(13), 23, true).is_err());
    assert!(
        rotated
            .dispatch(&mut s, dispatch(4, 23), &mut fake)
            .is_err()
    );
    rotated
        .reserve_read(&mut s, id(14), 23, true)
        .unwrap()
        .consume(23)
        .unwrap();
    assert!(rotated.reserve_read(&mut s, id(14), 23, true).is_err());
    drop(s);
    let mut s = t.open();
    assert!(
        rotated
            .dispatch(&mut s, dispatch(4, 24), &mut fake)
            .is_err()
    );
    rotated
        .dispatch(&mut s, dispatch(4, 63_023), &mut fake)
        .unwrap();
    assert_eq!(fake.requests.len(), 2);
}
#[test]
fn rate_limit_and_error_responses_are_unknown_not_retry_permission() {
    for status in [400, 401, 429, 500] {
        let t = Temp::new();
        let mut s = t.create();
        let g = gateway();
        seed(&mut s, &g);
        prepare(&mut s, 3, 1, TimeInForce::GoodTilCancelled, 10);
        prepare(&mut s, 4, 1, TimeInForce::GoodTilCancelled, 10);
        let mut fake = Fake::default();
        fake.replies.push_back(response(
            status,
            json!({"error":"bounded synthetic response"}),
            21,
        ));
        assert_eq!(
            g.dispatch(&mut s, dispatch(3, 20), &mut fake).unwrap(),
            Outcome::Unknown
        );
        assert!(
            g.dispatch(
                &mut s,
                Dispatch {
                    commit: id(150),
                    ..dispatch(3, 22)
                },
                &mut fake
            )
            .is_err()
        );
        if status == 429 {
            assert!(g.dispatch(&mut s, dispatch(4, 22), &mut fake).is_err());
            drop(s);
            let mut s = t.open();
            assert!(g.reserve_read(&mut s, id(15), 60_020, true).is_err());
            g.reserve_read(&mut s, id(15), 60_021, true)
                .unwrap()
                .consume(60_021)
                .unwrap();
        }
        assert_eq!(fake.requests.len(), 1);
    }
}

#[test]
fn read_429_is_scoped_idempotent_shared_and_survives_rotation_and_reopen() {
    for retry_after in [None, Some(1), Some(u64::MAX)] {
        let t = Temp::new();
        let mut s = t.create();
        let g = gateway();
        seed(&mut s, &g);
        prepare(&mut s, 3, 1, TimeInForce::GoodTilCancelled, 10);
        let before = s.head();
        assert!(
            g.record_read_limit(&mut s, id(99), 20, retry_after)
                .is_err()
        );
        assert!(g.record_read_limit(&mut s, id(2), 20, retry_after).is_err());
        assert_eq!(s.head(), before);
        g.reserve_read(&mut s, id(10), 20, false)
            .unwrap()
            .consume(20)
            .unwrap();
        let rotated = Gateway::new(profile(), policy(), Zeroizing::new([8; 32]), 2).unwrap();
        rotated.activate(&mut s, id(11), 22).unwrap();
        // The previous key's in-flight read still reports a shared limit. A clock
        // regression cannot regress the journal or drop the observed cooldown.
        g.record_read_limit(&mut s, id(10), 19, retry_after)
            .unwrap();
        let head = s.head();
        g.record_read_limit(&mut s, id(10), 19, retry_after)
            .unwrap();
        assert_eq!(s.head(), head);
        assert!(
            g.record_read_limit(&mut s, id(10), 20, retry_after)
                .is_err()
        );
        assert_eq!(s.head(), head);
        assert!(rotated.reserve_read(&mut s, id(12), 23, false).is_err());
        assert!(rotated.reserve_read(&mut s, id(12), 23, true).is_err());
        let mut fake = Fake::default();
        assert!(
            rotated
                .dispatch(&mut s, dispatch(3, 23), &mut fake)
                .is_err()
        );
        assert!(fake.requests.is_empty());
        drop(s);
        let mut s = t.open();
        let until = 22 + retry_after.unwrap_or(60_000).clamp(60_000, 3_600_000);
        assert!(
            rotated
                .reserve_read(&mut s, id(12), until - 1, true)
                .is_err()
        );
        rotated
            .reserve_read(&mut s, id(12), until, true)
            .unwrap()
            .consume(until)
            .unwrap();
        let head = s.head();
        // A late duplicate cannot restart its cooldown, even after other commits.
        g.record_read_limit(&mut s, id(10), 19, retry_after)
            .unwrap();
        assert_eq!(s.head(), head);
        rotated.reserve_read(&mut s, id(13), until, true).unwrap();
    }
}

#[test]
fn rejected_post_send_replies_remain_unknown_with_status_and_cooldown() {
    for status in [200, 429] {
        for (oversized, early) in [(true, false), (false, true), (true, true)] {
            let t = Temp::new();
            let mut s = t.create();
            let g = gateway();
            seed(&mut s, &g);
            prepare(&mut s, 3, 1, TimeInForce::GoodTilCancelled, 10);
            prepare(&mut s, 4, 1, TimeInForce::GoodTilCancelled, 10);
            let mut body = br#"{"order_id":1}"#.to_vec();
            if oversized {
                body.resize(observation::MAX_BODY + 1, b' ');
            }
            let mut fake = Fake::default();
            fake.replies.push_back(Reply::Response {
                status,
                body: PrivateBytes::new(body).unwrap(),
                received_at: if early { 19 } else { 21 },
                retry_after_ms: Some(1),
            });
            assert_eq!(
                g.dispatch(&mut s, dispatch(3, 20), &mut fake).unwrap(),
                Outcome::Unknown
            );
            let tx = s.transactions().last().unwrap();
            let at = if early { 20 } else { 21 };
            assert_eq!(tx.at, at);
            assert_eq!(tx.order_observations[0].status, Status::Unknown);
            assert_eq!(
                tx.order_observations[0].raw.as_bytes(),
                b"response rejected: bound or clock"
            );
            let provenance: Value = serde_json::from_slice(
                tx.evidence[0]
                    .as_bytes()
                    .strip_prefix(b"CINDER-PACIFICA-EXECUTION-1\0")
                    .unwrap(),
            )
            .unwrap();
            assert_eq!(provenance["record"]["Response"]["status"], status);
            assert_eq!(tx.evidence.len(), if status == 429 { 2 } else { 1 });
            assert!(s.state().unwrap().orders()[0].unknown);
            assert!(!s.state().unwrap().orders()[0].acknowledged);
            assert_eq!(
                s.state()
                    .unwrap()
                    .reserved(Resource::Customer(user(1)))
                    .unwrap(),
                cash(20)
            );
            assert_eq!(
                s.state().unwrap().ledger().venue().positions()[0].quantity(),
                q(0)
            );
            drop(s);
            let mut s = t.open();
            assert!(
                g.dispatch(
                    &mut s,
                    Dispatch {
                        commit: id(150),
                        ..dispatch(3, 22)
                    },
                    &mut fake
                )
                .is_err()
            );
            if status == 429 {
                assert!(g.dispatch(&mut s, dispatch(4, 22), &mut fake).is_err());
                assert!(g.reserve_read(&mut s, id(15), at + 59_999, true).is_err());
                g.reserve_read(&mut s, id(15), at + 60_000, true).unwrap();
            }
            assert_eq!(fake.requests.len(), 1);
        }
    }
}
#[test]
fn durable_key_revocation_and_private_grant_revocation_prevent_dispatch() {
    let t = Temp::new();
    let mut s = t.create();
    let g = gateway();
    seed(&mut s, &g);
    prepare(&mut s, 3, 1, TimeInForce::GoodTilCancelled, 10);
    g.deactivate(&mut s, id(4), 11).unwrap();
    drop(s);
    let mut s = t.open();
    let mut fake = Fake::default();
    assert!(g.dispatch(&mut s, dispatch(3, 20), &mut fake).is_err());
    assert!(g.activate(&mut s, id(5), 20).is_err());
    let new = Gateway::new(profile(), policy(), Zeroizing::new([8; 32]), 3).unwrap();
    new.activate(&mut s, id(6), 20).unwrap();
    let mut tx = transaction(
        s.head(),
        7,
        vec![],
        vec![Control::Order(Action::AdvanceAuthority {
            account: user(1),
            epoch: 2,
        })],
    );
    tx.at = 20;
    s.commit(tx).unwrap();
    assert!(new.dispatch(&mut s, dispatch(3, 21), &mut fake).is_err());
    assert!(fake.requests.is_empty());
    assert!(!s.state().unwrap().attempts()[0].possibly_exposed);
}
#[test]
fn policy_scope_expiry_and_unqualified_profiles_reject_before_signing() {
    let mut p = profile();
    p.environment = Origin::Mainnet.url().into();
    assert!(Gateway::new(p, policy(), Zeroizing::new([7; 32]), 1).is_err());
    let mut p = profile();
    p.fills = Level::Observed;
    assert!(Gateway::new(p, policy(), Zeroizing::new([7; 32]), 1).is_err());
    let t = Temp::new();
    let mut s = t.create();
    let g = gateway();
    seed(&mut s, &g);
    prepare(&mut s, 3, 1, TimeInForce::GoodTilCancelled, 10);
    let mut fake = Fake::default();
    assert!(g.dispatch(&mut s, dispatch(3, 100_000), &mut fake).is_err());
    let mut changed = policy();
    changed.credits += 10;
    let changed = Gateway::new(profile(), changed, Zeroizing::new([7; 32]), 1).unwrap();
    assert!(
        changed
            .dispatch(&mut s, dispatch(3, 20), &mut fake)
            .is_err()
    );
    assert!(fake.requests.is_empty());
}
#[test]
fn malformed_success_and_delayed_ack_do_not_create_fills_or_release() {
    for body in [
        json!({}),
        json!({"success":false,"order_id":1}),
        json!({"order_id":1,"data":{"order_id":2}}),
        json!({"order_id":1.2}),
    ] {
        let t = Temp::new();
        let mut s = t.create();
        let g = gateway();
        seed(&mut s, &g);
        prepare(&mut s, 3, 1, TimeInForce::GoodTilCancelled, 10);
        let mut fake = Fake::default();
        fake.replies.push_back(response(200, body, 5000));
        assert_eq!(
            g.dispatch(&mut s, dispatch(3, 20), &mut fake).unwrap(),
            Outcome::Unknown
        );
        assert_eq!(
            s.state().unwrap().ledger().venue().positions()[0].quantity(),
            q(0)
        );
        assert_eq!(
            s.state()
                .unwrap()
                .reserved(Resource::Customer(user(1)))
                .unwrap(),
            cash(10)
        );
    }
}
#[test]
fn native_signature_omits_account_but_gateway_binds_it_and_has_no_money_methods() {
    let t = Temp::new();
    let mut s = t.create();
    let g = gateway();
    seed(&mut s, &g);
    prepare(&mut s, 3, 1, TimeInForce::GoodTilCancelled, 10);
    let mut f = Fake::default();
    g.dispatch(&mut s, dispatch(3, 20), &mut f).unwrap();
    let body = parsed(&f.requests[0]);
    assert_eq!(body["account"], profile().account);
    assert_ne!(body["agent_wallet"], body["account"]);
    assert!(signature_valid(&f.requests[0]));
    // Native account is outside signed data; this test explicitly does NOT claim
    // cryptographic account/network binding from Pacifica's signature itself.
    let original = body["signature"].clone();
    let mut forged = body;
    forged["account"] = json!(bs58::encode([43; 32]).into_string());
    assert_eq!(forged["signature"], original);
    let mut other = profile();
    other.account = bs58::encode([43; 32]).into_string();
    let other = Gateway::new(other, policy(), Zeroizing::new([7; 32]), 1).unwrap();
    assert!(other.reserve_read(&mut s, id(9), 21, false).is_err());
    assert_eq!(f.requests[0].path(), "/api/v1/orders/create");
}

struct FaultBackend {
    inner: cinder_journal::sqlite::SqliteBackend,
    fault: std::rc::Rc<std::cell::Cell<u8>>,
}
impl Backend for FaultBackend {
    fn load(&mut self) -> Result<Vec<Frame>, Error> {
        self.inner.load()
    }
    fn append(&mut self, expected: Option<Head>, frame: &Frame) -> Result<(), Error> {
        if self.fault.get() == 1 {
            self.fault.set(0);
            return Err(Error::Storage);
        }
        self.inner.append(expected, frame)?;
        if self.fault.get() == 2 {
            self.fault.set(0);
            return Err(Error::Storage);
        }
        if self.fault.get() == 3 {
            self.fault.set(4);
        }
        Ok(())
    }
    fn check_current(&mut self, _: Head) -> Result<(), Error> {
        if self.fault.get() == 4 {
            Err(Error::Stale)
        } else {
            Ok(())
        }
    }
}
#[test]
fn crash_before_after_commit_and_fencing_before_signature_never_send() {
    use cinder_journal::sqlite::{Migration, SqliteBackend};
    use std::{cell::Cell, rc::Rc};
    for failure in [1, 2, 3] {
        let t = Temp::new();
        let fault = Rc::new(Cell::new(0));
        let b = FaultBackend {
            inner: SqliteBackend::create(&t.db).unwrap(),
            fault: fault.clone(),
        };
        let mut s = Journal::create(b, FixtureProtection, config()).unwrap();
        let g = gateway();
        seed(&mut s, &g);
        prepare(&mut s, 3, 1, TimeInForce::GoodTilCancelled, 10);
        fault.set(failure);
        let mut fake = Fake::default();
        assert!(g.dispatch(&mut s, dispatch(3, 20), &mut fake).is_err());
        assert!(fake.requests.is_empty());
        assert!(s.state().is_err());
        drop(s);
        fault.set(0);
        let mut s = Journal::open(
            FaultBackend {
                inner: SqliteBackend::open(&t.db, Migration::None).unwrap(),
                fault,
            },
            FixtureProtection,
            config(),
        )
        .unwrap();
        assert_eq!(
            s.state().unwrap().attempts()[0].possibly_exposed,
            failure != 1
        );
        if failure != 1 {
            assert!(
                g.dispatch(
                    &mut s,
                    Dispatch {
                        commit: id(201),
                        ..dispatch(3, 21)
                    },
                    &mut fake
                )
                .is_err()
            );
            assert!(fake.requests.is_empty());
        } else {
            g.dispatch(&mut s, dispatch(3, 21), &mut fake).unwrap();
            assert_eq!(fake.requests.len(), 1);
        }
    }
}

#[test]
fn lost_response_commit_leaves_signed_attempt_reserved_and_never_retried() {
    use cinder_journal::sqlite::{Migration, SqliteBackend};
    use std::{cell::Cell, rc::Rc};
    struct LoseReply {
        fault: Rc<Cell<u8>>,
        sent: usize,
    }
    impl Transport for LoseReply {
        fn post(&mut self, _: Outbound) -> Reply {
            self.sent += 1;
            self.fault.set(2);
            response(200, json!({"order_id":9}), 21)
        }
    }
    let t = Temp::new();
    let fault = Rc::new(Cell::new(0));
    let mut s = Journal::create(
        FaultBackend {
            inner: SqliteBackend::create(&t.db).unwrap(),
            fault: fault.clone(),
        },
        FixtureProtection,
        config(),
    )
    .unwrap();
    let g = gateway();
    seed(&mut s, &g);
    prepare(&mut s, 3, 1, TimeInForce::GoodTilCancelled, 10);
    let mut transport = LoseReply {
        fault: fault.clone(),
        sent: 0,
    };
    assert!(g.dispatch(&mut s, dispatch(3, 20), &mut transport).is_err());
    assert_eq!(transport.sent, 1);
    drop(s);
    let mut s = Journal::open(
        FaultBackend {
            inner: SqliteBackend::open(&t.db, Migration::None).unwrap(),
            fault,
        },
        FixtureProtection,
        config(),
    )
    .unwrap();
    assert!(s.state().unwrap().orders()[0].acknowledged);
    assert_eq!(
        s.state()
            .unwrap()
            .reserved(Resource::Customer(user(1)))
            .unwrap(),
        cash(10)
    );
    assert!(
        g.dispatch(
            &mut s,
            Dispatch {
                commit: id(202),
                ..dispatch(3, 22)
            },
            &mut transport
        )
        .is_err()
    );
    assert_eq!(transport.sent, 1);
}

#[test]
fn cancel_can_use_reserved_credits_when_new_orders_cannot() {
    let t = Temp::new();
    let mut s = t.create();
    let mut p = policy();
    p.credits = 30;
    p.cleanup_reserve = 20;
    let g = Gateway::new(profile(), p, Zeroizing::new([7; 32]), 1).unwrap();
    seed(&mut s, &g);
    prepare(&mut s, 3, 1, TimeInForce::GoodTilCancelled, 10);
    let mut f = Fake::default();
    g.dispatch(&mut s, dispatch(3, 20), &mut f).unwrap();
    assert!(g.reserve_read(&mut s, id(4), 21, false).is_err());
    let cancel = AttemptKey {
        request: request(3),
        attempt: AttemptId::new([99; 32]).unwrap(),
    };
    let mut tx = transaction(
        s.head(),
        5,
        vec![],
        vec![Control::Order(Action::PrepareCancel {
            attempt: cancel,
            authority_epoch: 1,
            expires_at: 1000,
        })],
    );
    tx.at = 21;
    assert_eq!(s.commit(tx).unwrap().receipt.controls, None);
    g.dispatch(
        &mut s,
        Dispatch {
            attempt: cancel,
            commit: id(105),
            at: 22,
        },
        &mut f,
    )
    .unwrap();
    assert_eq!(f.requests.len(), 2);
    assert_eq!(f.requests[1].path(), "/api/v1/orders/cancel");
}

#[test]
fn read_permit_expiry_and_pending_delivery_credit_tail_are_bounded() {
    let t = Temp::new();
    let mut s = t.create();
    let mut p = policy();
    p.credits = 30;
    p.cleanup_reserve = 20;
    let g = Gateway::new(profile(), p, Zeroizing::new([7; 32]), 1).unwrap();
    seed(&mut s, &g);
    let permit = g.reserve_read(&mut s, id(3), 20, false).unwrap();
    assert!(permit.consume(3020).is_err());
    // No receipt/refund: reserve remains until the full 60s window after the
    // latest admissible send, not merely 60s after pre-dispatch persistence.
    assert!(g.reserve_read(&mut s, id(4), 60_020, false).is_err());
    g.reserve_read(&mut s, id(4), 63_020, false)
        .unwrap()
        .consume(63_020)
        .unwrap();
}
