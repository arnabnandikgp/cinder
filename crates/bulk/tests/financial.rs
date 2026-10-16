//! Synthetic native financial shapes; no network, credentials or financial posting.
use cinder_bulk::financial::{
    fees::{self, FeeScope},
    history::{self, Observation, RiskKind},
};
use cinder_bulk::{
    Error, Network, Scope,
    financial::{
        account::{self, Metric},
        *,
    },
    precision::Market,
};
use cinder_kernel::{amounts::*, identity::*};
use serde_json::{Value, json};

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
fn unit() -> AssetUnit {
    AssetUnit {
        asset: AssetId::new([6; 32]).unwrap(),
        precision: PrecisionVersion::new(1).unwrap(),
    }
}
fn mapping(name: &str, id: u8) -> Mapping {
    Mapping::new(
        Market::new(name, "0.001", "0.1", 6).unwrap(),
        MarketUnit {
            market: MarketId::new([id; 32]).unwrap(),
            precision: PrecisionVersion::new(1).unwrap(),
            quote: unit(),
        },
    )
}
fn profile() -> Profile {
    Profile::new(
        scope(Network::Testnet),
        unit(),
        6,
        Target::Master,
        vec![mapping("BTC-USD", 7), mapping("ETH-USD", 8)],
    )
    .unwrap()
}
fn position() -> Value {
    json!({"symbol":"BTC-USD","size":"-0.012","price":"100.0123456789012345","fairPrice":"101.23456789","notional":"1.2","realizedPnl":"2","unrealizedPnl":"-0.1","fees":"-0.02","funding":"0.03","maintenanceMargin":"0.12","averageEntryTime":18446744073709551615_u64,"iso":false})
}
fn snapshot() -> Value {
    json!([{"fullAccount":{"kind":"MasterEOA","margin":{"totalMargin":"100","availableMargin":"90","marginUsed":"10","notional":"25","realizedPnl":"2","unrealizedPnl":"3","fees":"-1","funding":"-2"},"positions":[position()],"openOrders":[]}}])
}
fn bytes(v: &Value) -> Vec<u8> {
    serde_json::to_vec(v).unwrap()
}

#[test]
fn account_keeps_signed_lots_exact_reported_amounts_and_display_not_basis() {
    let p = profile();
    let view = account::decode_account(&p, &bytes(&snapshot())).unwrap();
    assert_eq!(view.context, p.commitment());
    assert_eq!(view.local.get(Metric::Total).unwrap().atoms(), 100_000_000);
    assert_eq!(view.local.get(Metric::Fees).unwrap().atoms(), -1_000_000);
    assert_eq!(view.local.get(Metric::Funding).unwrap().atoms(), -2_000_000);
    assert_eq!(view.positions[0].quantity.lots(), -12);
    assert_eq!(view.positions[0].entry.text(), "100.0123456789012345");
    assert_eq!(
        view.positions[0]
            .metrics
            .get(Metric::Maintenance)
            .unwrap()
            .atoms(),
        120_000
    );
    assert_eq!(view.positions[0].average_entry_time, Some(u64::MAX));
    assert!(view.held.is_empty());
}
#[test]
fn account_local_aggregate_and_absent_metrics_are_not_summed_or_zero_filled() {
    let mut input = snapshot();
    input[0]["fullAccount"]["margin"]["aggTotalMargin"] = json!("250");
    let a = account::decode_account(&profile(), &bytes(&input)).unwrap();
    assert_eq!(a.local.get(Metric::Total).unwrap().atoms(), 100_000_000);
    let agg = a.aggregate.unwrap();
    assert_eq!(agg.get(Metric::Total).unwrap().atoms(), 250_000_000);
    assert!(agg.get(Metric::Fees).is_none());
    assert!(a.local.get(Metric::Transferable).is_none());
    assert!(a.buffer_rate.is_none());
    assert!(a.locks.is_none());
    assert!(a.children.is_none());
    input[0]["fullAccount"]["subAccounts"] = json!([]);
    input[0]["fullAccount"]["lockedAmounts"] = json!([]);
    let a = account::decode_account(&profile(), &bytes(&input)).unwrap();
    assert_eq!(a.children, Some(vec![]));
    assert_eq!(a.locks, Some(vec![]));
}
#[test]
fn account_retains_unknown_and_isolated_positions_without_erasing_known_adverse_data() {
    let mut input = snapshot();
    let mut unknown = position();
    unknown["symbol"] = json!("OTHER-USD");
    let mut isolated = position();
    isolated["iso"] = json!(true);
    input[0]["fullAccount"]["positions"] = json!([position(), unknown, isolated]);
    let a = account::decode_account(&profile(), &bytes(&input)).unwrap();
    assert_eq!(a.positions.len(), 1);
    assert_eq!(a.held.len(), 2);
    assert_eq!(a.held[0].reason, HeldReason::UnmappedMarket);
    assert_eq!(a.held[1].reason, HeldReason::IsolatedLane);
    assert_eq!(
        serde_json::from_str::<Value>(a.held[0].raw.body()).unwrap(),
        unknown
    );
    assert_eq!(format!("{:?}", a.held[0]), "HeldRow([PRIVATE])");
}
#[test]
fn exact_reported_decimal_equality_does_not_infer_atom_precision_or_basis() {
    assert_eq!(
        ReportedDecimal::new("100.1000".into()).unwrap(),
        ReportedDecimal::new("1.001e2".into()).unwrap()
    );
    assert_eq!(
        ReportedDecimal::new("0e-100".into()).unwrap(),
        ReportedDecimal::new("0".into()).unwrap()
    );
    assert!(ReportedDecimal::new("-0".into()).is_err());
    assert!(ReportedDecimal::new("NaN".into()).is_err());
    assert!(ReportedDecimal::new("1e32768".into()).is_err());
    assert_eq!(
        ReportedDecimal::new("1.123456789123456789".into())
            .unwrap()
            .text(),
        "1.123456789123456789"
    );
}
#[test]
fn malformed_missing_duplicate_and_inexact_financial_data_cannot_become_zero() {
    for field in ["totalMargin", "fees", "marginUsed"] {
        let mut input = snapshot();
        input[0]["fullAccount"]["margin"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(account::decode_account(&profile(), &bytes(&input)).is_err());
    }
    for bad in [Value::Null, json!("1e-7"), json!("-0"), json!("NaN")] {
        let mut input = snapshot();
        input[0]["fullAccount"]["margin"]["fees"] = bad;
        assert!(account::decode_account(&profile(), &bytes(&input)).is_err());
    }
    let raw = String::from_utf8(bytes(&snapshot())).unwrap();
    let duplicate = raw.replace(
        "\"totalMargin\":\"100\"",
        "\"totalMargin\":\"100\",\"totalMargin\":\"200\"",
    );
    assert!(account::decode_account(&profile(), duplicate.as_bytes()).is_err());
    let mut input = snapshot();
    input[0]["fullAccount"]["positions"] = json!([position(), position()]);
    assert!(matches!(
        account::decode_account(&profile(), &bytes(&input)),
        Err(Error::Conflict)
    ));
}
#[test]
fn account_target_parent_and_original_inventory_are_explicit() {
    let mut input = snapshot();
    input[0]["fullAccount"]["kind"] = json!("SubAccount");
    input[0]["fullAccount"]["parent"] = json!(bs58::encode([2; 32]).into_string());
    assert!(account::decode_account(&profile(), &bytes(&input)).is_err());
    let p = Profile::new(
        scope(Network::Testnet),
        unit(),
        6,
        Target::SubAccount { parent: [2; 32] },
        vec![mapping("BTC-USD", 7)],
    )
    .unwrap();
    input[0]["fullAccount"]["openOrders"] = json!([{"orderId":"retained","unknown":123}]);
    let view = account::decode_account(&p, &bytes(&input)).unwrap();
    assert_eq!(view.open_orders.len(), 1);
    input[0]["fullAccount"]["parent"] = json!(bs58::encode([3; 32]).into_string());
    assert!(account::decode_account(&p, &bytes(&input)).is_err());
}
#[test]
fn profiles_bind_units_grids_source_network_parent_and_quote_precision() {
    let p = profile();
    let mut wrong = unit();
    wrong.precision = PrecisionVersion::new(2).unwrap();
    assert!(
        Profile::new(
            scope(Network::Testnet),
            wrong,
            6,
            Target::Master,
            vec![mapping("BTC-USD", 7)]
        )
        .is_err()
    );
    assert!(matches!(
        Profile::new(
            scope(Network::Testnet),
            unit(),
            6,
            Target::Master,
            vec![mapping("BTC-USD", 7), mapping("BTC-USD", 8)]
        ),
        Err(Error::Conflict)
    ));
    assert!(matches!(
        Profile::new(
            scope(Network::Testnet),
            unit(),
            6,
            Target::Master,
            vec![mapping("BTC-USD", 7), mapping("ETH-USD", 7)]
        ),
        Err(Error::Conflict)
    ));
    let other = Profile::new(
        scope(Network::Devnet),
        unit(),
        6,
        Target::Master,
        vec![mapping("BTC-USD", 7), mapping("ETH-USD", 8)],
    )
    .unwrap();
    assert_ne!(p.commitment(), other.commitment());
    assert!(
        Profile::new(
            scope(Network::Testnet),
            unit(),
            6,
            Target::SubAccount { parent: [1; 32] },
            vec![]
        )
        .is_err()
    );
    assert_eq!(format!("{p:?}"), "FinancialProfile([PRIVATE])");
}
#[test]
fn response_inventory_and_nested_objects_remain_bounded_and_secret_safe() {
    let mut input = snapshot();
    input[0]["fullAccount"]["openOrders"] = json!(vec![json!({}); 129]);
    assert!(matches!(
        account::decode_account(&profile(), &bytes(&input)),
        Err(Error::Limit)
    ));
    assert!(
        account::decode_account(&profile(), &vec![b' '; cinder_bulk::fills::MAX_BODY + 1]).is_err()
    );
    let view = account::decode_account(&profile(), &bytes(&snapshot())).unwrap();
    assert_eq!(format!("{view:?}"), "FinancialSnapshot([PRIVATE])");
    assert_eq!(format!("{:?}", view.positions[0]), "Position([PRIVATE])");
}

fn funding_row() -> Value {
    json!({"owner":bs58::encode([1;32]).into_string(),"symbol":"BTC-USD","size":"-0.012","payment":"0.3","fundingRate":"0.0001","markPrice":"2500.123456789","iso":false,"isoPubkey":null,"slot":9007199254740993_u64,"sequence":18446744073709551615_u64,"timestamp":18446744073709551615_u64})
}
fn risk_row() -> Value {
    json!({"owner":bs58::encode([1;32]).into_string(),"symbol":"BTC-USD","isBuy":true,"amount":"0.012","price":"100.0123456789","eventType":"adl","reason":"underfill residual","marginPrior":"100","marginAfter":"20","iso":false,"slot":9007199254740993_u64,"sequence":18446744073709551615_u64,"timestamp":18446744073709551615_u64})
}
fn history_page(rows: Vec<Value>) -> Value {
    json!({"data":rows,"page":{"nextCursor":null,"hasMore":false,"asOfSlot":9007199254740994_u64,"startSlot":9007199254740993_u64,"endSlot":9007199254740993_u64,"coverage":"complete","minAvailableSlot":0}})
}
fn fee_row() -> Value {
    json!([{"feeTier":{"scopeInstrument":"global","rollingVolume":"123456.123456","tierIndex":2,"tierThreshold":"100000","makerBps":"-0.2","takerBps":"1.1","windowDays":14}}])
}

#[test]
fn funding_is_the_native_signed_payment_not_a_formula_or_a_second_accrual() {
    let p = profile();
    let page = history::funding(&p, &bytes(&history_page(vec![funding_row()]))).unwrap();
    let Observation::Known(f) = &page.rows[0] else {
        panic!("configured payment must decode")
    };
    assert_eq!(f.quantity.lots(), -12);
    assert_eq!(f.payment.atoms(), 300000);
    assert_eq!(f.rate.text(), "0.0001");
    assert_eq!(f.mark.text(), "2500.123456789");
    assert_eq!(f.identity.native.slot, 9007199254740993);
    assert_eq!(f.identity.native.sequence, u64::MAX);
    assert_eq!(f.identity.timestamp, u64::MAX);
    let mut negative = funding_row();
    negative["size"] = json!("0.012");
    negative["payment"] = json!("-0.3");
    let page = history::funding(&p, &bytes(&history_page(vec![negative]))).unwrap();
    let Observation::Known(f) = &page.rows[0] else {
        panic!()
    };
    assert_eq!(f.payment.atoms(), -300000);
}
#[test]
fn risk_preserves_actual_reported_price_flags_and_optional_margins_without_a_loss_posting() {
    let page = history::risk(&profile(), &bytes(&history_page(vec![risk_row()]))).unwrap();
    let Observation::Known(r) = &page.rows[0] else {
        panic!()
    };
    assert_eq!(r.kind, RiskKind::Adl);
    assert_eq!(r.amount, 12);
    assert!(r.native_is_buy);
    assert_eq!(r.price.text(), "100.0123456789");
    assert_eq!(r.margin_prior.unwrap().atoms(), 100_000_000);
    assert_eq!(r.margin_after.unwrap().atoms(), 20_000_000);
    assert_eq!(r.reason.as_deref(), Some("underfill residual"));
    let mut row = risk_row();
    row.as_object_mut().unwrap().remove("marginPrior");
    row["marginAfter"] = json!("0");
    row["reason"] = json!("native future explanation");
    let page = history::risk(&profile(), &bytes(&history_page(vec![row]))).unwrap();
    let Observation::Known(r) = &page.rows[0] else {
        panic!()
    };
    assert!(r.margin_prior.is_none());
    assert_eq!(r.margin_after.unwrap().atoms(), 0);
    assert_eq!(r.kind, RiskKind::Adl);
    assert_eq!(r.reason.as_deref(), Some("native future explanation"));
}
#[test]
fn risk_vault_and_unknown_types_remain_raw_not_position_quantities() {
    for kind in ["risk_vault", "future_native_event"] {
        let mut row = risk_row();
        row["eventType"] = json!(kind);
        row["amount"] = json!("not-a-qualified-lot");
        let page = history::risk(&profile(), &bytes(&history_page(vec![row.clone()]))).unwrap();
        let Observation::Held(held) = &page.rows[0] else {
            panic!()
        };
        assert_eq!(held.row.reason, HeldReason::UnsupportedRisk);
        assert_eq!(
            serde_json::from_str::<Value>(held.row.raw.body()).unwrap(),
            row
        );
    }
}
#[test]
fn unknown_and_isolated_history_are_retained_beside_configured_adverse_events() {
    let mut unknown = risk_row();
    unknown["symbol"] = json!("OTHER-USD");
    let mut known = risk_row();
    known["sequence"] = json!(u64::MAX - 1);
    let page = history::risk(&profile(), &bytes(&history_page(vec![unknown, known]))).unwrap();
    assert!(matches!(page.rows[0], Observation::Held(_)));
    assert!(matches!(page.rows[1], Observation::Known(_)));
    let mut isolated = funding_row();
    isolated["iso"] = json!(true);
    isolated["isoPubkey"] = json!(bs58::encode([9; 32]).into_string());
    let page = history::funding(&profile(), &bytes(&history_page(vec![isolated]))).unwrap();
    let Observation::Held(h) = &page.rows[0] else {
        panic!()
    };
    assert_eq!(h.row.reason, HeldReason::IsolatedLane);
}
#[test]
fn funding_keys_separate_families_networks_and_precision_without_rekeying_the_event() {
    let p = profile();
    let f = history::funding(&p, &bytes(&history_page(vec![funding_row()]))).unwrap();
    let r = history::risk(&p, &bytes(&history_page(vec![risk_row()]))).unwrap();
    let Observation::Known(f) = &f.rows[0] else {
        panic!()
    };
    let Observation::Known(r) = &r.rows[0] else {
        panic!()
    };
    assert_eq!(f.identity.native, r.identity.native);
    assert_ne!(f.identity.key, r.identity.key);
    let mut different_unit = unit();
    different_unit.precision = PrecisionVersion::new(2).unwrap();
    let mapping = Mapping::new(
        Market::new("BTC-USD", "0.001", "0.1", 6).unwrap(),
        MarketUnit {
            market: MarketId::new([7; 32]).unwrap(),
            precision: PrecisionVersion::new(2).unwrap(),
            quote: different_unit,
        },
    );
    let changed = Profile::new(
        scope(Network::Testnet),
        different_unit,
        6,
        Target::Master,
        vec![mapping],
    )
    .unwrap();
    let other = history::funding(&changed, &bytes(&history_page(vec![funding_row()]))).unwrap();
    let Observation::Known(other) = &other.rows[0] else {
        panic!()
    };
    assert_eq!(f.identity.key, other.identity.key);
    assert_ne!(f, other);
    assert_ne!(p.commitment(), changed.commitment());
    let dev = Profile::new(
        scope(Network::Devnet),
        unit(),
        6,
        Target::Master,
        vec![self::mapping("BTC-USD", 7)],
    )
    .unwrap();
    let other = history::funding(&dev, &bytes(&history_page(vec![funding_row()]))).unwrap();
    let Observation::Known(other) = &other.rows[0] else {
        panic!()
    };
    assert_ne!(f.identity.key, other.identity.key);
}
#[test]
fn known_history_duplicates_normalize_exactly_and_conflicts_do_not_disappear() {
    let a = funding_row();
    let mut equal = a.clone();
    equal["payment"] = json!("0.3000000");
    equal["fundingRate"] = json!("1e-4");
    let p = history::funding(&profile(), &bytes(&history_page(vec![a.clone(), equal]))).unwrap();
    assert_eq!(p.rows.len(), 1);
    let mut changed = a.clone();
    changed["payment"] = json!("0.4");
    assert!(matches!(
        history::funding(&profile(), &bytes(&history_page(vec![a, changed]))),
        Err(Error::Conflict)
    ));
    let mut unknown = funding_row();
    unknown["symbol"] = json!("OTHER-USD");
    let mut different = unknown.clone();
    different["payment"] = json!("0.5");
    assert!(matches!(
        history::funding(&profile(), &bytes(&history_page(vec![unknown, different]))),
        Err(Error::Conflict)
    ));
}
#[test]
fn history_pagination_coverage_owner_and_order_never_supply_completion_authority() {
    for field in ["nextCursor", "asOfSlot", "coverage"] {
        let mut p = history_page(vec![funding_row()]);
        p["page"].as_object_mut().unwrap().remove(field);
        assert!(history::funding(&profile(), &bytes(&p)).is_err());
    }
    let mut p = history_page(vec![funding_row()]);
    p["page"]["hasMore"] = json!(true);
    assert!(history::funding(&profile(), &bytes(&p)).is_err());
    let mut p = history_page(vec![funding_row()]);
    p["page"]["minAvailableSlot"] = json!(u64::MAX);
    assert!(history::funding(&profile(), &bytes(&p)).is_err());
    p["page"]["coverage"] = json!("partial");
    assert!(history::funding(&profile(), &bytes(&p)).is_ok());
    let a = funding_row();
    let mut older = a.clone();
    older["sequence"] = json!(u64::MAX - 1);
    assert!(history::funding(&profile(), &bytes(&history_page(vec![older, a]))).is_err());
    let mut wrong = funding_row();
    wrong["owner"] = json!(bs58::encode([2; 32]).into_string());
    assert!(history::funding(&profile(), &bytes(&history_page(vec![wrong]))).is_err());
    assert!(
        history::funding(&profile(), &bytes(&history_page(vec![])))
            .unwrap()
            .rows
            .is_empty()
    );
}
#[test]
fn fees_are_reported_bps_quotes_not_another_execution_charge_or_collateral() {
    let quote = fees::quote(&profile(), &bytes(&fee_row())).unwrap();
    assert_eq!(quote.scope, FeeScope::Global);
    assert_eq!(quote.rolling_volume.atoms(), 123456123456);
    assert_eq!(quote.maker_bps.text(), "-0.2");
    assert_eq!(quote.taker_bps.text(), "1.1");
    assert_eq!(quote.window_days, 14);
    let mut row = fee_row();
    row[0]["feeTier"]["scopeInstrument"] = json!("BTC-USD");
    assert!(matches!(
        fees::quote(&profile(), &bytes(&row)).unwrap().scope,
        FeeScope::Market(_)
    ));
    row[0]["feeTier"]["scopeInstrument"] = json!("OTHER-USD");
    assert!(matches!(
        fees::quote(&profile(), &bytes(&row)),
        Err(Error::Unsupported)
    ));
    let mut row = fee_row();
    row[0]["feeTier"]["rollingVolume"] = json!("-1");
    assert!(fees::quote(&profile(), &bytes(&row)).is_err());
}
#[test]
fn history_invalid_amounts_unknown_response_shapes_and_work_limits_fail_without_rounding() {
    for (field, bad) in [
        ("payment", json!("0.0000001")),
        ("size", json!("0.0001")),
        ("payment", Value::Null),
        ("markPrice", json!("0")),
    ] {
        let mut row = funding_row();
        row[field] = bad;
        assert!(history::funding(&profile(), &bytes(&history_page(vec![row]))).is_err());
    }
    assert!(history::funding(&profile(), b"{\"error\":{\"code\":\"UNAVAILABLE\"}}").is_err());
    assert!(matches!(
        history::risk(&profile(), &bytes(&history_page(vec![risk_row(); 129]))),
        Err(Error::Limit)
    ));
    let raw = String::from_utf8(bytes(&history_page(vec![funding_row()]))).unwrap();
    let raw = raw.replace(
        "\"payment\":\"0.3\"",
        "\"payment\":\"0.3\",\"payment\":\"0.4\"",
    );
    assert!(history::funding(&profile(), raw.as_bytes()).is_err());
    let page = history::funding(&profile(), &bytes(&history_page(vec![funding_row()]))).unwrap();
    assert_eq!(format!("{page:?}"), "FinancialPage([PRIVATE])");
    assert_eq!(
        format!("{:?}", page.rows[0]),
        "FinancialObservation([PRIVATE])"
    );
}

#[test]
fn profile_order_is_canonical_but_grid_changes_do_not_mint_another_source_event() {
    let reordered = Profile::new(
        scope(Network::Testnet),
        unit(),
        6,
        Target::Master,
        vec![mapping("ETH-USD", 8), mapping("BTC-USD", 7)],
    )
    .unwrap();
    assert_eq!(profile().commitment(), reordered.commitment());
    let btc = Mapping::new(
        Market::new("BTC-USD", "0.002", "0.1", 6).unwrap(),
        MarketUnit {
            market: MarketId::new([7; 32]).unwrap(),
            precision: PrecisionVersion::new(1).unwrap(),
            quote: unit(),
        },
    );
    let changed = Profile::new(
        scope(Network::Testnet),
        unit(),
        6,
        Target::Master,
        vec![btc, mapping("ETH-USD", 8)],
    )
    .unwrap();
    let body = bytes(&history_page(vec![funding_row()]));
    let a = history::funding(&profile(), &body).unwrap();
    let b = history::funding(&changed, &body).unwrap();
    let (Observation::Known(a), Observation::Known(b)) = (&a.rows[0], &b.rows[0]) else {
        panic!()
    };
    assert_eq!(a.identity.key, b.identity.key);
    assert_eq!(a.quantity.lots(), -12);
    assert_eq!(b.quantity.lots(), -6);
    assert_ne!(profile().commitment(), changed.commitment());
    assert_ne!(a, b);
}

#[test]
fn same_history_identity_on_another_configured_market_is_a_conflict_not_a_second_event() {
    let btc = funding_row();
    let mut eth = btc.clone();
    eth["symbol"] = json!("ETH-USD");
    assert!(matches!(
        history::funding(
            &profile(),
            &bytes(&history_page(vec![btc.clone(), eth.clone()]))
        ),
        Err(Error::Conflict)
    ));
    eth["sequence"] = json!(u64::MAX - 1);
    let page = history::funding(&profile(), &bytes(&history_page(vec![btc, eth]))).unwrap();
    let (Observation::Known(btc), Observation::Known(eth)) = (&page.rows[0], &page.rows[1]) else {
        panic!()
    };
    assert_ne!(btc.quantity.unit(), eth.quantity.unit());
    assert_ne!(btc.identity.key, eth.identity.key);
}

#[test]
fn raw_numeric_lexemes_flat_positions_and_nullable_isolation_are_preserved() {
    let mut input = snapshot();
    input[0]["fullAccount"]["positions"][0]["size"] = json!("0");
    input[0]["fullAccount"]["positions"][0]["price"] = json!("0");
    input[0]["fullAccount"]["positions"][0]["isoPubkey"] = Value::Null;
    let view = account::decode_account(&profile(), &bytes(&input)).unwrap();
    assert_eq!(view.positions[0].quantity.lots(), 0);
    assert_eq!(view.positions[0].entry.text(), "0");
    input[0]["fullAccount"]["positions"] = json!([]);
    assert!(
        account::decode_account(&profile(), &bytes(&input))
            .unwrap()
            .positions
            .is_empty()
    );

    let mut row = funding_row();
    row["symbol"] = json!("OTHER-USD");
    let raw = String::from_utf8(bytes(&history_page(vec![row]))).unwrap();
    let raw = raw.replace("\"payment\":\"0.3\"", "\"payment\":3.0000000000000001e-1");
    let page = history::funding(&profile(), raw.as_bytes()).unwrap();
    let Observation::Held(held) = &page.rows[0] else {
        panic!()
    };
    assert!(held.row.raw.body().contains("3.0000000000000001e-1"));
    let duplicate = raw.replace("\"iso\":false", "\"future\":1,\"future\":2,\"iso\":false");
    assert!(history::funding(&profile(), duplicate.as_bytes()).is_err());
}
