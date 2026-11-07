//! Offline canonical/negative codec qualification; synthetic data, no venue I/O.
use cinder_bulk::{
    Error, Network, Scope,
    fills::{self, Coverage, Reason, TradeId},
    precision::{Market, scaled},
    wire::{Action, MAX_WIRE, TimeInForce},
};
use cinder_kernel::identity::*;

fn scope(network: Network) -> Scope {
    Scope::new(
        EventScope {
            domain: Domain {
                network: NetworkId::new([1; 32]).unwrap(),
                deployment: DeploymentId::new([2; 32]).unwrap(),
            },
            venue: VenueId::new([3; 32]).unwrap(),
            account: VenueAccountId::new([4; 32]).unwrap(),
            namespace: NamespaceId::new([5; 32]).unwrap(),
        },
        network,
        [1; 32],
    )
    .unwrap()
}
fn market() -> Market {
    Market::new("BTC-USD", "0.00000001", "0.1", 8).unwrap()
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn owner() -> String {
    bs58::encode([1; 32]).into_string()
}
fn other() -> String {
    bs58::encode([2; 32]).into_string()
}
fn row() -> String {
    format!(
        r#"{{"tradeId":"9007199254740993:18446744073709551615","maker":"{}","taker":"{}","orderIdMaker":"{}","orderIdTaker":"{}","isBuy":false,"symbol":"BTC-USD","amount":0.00100001,"price":100.1,"makerFee":-0.00000001,"takerFee":0.00000002,"fee":-0.00000001,"reasonCode":0,"iso":false,"slot":9007199254740993,"sequence":18446744073709551615,"timestamp":1763316177219383423}}"#,
        owner(),
        other(),
        owner(),
        other()
    )
}
fn page(rows: &str) -> String {
    format!(
        r#"{{"data":[{rows}],"page":{{"nextCursor":null,"hasMore":false,"asOfSlot":9007199254740994,"startSlot":9007199254740993,"endSlot":9007199254740993,"coverage":"complete","minAvailableSlot":0}}}}"#
    )
}
fn ws() -> String {
    format!(
        r#"{{"type":"account","topic":"account.{}","data":{{"type":"fill","tradeId":"9007199254740993:18446744073709551615","symbol":"BTC-USD","orderId":"{}","price":100.1,"size":0.00100001,"fee":-0.00000001,"isBuy":true,"maker":true,"reasonCode":0,"timestamp":1763316177219383423}}}}"#,
        owner(),
        owner()
    )
}

#[test]
fn exact_native_decimals_include_scientific_notation_without_floating_money() {
    for (s, p, n) in [
        ("1e-8", 8, 1),
        ("100.100", 8, 10_010_000_000),
        ("-1E-8", 8, -1),
        ("1.000000000", 8, 100_000_000),
        ("1e+2", 0, 100),
        ("-170141183460469231731687303715884105728", 0, i128::MIN),
    ] {
        assert_eq!(scaled(s, p), Ok(n), "{s}");
    }
    for s in [
        "",
        "-0",
        "-0e3",
        "+1",
        "01",
        "1.",
        ".1",
        "1e",
        "1e-",
        "1e1e2",
        "1e9999",
        "NaN",
        "Infinity",
        " 1",
        "1e-9",
        "1.000000001",
        "1,2",
    ] {
        assert!(scaled(s, 8).is_err(), "{s}");
    }
    assert!(scaled("1", 39).is_err());
}
#[test]
fn exact_market_grids_have_no_zero_step_negative_or_off_grid_path() {
    let m = market();
    assert_eq!(m.quantity("1e-8"), Ok(1));
    assert_eq!(m.price("100.1"), Ok(1001));
    assert_eq!(m.fee("-1e-8"), Ok(-1));
    for p in ["0", "-1", "100.01", "0.000000001"] {
        assert!(m.price(p).is_err());
    }
    for (s, p) in [("0", "0.1"), ("-1", "0.1"), ("1e-9", "0.1"), ("1", "0")] {
        assert!(Market::new("BTC-USD", s, p, 8).is_err());
    }
    assert!(Market::new("btc-usd", "1", "1", 8).is_err());
}
#[test]
fn upstream_legacy_limit_bytes_and_indexed_order_id_are_exact() {
    // Derived limit subaction from Apache-2.0 SDK legacy vector; provenance in fixtures/README.md.
    let action = Action::limit(&market(), true, "100", "3", TimeInForce::Gtc).unwrap();
    let message = hex(&action.message(&scope(Network::Devnet), 42).unwrap());
    assert_eq!(
        message,
        "01000000000000000100000007000000000000004254432d5553440100e40b540200000000a3e111000000000000000000002a00000000000000010101010101010101010101010101010101010101010101010101010101010103"
    );
    assert_eq!(
        bs58::encode(action.order_id(&scope(Network::Devnet), 42).unwrap()).into_string(),
        "6tU7wYpGmJt7MgiK7HzEVjkxpJ2UCjSL7VwgMrrds5Vp"
    );
}
#[test]
fn native_domain_changes_signed_bytes_but_not_unscoped_order_hash() {
    let a = Action::limit(&market(), false, "100", "0.01", TimeInForce::Ioc).unwrap();
    let main = scope(Network::Mainnet);
    let test = scope(Network::Testnet);
    let x = a.message(&main, u64::MAX).unwrap();
    let y = a.message(&test, u64::MAX).unwrap();
    assert_eq!(&x[..x.len() - 1], &y[..y.len() - 1]);
    assert_eq!(x.last(), Some(&1));
    assert_eq!(y.last(), Some(&2));
    assert_eq!(a.order_id(&main, 42), a.order_id(&test, 42));
    assert_ne!(main.commitment(), test.commitment());
    assert!(a.message(&main, 0).is_err());
}
#[test]
fn cancellation_json_and_binary_use_the_documented_cx_tag() {
    let a = Action::cancel("BTC-USD", [2; 32]).unwrap();
    let j: serde_json::Value = serde_json::from_slice(&a.json().unwrap()).unwrap();
    assert!(j.get("cx").is_some());
    assert!(j.get("cancel").is_none());
    let bytes = a.message(&scope(Network::Testnet), 42).unwrap();
    assert_eq!(&bytes[8..12], &3_u32.to_le_bytes());
    assert!(a.order_id(&scope(Network::Testnet), 42).is_err());
}
#[test]
fn encoder_uses_exact_strings_and_no_native_pooled_reduce_only_or_isolation() {
    let a = Action::limit(&market(), true, "100.1", "0.00100001", TimeInForce::Alo).unwrap();
    let j: serde_json::Value = serde_json::from_slice(&a.json().unwrap()).unwrap();
    assert_eq!(j["l"]["px"], "100.10000000");
    assert_eq!(j["l"]["sz"], "0.00100001");
    assert_eq!(j["l"]["r"], false);
    assert_eq!(j["l"]["i"], false);
    assert!(Action::limit(&market(), true, "100.01", "1", TimeInForce::Gtc).is_err());
    assert!(Action::limit(&market(), true, "100", "99999999", TimeInForce::Gtc).is_err());
}
#[test]
fn bounded_wire_atoms_match_independent_native_binary64_rounding_oracle() {
    // f64 is used only in this independent test oracle, never production arithmetic.
    let mut seed = 0x812309_u64;
    for _ in 0..20000 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let n = seed % MAX_WIRE + 1;
        let text = format!("{}.{:08}", n / 100_000_000, n % 100_000_000);
        assert_eq!((text.parse::<f64>().unwrap() * 1e8).round() as u64, n);
        assert_eq!(scaled(&text, 8).unwrap(), i128::from(n));
    }
}
#[test]
fn rest_keeps_wide_ids_exact_price_fee_and_account_side_for_maker() {
    let p = fills::rest(
        &scope(Network::Testnet),
        &[market()],
        page(&row()).as_bytes(),
    )
    .unwrap();
    let f = &p.fills[0];
    assert_eq!(f.id.slot, 9007199254740993);
    assert_eq!(f.id.sequence, u64::MAX);
    assert_eq!(f.timestamp, 1763316177219383423);
    assert_eq!(f.quantity, 100001);
    assert_eq!(f.price, 1001);
    assert_eq!(f.fee, -1);
    assert!(!f.buy);
    assert_eq!(f.order, [1; 32]);
    assert_eq!(p.coverage, Coverage::Complete);
}
#[test]
fn rest_and_ws_maker_side_normalize_to_one_semantic_execution() {
    let s = scope(Network::Testnet);
    let rest = fills::rest(&s, &[market()], page(&row()).as_bytes()).unwrap();
    let ws = fills::stream(&s, &market(), ws().as_bytes()).unwrap();
    assert_eq!(rest.fills[0], ws);
    let mut w = ws_json();
    w["data"]["maker"] = false.into();
    w["data"]["isBuy"] = false.into();
    assert!(
        !fills::stream(&s, &market(), &serde_json::to_vec(&w).unwrap())
            .unwrap()
            .buy
    );
}
fn ws_json() -> serde_json::Value {
    serde_json::from_str(&ws()).unwrap()
}
#[test]
fn wrong_account_topic_symbol_and_native_isolation_are_not_accepted() {
    let s = scope(Network::Testnet);
    for (field, value) in [
        ("topic", serde_json::json!("account.wrong")),
        ("type", serde_json::json!("trades")),
    ] {
        let mut w = ws_json();
        w[field] = value;
        assert!(fills::stream(&s, &market(), &serde_json::to_vec(&w).unwrap()).is_err());
    }
    for (field, value) in [
        ("symbol", serde_json::json!("ETH-USD")),
        ("iso", serde_json::json!(true)),
        ("isoPubkey", serde_json::json!(owner())),
    ] {
        let mut w = ws_json();
        w["data"][field] = value;
        assert!(fills::stream(&s, &market(), &serde_json::to_vec(&w).unwrap()).is_err());
    }
    assert!(
        fills::rest(
            &s,
            &[market()],
            page(&row().replace(&owner(), &other())).as_bytes()
        )
        .is_err()
    );
}
#[test]
fn fill_scope_separates_network_account_venue_deployment_and_namespace() {
    let id = TradeId::parse("1:2").unwrap();
    let s = scope(Network::Testnet);
    assert_ne!(id.scoped(&s), id.scoped(&scope(Network::Devnet)));
    let base = EventScope {
        domain: Domain {
            network: NetworkId::new([1; 32]).unwrap(),
            deployment: DeploymentId::new([2; 32]).unwrap(),
        },
        venue: VenueId::new([3; 32]).unwrap(),
        account: VenueAccountId::new([4; 32]).unwrap(),
        namespace: NamespaceId::new([5; 32]).unwrap(),
    };
    for source in [
        EventScope {
            venue: VenueId::new([8; 32]).unwrap(),
            ..base
        },
        EventScope {
            account: VenueAccountId::new([8; 32]).unwrap(),
            ..base
        },
        EventScope {
            namespace: NamespaceId::new([8; 32]).unwrap(),
            ..base
        },
        EventScope {
            domain: Domain {
                deployment: DeploymentId::new([8; 32]).unwrap(),
                ..base.domain
            },
            ..base
        },
    ] {
        assert_ne!(
            id.scoped(&s),
            id.scoped(&Scope::new(source, Network::Testnet, [1; 32]).unwrap())
        );
    }
    assert_ne!(
        id.scoped(&s),
        id.scoped(&Scope::new(base, Network::Testnet, [2; 32]).unwrap())
    );
}
#[test]
fn missing_native_identity_null_noncanonical_or_conflicting_ids_are_rejected() {
    for id in [
        "1",
        "1:",
        "01:2",
        "1:02",
        "-1:2",
        "1:2:3",
        "18446744073709551616:2",
    ] {
        assert!(TradeId::parse(id).is_err());
    }
    for r in [
        row()
            .replace("\"tradeId\":\"9007199254740993:18446744073709551615\",", "")
            .replace("\"sequence\":18446744073709551615,", ""),
        row()
            .replace("\"tradeId\":\"9007199254740993:18446744073709551615\",", "")
            .replace("\"slot\":9007199254740993,", ""),
        row().replace(
            "\"tradeId\":\"9007199254740993:18446744073709551615\"",
            "\"tradeId\":null",
        ),
        row().replace("\"slot\":9007199254740993", "\"slot\":9007199254740994"),
    ] {
        assert!(fills::rest(&scope(Network::Testnet), &[market()], page(&r).as_bytes()).is_err());
    }
}
#[test]
fn observed_paper_fill_uses_official_slot_sequence_identity_without_rewriting() {
    let body = include_bytes!("fixtures/paper-20261020-missing-trade-id.json");
    let market = Market::new("BTC-USD", "0.00000001", "0.01", 8).unwrap();
    let original = fills::rest(
        &scope(Network::Testnet),
        std::slice::from_ref(&market),
        body,
    )
    .unwrap();
    // The SDK defines the absent-field identity as the required native pair.
    // A response with the explicit redundant spelling denotes the SAME event.
    let mut amended: serde_json::Value = serde_json::from_slice(body).unwrap();
    amended["data"][0]["tradeId"] = "10710440:0".into();
    let page = fills::rest(
        &scope(Network::Testnet),
        std::slice::from_ref(&market),
        &serde_json::to_vec(&amended).unwrap(),
    )
    .unwrap();
    assert_eq!(page.fills.len(), 1);
    assert_eq!(page.fills[0].quantity, 2000);
    assert_eq!(page.fills[0].price, 10_000_000);
    assert_eq!(page.fills[0].fee, 0);
    assert_eq!(page.coverage, Coverage::Unknown);
    assert_eq!(original.fills, page.fills);
    // Missing/null/negative/overflow native fields and explicit null/mismatch
    // must never receive a guessed replacement from timestamp/order data.
    for (field, value) in [
        ("tradeId", serde_json::Value::Null),
        ("tradeId", "10710440:1".into()),
        ("sequence", (-1).into()),
        ("sequence", "18446744073709551616".into()),
        ("slot", serde_json::Value::Null),
    ] {
        let mut bad: serde_json::Value = serde_json::from_slice(body).unwrap();
        bad["data"][0][field] = value;
        assert!(
            fills::rest(
                &scope(Network::Testnet),
                std::slice::from_ref(&market),
                &serde_json::to_vec(&bad).unwrap()
            )
            .is_err()
        );
    }
}
#[test]
fn explicit_and_native_pair_id_spellings_deduplicate_and_conflicting_pairs_do_not() {
    let legacy = row().replace("\"tradeId\":\"9007199254740993:18446744073709551615\",", "");
    let mixed = page(&format!("{},{}", row(), legacy));
    assert_eq!(
        fills::rest(&scope(Network::Testnet), &[market()], mixed.as_bytes())
            .unwrap()
            .fills
            .len(),
        1
    );
    let conflict = page(&format!(
        "{},{}",
        row(),
        legacy.replace("\"price\":100.1", "\"price\":100.2")
    ));
    assert!(matches!(
        fills::rest(&scope(Network::Testnet), &[market()], conflict.as_bytes()),
        Err(Error::Conflict)
    ));
}
#[test]
fn duplicate_executions_collapse_but_conflicting_economics_are_reported() {
    let s = scope(Network::Testnet);
    assert_eq!(
        fills::rest(
            &s,
            &[market()],
            page(&format!("{},{}", row(), row())).as_bytes()
        )
        .unwrap()
        .fills
        .len(),
        1
    );
    assert!(matches!(
        fills::rest(
            &s,
            &[market()],
            page(&format!("{},{}", row(), row().replace("100.1", "100.2"))).as_bytes()
        ),
        Err(Error::Conflict)
    ));
}
#[test]
fn raw_number_and_string_decimals_share_exact_semantics() {
    let s = scope(Network::Testnet);
    let a = fills::rest(&s, &[market()], page(&row()).as_bytes())
        .unwrap()
        .fills;
    let r = row()
        .replace("0.00100001", "\"1.00001e-3\"")
        .replace("100.1", "\"100.100000000\"")
        .replace("-0.00000001", "-1e-8");
    assert_eq!(
        a,
        fills::rest(&s, &[market()], page(&r).as_bytes())
            .unwrap()
            .fills
    );
}
#[test]
fn scalar_overprecision_duplicate_fields_and_bad_fee_selection_fail_closed() {
    for r in [
        row().replace("0.00100001", "0.001000000001"),
        row().replace("\"price\":100.1", "\"price\":100.1,\"price\":100.2"),
        row().replace("\"fee\":-0.00000001", "\"fee\":0.00000002"),
        row().replace("\"iso\":false", "\"iso\":true"),
    ] {
        assert!(fills::rest(&scope(Network::Testnet), &[market()], page(&r).as_bytes()).is_err());
    }
}
#[test]
fn native_reason_is_explicit_and_unknown_or_conflicting_labels_are_not_guessed() {
    for (code, reason) in [
        (0, Reason::Trade),
        (1, Reason::Liquidation),
        (2, Reason::Adl),
        (3, Reason::LiquidationSweep),
    ] {
        let w = ws().replace("\"reasonCode\":0", &format!("\"reasonCode\":{code}"));
        assert_eq!(
            fills::stream(&scope(Network::Testnet), &market(), w.as_bytes())
                .unwrap()
                .reason,
            reason
        );
    }
    for w in [
        ws().replace("\"reasonCode\":0", "\"reasonCode\":4"),
        ws().replace(
            "\"reasonCode\":0",
            "\"reasonCode\":2,\"reason\":\"liquidation\"",
        ),
    ] {
        assert!(fills::stream(&scope(Network::Testnet), &market(), w.as_bytes()).is_err());
    }
}
#[test]
fn coverage_cursor_bounds_and_retention_do_not_admit_malformed_pages() {
    let s = scope(Network::Testnet);
    let base = page(&row());
    for p in [
        base.replace("\"hasMore\":false", "\"hasMore\":true"),
        base.replace("\"nextCursor\":null", "\"nextCursor\":\"\""),
        base.replace("\"asOfSlot\":9007199254740994", "\"asOfSlot\":1"),
        base.replace(
            "\"minAvailableSlot\":0",
            "\"minAvailableSlot\":18446744073709551615",
        ),
        base.replace(
            "\"startSlot\":9007199254740993",
            "\"startSlot\":9007199254740994",
        ),
    ] {
        assert!(fills::rest(&s, &[market()], p.as_bytes()).is_err());
    }
    for coverage in ["partial", "unknown"] {
        assert!(
            fills::rest(
                &s,
                &[market()],
                base.replace("complete", coverage).as_bytes()
            )
            .is_ok()
        );
    }
    assert!(
        fills::rest(&s, &[market()], page("").as_bytes())
            .unwrap()
            .fills
            .is_empty()
    );
}
#[test]
fn observation_bodies_rows_and_private_debug_output_are_bounded() {
    let s = scope(Network::Testnet);
    assert!(fills::rest(&s, &[market()], &vec![b' '; fills::MAX_BODY + 1]).is_err());
    assert!(
        fills::rest(
            &s,
            &[market()],
            page(&vec![row(); fills::MAX_ROWS + 1].join(",")).as_bytes()
        )
        .is_err()
    );
    let f = fills::stream(&s, &market(), ws().as_bytes()).unwrap();
    assert_eq!(format!("{f:?}"), "Fill([PRIVATE])");
    assert_eq!(format!("{s:?}"), "Scope([PRIVATE])");
}

#[test]
fn page_order_and_retained_range_metadata_are_explicit() {
    let s = scope(Network::Testnet);
    let newer = row();
    let older = row().replace("18446744073709551615", "18446744073709551614");
    let p = fills::rest(
        &s,
        &[market()],
        page(&format!("{newer},{older}")).as_bytes(),
    )
    .unwrap();
    assert_eq!(p.fills.len(), 2);
    assert_eq!(p.as_of_slot, 9007199254740994);
    assert_eq!(p.start_slot, 9007199254740993);
    assert_eq!(p.end_slot, 9007199254740993);
    assert_eq!(p.min_available_slot, Some(0));
    assert!(
        fills::rest(
            &s,
            &[market()],
            page(&format!("{older},{newer}")).as_bytes()
        )
        .is_err()
    );
}

#[test]
fn fill_retains_the_exact_market_mapping_as_well_as_execution_identity() {
    let s = scope(Network::Testnet);
    let original = fills::stream(&s, &market(), ws().as_bytes()).unwrap();
    let changed = Market::new("BTC-USD", "0.00000001", "0.01", 8).unwrap();
    let changed = fills::stream(&s, &changed, ws().as_bytes()).unwrap();
    assert_eq!(original.key, changed.key); // Same event, never a second economic credit.
    assert_eq!(original.market.symbol(), "BTC-USD");
    assert_ne!(original.market, changed.market);
    assert_ne!(original, changed);
}

#[test]
fn exact_zero_accepts_large_bounded_exponents_without_accepting_negative_zero() {
    for text in ["0e-100", "0e100", "0e-32768", "0e32767", "0.000e-100"] {
        assert_eq!(scaled(text, 8), Ok(0), "{text}");
    }
    assert_eq!(scaled("1e-100", 8), Err(Error::Invalid));
    assert_eq!(scaled("1e100", 8), Err(Error::Limit));
    for text in ["-0e-100", "-0e100", "0e32768", "0e", "0e+", "0e--1"] {
        assert!(scaled(text, 8).is_err(), "{text}");
    }
}

#[test]
fn ioc_collar_retains_native_precision_without_relaxing_resting_ticks_or_bounds() {
    let m = market();
    let action = Action::limit(&m, true, "100.01", "1", TimeInForce::Ioc).unwrap();
    let json: serde_json::Value = serde_json::from_slice(&action.json().unwrap()).unwrap();
    assert_eq!(json["l"]["px"], "100.01000000");
    let message = action.message(&scope(Network::Testnet), 42).unwrap();
    assert_eq!(&message[28..36], &10_001_000_000_u64.to_le_bytes());
    for tif in [TimeInForce::Gtc, TimeInForce::Alo] {
        assert!(Action::limit(&m, true, "100.01", "1", tif).is_err());
    }
    for price in ["0", "-1", "1e-9", "99999999"] {
        assert!(Action::limit(&m, true, price, "1", TimeInForce::Ioc).is_err());
    }
    assert!(Action::limit(&m, true, "100.01", "1e-9", TimeInForce::Ioc).is_err());
}

#[test]
fn mixed_market_page_does_not_erase_a_configured_adverse_fill() {
    let s = scope(Network::Testnet);
    let eth = row().replace("BTC-USD", "ETH-USD");
    let btc = row()
        .replace("18446744073709551615", "18446744073709551614")
        .replace("\"reasonCode\":0", "\"reasonCode\":2");
    let p = fills::rest(&s, &[market()], page(&format!("{eth},{btc}")).as_bytes()).unwrap();
    assert_eq!(p.fills.len(), 1);
    assert_eq!(p.fills[0].reason, Reason::Adl);
    assert_eq!(p.unmapped.len(), 1);
    assert_eq!(p.unmapped[0].symbol, "ETH-USD");
    assert_eq!(p.unmapped[0].body(), eth);
    assert_eq!(p.unmapped[0].key, p.unmapped[0].id.scoped(&s));
    assert_eq!(format!("{:?}", p.unmapped[0]), "UnmappedFill([PRIVATE])");
    assert_eq!(p.coverage, Coverage::Complete); // Reported coverage is not reconciliation.
}

#[test]
fn mixed_market_page_uses_each_mapping_and_preserves_cross_market_order() {
    let s = scope(Network::Testnet);
    let markets = [
        market(),
        Market::new("ETH-USD", "0.00001", "0.01", 6).unwrap(),
    ];
    let eth = row()
        .replace("BTC-USD", "ETH-USD")
        .replace("0.00100001", "0.002")
        .replace("100.1", "100.01")
        .replace("-0.00000001", "-0.000001")
        .replace("0.00000002", "0.000002");
    let btc = row().replace("18446744073709551615", "18446744073709551614");
    let p = fills::rest(&s, &markets, page(&format!("{eth},{btc}")).as_bytes()).unwrap();
    assert_eq!(p.fills.len(), 2);
    assert!(p.unmapped.is_empty());
    assert_eq!(p.fills[0].market, markets[1]);
    assert_eq!(
        (p.fills[0].quantity, p.fills[0].price, p.fills[0].fee),
        (200, 10001, -1)
    );
    assert_eq!(p.fills[1].market, markets[0]);
    assert_eq!(
        (p.fills[1].quantity, p.fills[1].price, p.fills[1].fee),
        (100001, 1001, -1)
    );
    assert!(fills::rest(&s, &markets, page(&format!("{btc},{eth}")).as_bytes()).is_err());
}

#[test]
fn unmapped_rows_are_bounded_deduplicated_and_cannot_hide_identity_conflicts() {
    let s = scope(Network::Testnet);
    let eth = row().replace("BTC-USD", "ETH-USD");
    let p = fills::rest(&s, &[], page(&format!("{eth},{eth}")).as_bytes()).unwrap();
    assert!(p.fills.is_empty());
    assert_eq!(p.unmapped.len(), 1);
    assert_eq!(p.unmapped[0].body(), eth);
    let changed = eth.replace("100.1", "100.2");
    assert!(matches!(
        fills::rest(
            &s,
            &[market()],
            page(&format!("{eth},{changed}")).as_bytes()
        ),
        Err(Error::Conflict)
    ));
    for rows in [format!("{},{}", row(), eth), format!("{},{}", eth, row())] {
        assert!(matches!(
            fills::rest(&s, &[market()], page(&rows).as_bytes()),
            Err(Error::Conflict)
        ));
    }
    for raw in [
        eth.replace(&owner(), &other()),
        eth.replace("\"slot\":9007199254740993", "\"slot\":1"),
        eth.replace("ETH-USD", "eth-usd"),
    ] {
        assert!(fills::rest(&s, &[market()], page(&raw).as_bytes()).is_err());
    }
    assert!(matches!(
        fills::rest(&s, &[market(), market()], page("").as_bytes()),
        Err(Error::Conflict)
    ));
    assert!(matches!(
        fills::rest(
            &s,
            &vec![market(); fills::MAX_MARKETS + 1],
            page("").as_bytes()
        ),
        Err(Error::Limit)
    ));
}
