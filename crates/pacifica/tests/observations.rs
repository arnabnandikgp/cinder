//! Joined synthetic wire/journal tests; no venue or network qualification.
#[path = "../../journal/tests/support/mod.rs"]
mod support;
use cinder_journal::orders::Observation;
use cinder_journal::{model::*, orders::*, *};
use cinder_kernel::identity::{EconomicEventId, EventKey};
use cinder_kernel::ledger::{evidence::*, *};
use cinder_pacifica::{client_id, observation::*, profile::*};
use serde_json::json;
use sha2::{Digest, Sha256};
use support::*;

fn profile() -> Profile {
    Profile {
        config: config(),
        source: config().sources[0].scope,
        account: "fixture-pool".into(),
        environment: "synthetic-only".into(),
        revision: 1,
        evidence: "offline test, not native qualification".into(),
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
fn msg(kind: Kind, body: serde_json::Value) -> Message {
    Message {
        kind,
        account: "fixture-pool".into(),
        cursor: None,
        received_at: 100,
        body: body.to_string(),
    }
}
fn rest(data: serde_json::Value) -> serde_json::Value {
    json!({"success":true,"data":data,"has_more":false})
}
fn row(id: u64, side: &str) -> serde_json::Value {
    json!({"history_id":id,"order_id":9007199254741001_u64,"client_order_id":client_id(attempt(2)),"symbol":"BTC","amount":"2","price":"100","entry_price":"100","fee":"1","pnl":"-1","event_type":"fulfill_taker","side":side,"cause":"normal","created_at":50})
}
fn ws(id: u64) -> Message {
    msg(
        Kind::TradeStream,
        json!({"channel":"account_trades","data":[{"h":id,"i":9007199254741001_u64,"I":client_id(attempt(2)),"u":"fixture-pool","s":"BTC","a":"2","p":"100","o":"100","f":"1","n":"-1","te":"fulfill_taker","ts":"open_long","tc":"normal","t":50,"li":999,"it":0}]}),
    )
}
fn put(s: &mut Store, n: u8, m: Message) -> Committed {
    ingest(s, &profile(), m, CommitId::new([n; 32]).unwrap(), 100).unwrap()
}

#[test]
fn certified_source_cut_is_explicit_exact_body_bound_and_releases_only_complete_fills() {
    let t = Temp::new();
    let mut s = setup(&t);
    let message = msg(Kind::Trades, rest(json!([row(1, "open_long")])));
    let coverage = Coverage {
        profile: profile().commitment().unwrap(),
        body: Sha256::digest(message.body.as_bytes()).into(),
        through: 7,
        evidence: [92; 32],
    };
    for bad in [
        Coverage {
            profile: [99; 32],
            ..coverage.clone()
        },
        Coverage {
            body: [99; 32],
            ..coverage.clone()
        },
        Coverage {
            through: 0,
            ..coverage.clone()
        },
        Coverage {
            evidence: [0; 32],
            ..coverage.clone()
        },
    ] {
        let before = s.head();
        assert!(
            ingest_covered(
                &mut s,
                &profile(),
                message.clone(),
                CommitId::new([3; 32]).unwrap(),
                100,
                bad
            )
            .is_err()
        );
        assert_eq!(s.head(), before);
    }
    let result = ingest_covered(
        &mut s,
        &profile(),
        message.clone(),
        CommitId::new([3; 32]).unwrap(),
        100,
        coverage.clone(),
    )
    .unwrap();
    assert_eq!(
        result.receipt.inputs,
        [InputResult::Normalized(Disposition::Applied)]
    );
    assert_eq!(s.state().unwrap().orders()[0].executions[0].1, Some(7));
    let again = ingest_covered(
        &mut s,
        &profile(),
        message,
        CommitId::new([4; 32]).unwrap(),
        100,
        coverage,
    )
    .unwrap();
    assert_eq!(
        again.receipt.inputs,
        [InputResult::Normalized(Disposition::Duplicate)]
    );
    let fill = EventKey {
        scope: profile().source,
        event: EconomicEventId::new(b"trade:1").unwrap(),
        leg: 0,
    };
    let mut tx = transaction(
        s.head(),
        5,
        vec![],
        vec![Control::Order(Action::Release {
            request: request(2),
        })],
    );
    tx.at = 100;
    tx.order_observations.push(Observation {
        key: key(990),
        attempt: attempt(2),
        status: Status::Terminal(Terminal {
            filled: q(2),
            executions: vec![fill],
            through: 7,
        }),
        authority_epoch: 1,
        observed_at: 100,
        raw: raw(b"explicit synthetic qualified coverage"),
    });
    assert_eq!(s.commit(tx).unwrap().receipt.controls, None);
    assert!(!s.state().unwrap().holds()[0].active);
    let l = s.state().unwrap().ledger().clone();
    drop(s);
    assert_eq!(t.open().state().unwrap().ledger(), &l);
}

#[test]
fn ordinary_parser_cannot_upgrade_causal_coverage_by_a_later_duplicate() {
    let t = Temp::new();
    let mut s = setup(&t);
    let message = msg(Kind::Trades, rest(json!([row(1, "open_long")])));
    put(&mut s, 3, message.clone());
    let coverage = Coverage {
        profile: profile().commitment().unwrap(),
        body: Sha256::digest(message.body.as_bytes()).into(),
        through: 7,
        evidence: [92; 32],
    };
    ingest_covered(
        &mut s,
        &profile(),
        message,
        CommitId::new([4; 32]).unwrap(),
        100,
        coverage,
    )
    .unwrap();
    assert_eq!(s.state().unwrap().orders()[0].executions[0].1, None);
    assert!(!s.state().unwrap().orders()[0].complete());
    assert!(!replay(&mut s, &profile()).unwrap().complete_history());
}
fn setup(t: &Temp) -> Store {
    let mut s = t.create();
    s.commit(transaction(
        s.head(),
        1,
        vec![receipt(1, Owner::Customer(user(1)), 1000)],
        vec![Control::Order(Action::AdvanceAuthority {
            account: user(1),
            epoch: 1,
        })],
    ))
    .unwrap();
    let i = Intent {
        time_in_force: TimeInForce::GoodTilCancelled,
        request: request(2),
        quantity: q(10),
        minimum: p(90),
        maximum: p(110),
        maximum_fee_per_lot: cash(1),
        reduce_only: false,
        policy: config().policy,
        authority_epoch: 1,
        expires_at: 1000,
    };
    let approval = Approval {
        account: user(1),
        intent_hash: i.digest().unwrap(),
        authority_epoch: 1,
    };
    let c = s
        .commit(transaction(
            s.head(),
            2,
            vec![],
            vec![
                Control::Order(Action::Accept {
                    intent: Box::new(i),
                    approval,
                    reservations: vec![
                        Reservation {
                            resource: Resource::Customer(user(1)),
                            amount: cash(100),
                        },
                        Reservation {
                            resource: Resource::Location(Location::Venue),
                            amount: cash(100),
                        },
                    ],
                }),
                Control::Order(Action::Prepare {
                    attempt: attempt(2),
                }),
                Control::Expose(attempt(2)),
            ],
        ))
        .unwrap();
    assert_eq!(c.receipt.controls, None);
    s
}

#[test]
fn rejected_pages_preserve_prior_views_cursors_and_scan_identity_on_replay() {
    let order = |id, filled: &str| {
        json!({"order_id":id,"symbol":"BTC","side":"bid",
        "amount":"10","filled_amount":filled,"order_status":"open","updated_at":90})
    };
    for kind in [Kind::Orders, Kind::Trades] {
        let t = Temp::new();
        let mut s = setup(&t);
        put(&mut s, 3, msg(Kind::Orders, rest(json!([order(10, "2")]))));
        let before = replay(&mut s, &profile()).unwrap();
        let data = if kind == Kind::Orders {
            json!([order(20, "2"), order(21, "11")])
        } else {
            json!(
                (100..133)
                    .map(|id| row(id, "open_long"))
                    .collect::<Vec<_>>()
            )
        };
        let rejected = msg(
            kind,
            json!({"success":true,"data":data,
            "has_more":true,"next_cursor":"same-next-cursor"}),
        );
        assert_eq!(
            put(&mut s, 4, rejected).receipt.inputs,
            [InputResult::Unnormalized]
        );
        let after = replay(&mut s, &profile()).unwrap();
        assert_eq!(after.orders, before.orders);
        assert_eq!(after.cursors, before.cursors);
        assert!(after.gaps.contains(&Gap::Unnormalized));
        drop(s);
        let mut reopened = t.open();
        let rebuilt = replay(&mut reopened, &profile()).unwrap();
        assert_eq!(rebuilt.orders, before.orders);
        assert_eq!(rebuilt.cursors, before.cursors);
        put(
            &mut reopened,
            5,
            msg(
                kind,
                json!({"success":true,"data":[],
            "has_more":true,"next_cursor":"same-next-cursor"}),
            ),
        );
        let accepted = replay(&mut reopened, &profile()).unwrap();
        assert_eq!(accepted.cursors[&kind], Some("same-next-cursor".into()));
        assert!(!accepted.gaps.contains(&Gap::Pagination));
    }
}

#[test]
fn full_page_and_gap_inputs_reference_one_exact_archive_and_replay() {
    use sha2::{Digest, Sha256};
    let t = Temp::new();
    let mut s = setup(&t);
    let rows: Vec<_> = (1000..1032).map(|id| row(id, "open_long")).collect();
    let page = msg(Kind::Trades, rest(json!(rows)));
    assert!(page.body.len() < MAX_BODY);
    let mut invalid = msg(Kind::Trades, rest(json!([])));
    invalid.body = "malformed exact response".into();
    for (n, message, count) in [(3, page, 32), (4, invalid, 1)] {
        let exact_body = message.body.clone();
        put(&mut s, n, message);
        let tx = s.transaction(CommitId::new([n; 32]).unwrap()).unwrap();
        assert_eq!(tx.evidence.len(), 1);
        assert_eq!(tx.inputs.len(), count);
        let archived = tx.evidence[0].as_bytes();
        let payload = archived
            .strip_prefix(b"CINDER-PACIFICA-EVIDENCE-1\0")
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(payload).unwrap();
        assert_eq!(body["message"]["body"], exact_body);
        let digest = Sha256::digest(archived);
        for (ordinal, input) in tx.inputs.iter().enumerate() {
            let reference = input
                .raw
                .as_bytes()
                .strip_prefix(b"CINDER-PACIFICA-INPUT-1\0")
                .unwrap();
            assert_eq!(reference.len(), 36);
            assert_eq!(&reference[..32], digest.as_slice());
            let expected = if input.event.is_some() {
                ordinal as u32
            } else {
                u32::MAX
            };
            assert_eq!(&reference[32..], &expected.to_be_bytes());
        }
        assert!(
            tx.inputs
                .iter()
                .map(|i| i.raw.as_bytes().len())
                .sum::<usize>()
                < count * 64
        );
    }
    let state = s.state().unwrap().clone();
    let view = replay(&mut s, &profile()).unwrap();
    drop(s);
    let mut reopened = t.open();
    assert_eq!(reopened.state().unwrap(), &state);
    let rebuilt = replay(&mut reopened, &profile()).unwrap();
    assert_eq!(rebuilt.gaps, view.gaps);
    assert_eq!(rebuilt.cursors, view.cursors);
}
#[test]
fn rest_ws_overlap_exact_ids_post_once_replay_and_same_commit_retry() {
    let t = Temp::new();
    let mut s = setup(&t);
    let id = 9007199254740993_u64;
    let m = msg(Kind::Trades, rest(json!([row(id, "open_long")])));
    put(&mut s, 3, m.clone());
    let c = put(&mut s, 3, m);
    assert!(c.duplicate);
    let c = put(&mut s, 4, ws(id));
    assert_eq!(
        c.receipt.inputs,
        [InputResult::Normalized(Disposition::Duplicate)]
    );
    assert_eq!(
        s.state().unwrap().ledger().venue().positions()[0].quantity(),
        q(2)
    );
    assert_eq!(s.state().unwrap().ledger().venue().cash(), cash(999));
    assert_eq!(
        s.state()
            .unwrap()
            .ledger()
            .book(Owner::Customer(user(1)))
            .unwrap()
            .cash(),
        cash(999)
    );
    assert!(s.state().unwrap().orders()[0].executions[0].1.is_none());
    assert!(!s.state().unwrap().orders()[0].complete());
    let head = s.head();
    drop(s);
    let mut s = t.open();
    assert_eq!(s.head(), head);
    assert!(replay(&mut s, &profile()).unwrap().gaps.is_empty());
    assert!(!replay(&mut s, &profile()).unwrap().complete_history());
}
#[test]
fn reversal_legs_distinct_and_native_side_does_not_choose_customer() {
    let t = Temp::new();
    let mut s = t.create();
    let mut a = row(1, "close_long");
    a["pnl"] = json!("-1");
    put(
        &mut s,
        1,
        msg(Kind::Trades, rest(json!([a, row(2, "open_short")]))),
    );
    assert_eq!(
        s.state().unwrap().ledger().venue().positions()[0].quantity(),
        q(-4)
    );
    assert_eq!(s.state().unwrap().ledger().unresolved_attribution(), 2);
}
#[test]
fn full_snapshot_empty_flat_older_ignored_equal_conflict_not_replaced() {
    let t = Temp::new();
    let mut s = t.create();
    let snapshot = |li, amount: &str| {
        msg(
            Kind::PositionStream,
            json!({"channel":"account_positions","li":li,"data":[{"s":"BTC","d":"bid","a":amount,"p":"101.5","f":"0.000002","i":false}]}),
        )
    };
    put(&mut s, 1, snapshot(10, "2"));
    put(
        &mut s,
        2,
        msg(
            Kind::PositionStream,
            json!({"channel":"account_positions","li":11,"data":[]}),
        ),
    );
    put(&mut s, 3, snapshot(10, "2"));
    assert_eq!(
        replay(&mut s, &profile()).unwrap().positions.unwrap().1[0].quantity,
        q(0)
    );
    put(&mut s, 4, snapshot(11, "3"));
    let v = replay(&mut s, &profile()).unwrap();
    assert!(v.gaps.contains(&Gap::ConflictingSnapshot));
    assert_eq!(v.positions.unwrap().1[0].quantity, q(0));
    assert_eq!(
        s.state().unwrap().ledger().venue().positions()[0].quantity(),
        q(0)
    );
    assert!(s.state().unwrap().unresolved_raw() > 0);
}
#[test]
fn corrections_conflict_and_late_unseen_is_retained_not_reordered_into_basis() {
    let t = Temp::new();
    let mut s = setup(&t);
    put(&mut s, 3, ws(1));
    let mut changed = row(1, "open_long");
    changed["fee"] = json!("2");
    put(&mut s, 4, msg(Kind::Trades, rest(json!([changed]))));
    let mut late = row(2, "open_long");
    late["created_at"] = json!(40);
    put(&mut s, 5, msg(Kind::Trades, rest(json!([late]))));
    let v = replay(&mut s, &profile()).unwrap();
    assert!(v.gaps.contains(&Gap::ConflictingTrade));
    assert!(v.gaps.contains(&Gap::LateExecution));
    assert_eq!(
        s.state().unwrap().ledger().venue().positions()[0].quantity(),
        q(2)
    );
    assert!(
        s.transaction(CommitId::new([5; 32]).unwrap())
            .unwrap()
            .evidence[0]
            .as_bytes()
            .windows(10)
            .any(|x| x == b"created_at")
    );
}
#[test]
fn pagination_cycles_skips_and_disconnect_never_become_complete_history() {
    let t = Temp::new();
    let mut s = t.create();
    let first = msg(
        Kind::Trades,
        json!({"success":true,"data":[],"has_more":true,"next_cursor":"A"}),
    );
    put(&mut s, 1, first);
    assert_eq!(
        replay(&mut s, &profile()).unwrap().cursors[&Kind::Trades],
        Some("A".into())
    );
    let mut m = msg(
        Kind::Trades,
        json!({"success":true,"data":[],"has_more":true,"next_cursor":"A"}),
    );
    m.cursor = Some("A".into());
    put(&mut s, 2, m);
    put(&mut s, 3, msg(Kind::Disconnect, json!({})));
    let v = replay(&mut s, &profile()).unwrap();
    assert!(v.gaps.contains(&Gap::Pagination));
    assert!(v.gaps.contains(&Gap::Disconnected));
    assert!(!v.complete_history());
}
#[test]
fn unknown_funding_forced_events_and_observed_only_profiles_are_contained() {
    let t = Temp::new();
    let mut s = t.create();
    put(
        &mut s,
        1,
        msg(
            Kind::Funding,
            rest(
                json!([{"history_id":1,"symbol":"BTC","side":"bid","amount":"2","payout":"-1","rate":"0.001","created_at":40}]),
            ),
        ),
    );
    let mut forced = row(1, "close_long");
    forced["cause"] = json!("settlement");
    put(&mut s, 2, msg(Kind::Trades, rest(json!([forced]))));
    let v = replay(&mut s, &profile()).unwrap();
    assert!(v.gaps.contains(&Gap::FundingQualification));
    assert!(v.gaps.contains(&Gap::Unnormalized));
    assert_eq!(s.state().unwrap().ledger().venue().cash(), cash(0));
    let t2 = Temp::new();
    let mut s2 = t2.create();
    let mut p = profile();
    p.fills = Level::Observed;
    ingest(&mut s2, &p, ws(1), CommitId::new([1; 32]).unwrap(), 100).unwrap();
    assert!(
        replay(&mut s2, &p)
            .unwrap()
            .gaps
            .contains(&Gap::FillQualification)
    );
    assert!(replay(&mut s2, &profile()).is_err());
}
#[test]
fn lossless_codecs_reject_numeric_money_duplicate_aliases_wrong_account_and_overflow() {
    for mutate in 0..6 {
        let t = Temp::new();
        let mut s = t.create();
        let mut r = row(1, "open_long");
        match mutate {
            0 => r["amount"] = json!(1.2),
            1 => r["amount"] = json!("1e2"),
            2 => r["h"] = json!(2),
            3 => r["account"] = json!("other-pool"),
            4 => r["amount"] = json!("9223372036854775808"),
            _ => r["spot_fee"] = json!("0.1"),
        };
        put(&mut s, 1, msg(Kind::Trades, rest(json!([r]))));
        assert!(s.state().unwrap().unresolved_raw() > 0);
        assert_eq!(s.state().unwrap().ledger().venue().cash(), cash(0));
    }
}
#[test]
fn account_metrics_and_native_order_terminal_are_not_cash_or_release_authority() {
    let t = Temp::new();
    let mut s = setup(&t);
    put(
        &mut s,
        3,
        msg(
            Kind::Account,
            rest(
                json!({"balance":"998","account_equity":"1200","available_to_spend":"500","available_to_withdraw":"400","pending_balance":"2","updated_at":90}),
            ),
        ),
    );
    put(
        &mut s,
        4,
        msg(
            Kind::Orders,
            rest(
                json!([{"order_id":10,"client_order_id":client_id(attempt(2)),"symbol":"BTC","side":"bid","amount":"10","filled_amount":"2","order_status":"cancelled","updated_at":90}]),
            ),
        ),
    );
    let v = replay(&mut s, &profile()).unwrap();
    assert_eq!(v.account.unwrap().equity, cash(1200));
    assert_eq!(v.orders[&10].status, "cancelled");
    assert_eq!(s.state().unwrap().ledger().venue().cash(), cash(1000));
    assert!(!s.state().unwrap().orders()[0].complete());
    let e = event(
        80,
        Change::Reconcile(NativeCheck {
            expected_version: s.state().unwrap().ledger().version(),
            cash: Some(cash(998)),
            funding: None,
            positions: None,
            complete: false,
            resolves: vec![],
        }),
    );
    let mut tx = transaction(s.head(), 5, vec![e], vec![]);
    tx.at = 100;
    s.commit(tx).unwrap();
    assert_eq!(
        s.state()
            .unwrap()
            .ledger()
            .issues()
            .last()
            .unwrap()
            .difference,
        Some(cash(-2))
    );
    assert_eq!(s.state().unwrap().ledger().venue().cash(), cash(1000));
}
#[test]
fn grids_and_profile_dimensions_are_exact_and_debug_hides_raw() {
    let grid = Grid { places: 5, step: 1 };
    assert_eq!(grid.format(14).unwrap(), "0.00014");
    assert_eq!(grid.parse("0.00014").unwrap(), 14);
    let mut p = profile();
    p.markets[0].size = grid;
    assert!(p.commitment().is_err());
    assert!(!format!("{:?}", ws(1)).contains("fixture-pool"));
}

#[test]
fn sanitized_numeric_capture_preserves_net_fee_convention_and_two_atom_difference() {
    use cinder_journal::sqlite::SqliteBackend;
    use cinder_kernel::{amounts::QuoteAtoms, position::Market};
    let t = Temp::new();
    let mut p = profile();
    p.quote_places = 6;
    p.markets[0].size = Grid { places: 5, step: 1 };
    p.config.markets[0] = Market::new(p.config.markets[0].unit(), 10, 1).unwrap();
    let mut s = Journal::create(
        SqliteBackend::create(&t.db).unwrap(),
        FixtureProtection,
        p.config.clone(),
    )
    .unwrap();
    let mut tx = transaction(
        s.head(),
        1,
        vec![receipt(90, Owner::House, 50_000_000)],
        vec![],
    );
    tx.at = 10;
    s.commit(tx).unwrap();
    let mut m = msg(Kind::Trades, serde_json::Value::Null);
    m.body = include_str!("fixtures/observed-trades-sanitized.json").into();
    ingest(&mut s, &p, m, CommitId::new([2; 32]).unwrap(), 100).unwrap();
    let l = s.state().unwrap().ledger();
    assert_eq!(l.venue().positions()[0].quantity().lots(), 0);
    assert_eq!(l.venue().cash().atoms(), 49_948_265);
    assert!(!l.issues().iter().any(|i| i.kind == IssueKind::NativePnl));
    // Synthetic two-atom reconciliation gap, reproducing the SIZE of the known
    // research discrepancy, not claiming these six rows were the full live run.
    let e = event(
        91,
        Change::Reconcile(NativeCheck {
            expected_version: l.version(),
            cash: Some(QuoteAtoms::new(p.config.quote, 49_948_263)),
            funding: None,
            positions: None,
            complete: false,
            resolves: vec![],
        }),
    );
    let mut tx = transaction(s.head(), 3, vec![e], vec![]);
    tx.at = 100;
    s.commit(tx).unwrap();
    let l = s.state().unwrap().ledger();
    assert_eq!(l.issues().last().unwrap().difference.unwrap().atoms(), -2);
    assert_eq!(p.quote("-0.000002").unwrap().atoms(), -2);
    assert_eq!(l.venue().cash().atoms(), 49_948_265);
}

#[test]
fn unsupported_row_does_not_drop_other_qualified_facts_in_same_response() {
    let t = Temp::new();
    let mut s = setup(&t);
    let mut forced = row(2, "close_long");
    forced["cause"] = json!("settlement");
    put(
        &mut s,
        3,
        msg(Kind::Trades, rest(json!([row(1, "open_long"), forced]))),
    );
    assert_eq!(
        s.state().unwrap().ledger().venue().positions()[0].quantity(),
        q(2)
    );
    assert!(s.state().unwrap().unresolved_raw() > 0);
}

#[test]
fn body_row_bounds_wrong_profile_and_changed_retry_fail_without_progress() {
    let t = Temp::new();
    let mut s = t.create();
    let m = msg(Kind::Trades, rest(json!([])));
    put(&mut s, 1, m.clone());
    let head = s.head();
    let mut changed = m.clone();
    changed.body.push(' ');
    assert!(
        ingest(
            &mut s,
            &profile(),
            changed,
            CommitId::new([1; 32]).unwrap(),
            100
        )
        .is_err()
    );
    let mut big = m.clone();
    big.body = " ".repeat(MAX_BODY + 1);
    assert!(
        ingest(
            &mut s,
            &profile(),
            big,
            CommitId::new([2; 32]).unwrap(),
            100
        )
        .is_err()
    );
    let mut wrong = profile();
    wrong.config.venue_account = cinder_kernel::identity::VenueAccountId::new([99; 32]).unwrap();
    assert!(ingest(&mut s, &wrong, m, CommitId::new([2; 32]).unwrap(), 100).is_err());
    assert_eq!(s.head(), head);
    put(
        &mut s,
        2,
        msg(
            Kind::Trades,
            rest(json!(
                (0..=MAX_ROWS)
                    .map(|n| row(n as u64, "open_long"))
                    .collect::<Vec<_>>()
            )),
        ),
    );
    assert_eq!(
        s.state().unwrap().ledger().venue().positions()[0].quantity(),
        q(0)
    );
    assert!(s.state().unwrap().unresolved_raw() > 0);
}
