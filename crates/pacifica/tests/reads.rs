//! Offline read-port qualification. No sockets, wallets or live venue evidence.
#[path = "../../journal/tests/support/mod.rs"]
mod support;
use cinder_journal::model::*;
use cinder_pacifica::{
    execution::{Gateway, Origin, Policy, Reply},
    observation::{self, Gap, Kind},
    profile::{Grid, Level, Mapping, Profile},
    reads::{self, MIN_READ_COST, Outcome, Poll, Request, Transport},
};
use serde_json::{Value, json};
use support::*;
use zeroize::Zeroizing;

fn profile() -> Profile {
    Profile {
        config: config(),
        source: config().sources[0].scope,
        account: bs58::encode([42; 32]).into_string(),
        environment: Origin::Testnet.url().into(),
        revision: 1,
        evidence: "offline read fixture only".into(),
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
        evidence: "offline read fixture only".into(),
        execution: Level::Qualified,
        origin: Origin::Testnet,
        expiry_ms: 1000,
        credits: 600,
        cleanup_reserve: 120,
        read_cost: MIN_READ_COST,
    }
}
fn gateway() -> Gateway {
    Gateway::new(profile(), policy(), Zeroizing::new([7; 32]), 1).unwrap()
}
fn activated(temp: &Temp) -> Store {
    let mut store = temp.create();
    gateway().activate(&mut store, id(250), 0).unwrap();
    store
}
fn id(n: u8) -> CommitId {
    CommitId::new([n; 32]).unwrap()
}
fn input(n: u8, kind: Kind, cursor: Option<&str>) -> Poll {
    Poll {
        kind,
        cursor: cursor.map(str::to_owned),
        reservation: id(n),
        evidence: id(n + 100),
        at: 100,
        cleanup: false,
    }
}
fn page(data: Value) -> Value {
    json!({"success":true,"data":data,"has_more":false})
}
fn response(status: u16, value: Value) -> Reply {
    Reply::Response {
        status,
        body: PrivateBytes::new(value.to_string().into_bytes()).unwrap(),
        received_at: 100,
        retry_after_ms: None,
    }
}
struct Fake {
    now: u64,
    reply: Option<Reply>,
    targets: Vec<String>,
}
impl Fake {
    fn new(reply: Reply) -> Self {
        Self {
            now: 100,
            reply: Some(reply),
            targets: vec![],
        }
    }
}
impl Transport for Fake {
    fn get(&mut self, request: Request) -> Reply {
        assert_eq!(format!("{request:?}"), "ReadRequest([PRIVATE])");
        let Ok(query) = request.consume(self.now) else {
            return Reply::Unknown;
        };
        assert_eq!(format!("{query:?}"), "ReadQuery([PRIVATE])");
        assert_eq!(query.origin(), Origin::Testnet);
        self.targets.push(query.target().to_owned());
        self.reply.take().unwrap_or(Reply::Unknown)
    }
}

#[test]
fn requests_use_only_bound_account_routes_and_do_not_turn_views_into_assets() {
    for (kind, route) in [
        (Kind::Trades, "/api/v1/trades/history"),
        (Kind::Funding, "/api/v1/funding/history"),
        (Kind::Orders, "/api/v1/orders/history"),
        (Kind::Positions, "/api/v1/positions"),
        (Kind::Account, "/api/v1/account"),
    ] {
        let temp = Temp::new();
        let mut store = activated(&temp);
        let ledger = store.state().unwrap().ledger().clone();
        let data = if kind == Kind::Account {
            json!({"balance":"900","account_equity":"1000","available_to_spend":"800",
                "available_to_withdraw":"700","pending_balance":"50","updated_at":99})
        } else {
            json!([])
        };
        let mut body = page(data);
        if kind == Kind::Positions {
            body["last_order_id"] = json!(9007199254741001_u64);
        }
        let mut transport = Fake::new(response(200, body));
        assert!(matches!(
            reads::poll(&mut store, &gateway(), input(1, kind, None), &mut transport).unwrap(),
            Outcome::Ingested(_)
        ));
        let suffix = if matches!(kind, Kind::Trades | Kind::Funding | Kind::Orders) {
            "&limit=32"
        } else {
            ""
        };
        assert_eq!(
            transport.targets,
            [format!("{route}?account={}{suffix}", profile().account)]
        );
        let view = observation::replay(&mut store, &profile()).unwrap();
        assert!(!view.complete_history());
        assert_eq!(store.state().unwrap().ledger(), &ledger);
        if kind == Kind::Funding {
            assert!(view.gaps.contains(&Gap::FundingQualification));
        }
        if kind == Kind::Account {
            assert_eq!(view.account.unwrap().balance.atoms(), 900);
        }
        drop(store);
        assert!(
            !observation::replay(&mut temp.open(), &profile())
                .unwrap()
                .complete_history()
        );
    }
}

#[test]
fn opaque_cursor_is_escaped_and_only_committed_progress_can_schedule_it() {
    let temp = Temp::new();
    let mut store = activated(&temp);
    let cursor = "a&account=evil?x=%/#+";
    let mut transport = Fake::new(response(
        200,
        json!({"success":true,"data":[],"has_more":true,"next_cursor":cursor}),
    ));
    reads::poll(
        &mut store,
        &gateway(),
        input(1, Kind::Trades, None),
        &mut transport,
    )
    .unwrap();
    let head = store.head();
    assert!(
        reads::poll(
            &mut store,
            &gateway(),
            input(2, Kind::Trades, Some("skipped")),
            &mut transport
        )
        .is_err()
    );
    assert_eq!(store.head(), head);
    drop(store);
    let mut store = temp.open();
    transport.reply = Some(response(200, page(json!([]))));
    reads::poll(
        &mut store,
        &gateway(),
        input(2, Kind::Trades, Some(cursor)),
        &mut transport,
    )
    .unwrap();
    assert!(transport.targets[1].ends_with("&cursor=a%26account%3Devil%3Fx%3D%25%2F%23%2B"));
    assert_eq!(transport.targets[1].matches("&account=").count(), 0);
    let head = store.head();
    assert!(
        reads::poll(
            &mut store,
            &gateway(),
            input(3, Kind::Trades, Some(cursor)),
            &mut transport
        )
        .is_err()
    );
    assert_eq!(store.head(), head);
}

#[test]
fn invalid_routes_cursors_ids_and_underpriced_reads_never_spend_or_expose() {
    let temp = Temp::new();
    let mut store = activated(&temp);
    let mut transport = Fake::new(Reply::Unknown);
    let head = store.head();
    for (kind, cursor) in [
        (Kind::TradeStream, None),
        (Kind::PositionStream, None),
        (Kind::Disconnect, None),
        (Kind::Account, Some("x")),
        (Kind::Positions, Some("x")),
        (Kind::Trades, Some("")),
        (Kind::Trades, Some("\r\n")),
        (Kind::Trades, Some("é")),
    ] {
        assert!(
            reads::poll(
                &mut store,
                &gateway(),
                input(1, kind, cursor),
                &mut transport
            )
            .is_err()
        );
        assert_eq!(store.head(), head);
    }
    let long = "x".repeat(257);
    assert!(
        reads::poll(
            &mut store,
            &gateway(),
            input(1, Kind::Trades, Some(&long)),
            &mut transport
        )
        .is_err()
    );
    let mut same = input(1, Kind::Trades, None);
    same.evidence = same.reservation;
    assert!(reads::poll(&mut store, &gateway(), same, &mut transport).is_err());
    let low = Policy {
        read_cost: MIN_READ_COST - 1,
        ..policy()
    };
    let low = Gateway::new(profile(), low, Zeroizing::new([7; 32]), 1).unwrap();
    assert!(
        reads::poll(
            &mut store,
            &low,
            input(1, Kind::Trades, None),
            &mut transport
        )
        .is_err()
    );
    assert_eq!(store.head(), head);
    assert!(transport.targets.is_empty());
}

#[test]
fn expired_or_future_permits_do_not_open_a_socket_and_are_not_refunded() {
    for now in [99, 1100] {
        let temp = Temp::new();
        let mut store = activated(&temp);
        let before = store.head();
        let mut transport = Fake {
            now,
            ..Fake::new(Reply::Unknown)
        };
        assert!(matches!(
            reads::poll(
                &mut store,
                &gateway(),
                input(1, Kind::Account, None),
                &mut transport
            )
            .unwrap(),
            Outcome::Unavailable
        ));
        assert_ne!(store.head(), before);
        assert!(store.transaction(id(1)).is_some());
        assert!(store.transaction(id(101)).is_none());
        assert!(transport.targets.is_empty());
        let head = store.head();
        assert!(
            reads::poll(
                &mut store,
                &gateway(),
                input(1, Kind::Account, None),
                &mut transport
            )
            .is_err()
        );
        assert_eq!(store.head(), head);
    }
}

#[test]
fn rate_limit_blocks_all_pool_requests_and_survives_restart() {
    let temp = Temp::new();
    let mut store = activated(&temp);
    let mut reply = response(429, json!({"error":"limited"}));
    if let Reply::Response { retry_after_ms, .. } = &mut reply {
        *retry_after_ms = Some(1000);
    }
    let mut transport = Fake::new(reply);
    assert!(matches!(
        reads::poll(
            &mut store,
            &gateway(),
            input(1, Kind::Trades, None),
            &mut transport
        )
        .unwrap(),
        Outcome::Limited
    ));
    assert!(store.transaction(id(101)).is_none());
    drop(store);
    let mut store = temp.open();
    let head = store.head();
    assert!(
        gateway()
            .reserve_read(&mut store, id(2), 101, true)
            .is_err()
    );
    assert!(
        reads::poll(
            &mut store,
            &gateway(),
            input(2, Kind::Account, None),
            &mut transport
        )
        .is_err()
    );
    assert_eq!(store.head(), head);
    assert_eq!(transport.targets.len(), 1);
    // The shared policy floors even a short Retry-After to the full venue window.
    transport.now = 60_100;
    let mut next = input(2, Kind::Account, None);
    next.at = 60_100;
    assert!(matches!(
        reads::poll(&mut store, &gateway(), next, &mut transport).unwrap(),
        Outcome::Unavailable
    ));
    assert_eq!(transport.targets.len(), 2);
}

#[test]
fn ordinary_reads_preserve_cleanup_capacity_even_after_restart() {
    let temp = Temp::new();
    let mut store = activated(&temp);
    let mut transport = Fake::new(Reply::Unknown);
    for n in 1..=4 {
        reads::poll(
            &mut store,
            &gateway(),
            input(n, Kind::Account, None),
            &mut transport,
        )
        .unwrap();
    }
    drop(store);
    let mut store = temp.open();
    let head = store.head();
    assert!(
        reads::poll(
            &mut store,
            &gateway(),
            input(5, Kind::Account, None),
            &mut transport
        )
        .is_err()
    );
    assert_eq!(store.head(), head);
    let mut cleanup = input(5, Kind::Account, None);
    cleanup.cleanup = true;
    reads::poll(&mut store, &gateway(), cleanup, &mut transport).unwrap();
    let mut exhausted = input(6, Kind::Account, None);
    exhausted.cleanup = true;
    assert!(reads::poll(&mut store, &gateway(), exhausted, &mut transport).is_err());
    assert_eq!(transport.targets.len(), 5);
}

#[test]
fn malformed_success_is_archived_as_a_gap_without_a_coverage_certificate() {
    let temp = Temp::new();
    let mut store = activated(&temp);
    let ledger = store.state().unwrap().ledger().clone();
    let mut transport = Fake::new(response(200, json!({"success":true,"data":"invalid"})));
    let Outcome::Ingested(result) = reads::poll(
        &mut store,
        &gateway(),
        input(1, Kind::Trades, None),
        &mut transport,
    )
    .unwrap() else {
        panic!("response was not archived");
    };
    assert_eq!(result.receipt.inputs, [InputResult::Unnormalized]);
    assert_eq!(store.state().unwrap().ledger(), &ledger);
    let tx = store.transaction(id(101)).unwrap();
    assert!(
        tx.inputs
            .iter()
            .all(|i| i.source_cut.is_none() && i.event.is_none())
    );
    assert!(
        observation::replay(&mut store, &profile())
            .unwrap()
            .gaps
            .contains(&Gap::Unnormalized)
    );
    let head = store.head();
    assert!(
        reads::poll(
            &mut store,
            &gateway(),
            input(1, Kind::Trades, None),
            &mut transport
        )
        .is_err()
    );
    assert_eq!(store.head(), head);
    assert_eq!(transport.targets.len(), 1);
}

#[test]
fn non_success_and_uncertain_responses_never_retry_or_change_entitlements() {
    for reply in [
        Reply::Unknown,
        response(401, json!({"error":"unauthorized"})),
        response(503, json!({"error":"temporary"})),
    ] {
        let temp = Temp::new();
        let mut store = activated(&temp);
        let ledger = store.state().unwrap().ledger().clone();
        let mut transport = Fake::new(reply);
        assert!(matches!(
            reads::poll(
                &mut store,
                &gateway(),
                input(1, Kind::Account, None),
                &mut transport
            )
            .unwrap(),
            Outcome::Unavailable
        ));
        assert_eq!(transport.targets.len(), 1);
        assert_eq!(store.state().unwrap().ledger(), &ledger);
        assert!(store.transaction(id(1)).is_some());
        assert!(store.transaction(id(101)).is_none());
    }
}

#[test]
fn missing_and_revoked_gateway_authority_cannot_schedule_reads() {
    let temp = Temp::new();
    let mut store = temp.create();
    let mut transport = Fake::new(Reply::Unknown);
    let head = store.head();
    assert!(
        reads::poll(
            &mut store,
            &gateway(),
            input(1, Kind::Account, None),
            &mut transport
        )
        .is_err()
    );
    assert_eq!(store.head(), head);
    gateway().activate(&mut store, id(250), 0).unwrap();
    gateway().deactivate(&mut store, id(251), 1).unwrap();
    let head = store.head();
    assert!(
        reads::poll(
            &mut store,
            &gateway(),
            input(1, Kind::Account, None),
            &mut transport
        )
        .is_err()
    );
    assert_eq!(store.head(), head);
    assert!(transport.targets.is_empty());
}

#[test]
fn backward_time_invalid_utf8_and_oversized_bodies_cannot_enter_the_journal() {
    for (received_at, body) in [
        (99, b"{}".to_vec()),
        (100, vec![0xff]),
        (100, vec![b'x'; observation::MAX_BODY + 1]),
    ] {
        let temp = Temp::new();
        let mut store = activated(&temp);
        let mut transport = Fake::new(Reply::Response {
            status: 200,
            body: PrivateBytes::new(body).unwrap(),
            received_at,
            retry_after_ms: None,
        });
        assert!(
            reads::poll(
                &mut store,
                &gateway(),
                input(1, Kind::Account, None),
                &mut transport
            )
            .is_err()
        );
        assert!(store.transaction(id(1)).is_some());
        assert!(store.transaction(id(101)).is_none());
        assert_eq!(transport.targets.len(), 1);
    }
}
