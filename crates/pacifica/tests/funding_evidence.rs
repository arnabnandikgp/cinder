//! Documented wire components only. No sockets, source-cut or payment certificate.
#[path = "../../journal/tests/support/mod.rs"]
mod support;
use cinder_pacifica::{
    execution::Origin,
    funding::evidence::{self, TransferObservation},
    profile::{Grid, Level, Mapping, Profile},
};
use serde_json::{Value, json};
use support::*;

fn profile() -> Profile {
    Profile {
        config: config(),
        source: config().sources[0].scope,
        account: bs58::encode([42; 32]).into_string(),
        environment: Origin::Testnet.url().into(),
        revision: 1,
        evidence: "offline funding wire fixture only".into(),
        precision: Level::Documented,
        fills: Level::Documented,
        quote_places: 6,
        perp_tag: 0,
        markets: vec![Mapping {
            symbol: "BTC".into(),
            market: 0,
            size: Grid { places: 3, step: 1 },
            price: Grid { places: 3, step: 1 },
        }],
    }
}
fn transfer(event: &str) -> Value {
    json!({"channel":"account_transfers","data":{
        "u":profile().account,"e":event,"a":"USDP","am":"9.000001","t":100,
        "tx":bs58::encode([7;64]).into_string()
    }})
}
fn decode(v: &Value) -> Result<TransferObservation, cinder_pacifica::Error> {
    evidence::transfer(
        &profile(),
        "USDP",
        &serde_json::to_vec(v).unwrap(),
        101,
        1000,
    )
}
#[test]
fn links_exact_deposit_and_withdrawal_components_without_certifying_payment() {
    assert_eq!(
        decode(&transfer("deposit")).unwrap(),
        TransferObservation::Deposit {
            signature: [7; 64],
            amount: 9_000_001,
            at: 100
        }
    );
    let mut v = transfer("withdrawal_confirmed");
    v["data"]["bn"] = json!(42);
    v["data"]["ra"] = json!("10.000001");
    v["data"]["f"] = json!("1.000000");
    let observation = decode(&v).unwrap();
    assert_eq!(
        observation,
        TransferObservation::Withdrawal {
            signature: Some([7; 64]),
            batch: 42,
            amount: 9_000_001,
            requested: 10_000_001,
            fee: 1_000_000,
            at: 100,
            confirmed: true
        }
    );
    assert_eq!(format!("{observation:?}"), "TransferObservation([PRIVATE])");
    v["data"]["e"] = json!("withdrawal_pending");
    v["data"].as_object_mut().unwrap().remove("tx");
    assert!(matches!(
        decode(&v).unwrap(),
        TransferObservation::Withdrawal {
            signature: None,
            confirmed: false,
            ..
        }
    ));
    v["data"]["e"] = json!("withdrawal_confirmed");
    assert!(decode(&v).is_err());
}
#[test]
fn rejects_wrong_scopes_missing_linkage_null_aliases_and_unsafe_amounts() {
    for (field, value) in [
        ("u", json!(bs58::encode([41; 32]).into_string())),
        ("a", json!("USDC")),
        ("e", json!("subaccount_transfer")),
        ("tx", Value::Null),
        ("tx", json!(bs58::encode([0; 64]).into_string())),
        ("tx", json!("not-a-signature")),
        ("am", json!("1e1")),
        ("am", json!("-1")),
        ("am", json!("0")),
        ("am", json!("1.0000001")),
        ("am", json!("18446744073710.000000")),
        ("t", json!(0)),
        ("t", json!(102)),
        ("bn", Value::Null),
        ("s", Value::Null),
        ("unknown", json!(1)),
    ] {
        let mut v = transfer("deposit");
        v["data"][field] = value;
        assert!(decode(&v).is_err(), "accepted {field}");
    }
    for field in ["u", "e", "a", "am", "t", "tx"] {
        let mut v = transfer("deposit");
        v["data"].as_object_mut().unwrap().remove(field);
        assert!(decode(&v).is_err(), "accepted missing {field}");
    }
    let v = serde_json::to_vec(&transfer("deposit")).unwrap();
    for (now, age) in [(99, 1000), (1101, 1000), (101, 0), (101, 60_001)] {
        assert!(evidence::transfer(&profile(), "USDP", &v, now, age).is_err());
    }
    assert!(evidence::transfer(&profile(), "usdp", &v, 101, 1000).is_err());
    assert!(evidence::transfer(&profile(), "USDP", &vec![0; 65_537], 101, 1000).is_err());
    let duplicate = String::from_utf8(v)
        .unwrap()
        .replace("\"t\":100", "\"t\":100,\"t\":100");
    assert!(evidence::transfer(&profile(), "USDP", duplicate.as_bytes(), 101, 1000).is_err());
    let mut withdrawal = transfer("withdrawal_confirmed");
    withdrawal["data"]["bn"] = json!(42);
    withdrawal["data"]["ra"] = json!("10.000001");
    withdrawal["data"]["f"] = json!("1");
    for field in ["bn", "ra", "f", "tx"] {
        let mut v = withdrawal.clone();
        v["data"].as_object_mut().unwrap().remove(field);
        assert!(decode(&v).is_err());
    }
    withdrawal["data"]["ra"] = json!("10");
    assert!(decode(&withdrawal).is_err());
}
#[test]
fn setup_observes_defaults_debt_and_incompatible_margin_without_promoting_authority() {
    let settings = json!({"success":true,"data":{"auto_lend_disabled":true,"margin_settings":[]},"error":null});
    let loan = json!({"success":true,"data":{"borrowed":"0","pending_interest":"0.000001","updated_at":100},"error":null});
    let parse = |s: &Value, l: &Value| {
        evidence::setup(
            &profile(),
            &serde_json::to_vec(s).unwrap(),
            &serde_json::to_vec(l).unwrap(),
        )
    };
    let got = parse(&settings, &loan).unwrap();
    assert!(got.lending_disabled && got.compatible_margin);
    assert_eq!((got.borrowed, got.interest), (0, 1));
    assert_eq!(got.loan_at, 100);
    assert_eq!(format!("{got:?}"), "SetupObservation([PRIVATE])");
    for value in [Value::Null, json!(false)] {
        let mut s = settings.clone();
        s["data"]["auto_lend_disabled"] = value;
        assert!(!parse(&s, &loan).unwrap().lending_disabled);
    }
    let mut s = settings.clone();
    s["data"]
        .as_object_mut()
        .unwrap()
        .remove("auto_lend_disabled");
    assert!(!parse(&s, &loan).unwrap().lending_disabled);
    for margins in [
        json!([{"symbol":"BTC","isolated":true,"leverage":2}]),
        json!([{"symbol":"BTC","isolated":false,"leverage":0}]),
        json!([{"symbol":"ETH","isolated":false,"leverage":2}]),
        json!([{"symbol":"BTC","isolated":false,"leverage":2},{"symbol":"BTC","isolated":false,"leverage":2}]),
    ] {
        let mut s = settings.clone();
        s["data"]["margin_settings"] = margins;
        assert!(!parse(&s, &loan).unwrap().compatible_margin);
    }
    for field in ["borrowed", "pending_interest"] {
        let mut l = loan.clone();
        l["data"].as_object_mut().unwrap().remove(field);
        assert!(parse(&settings, &l).is_err());
        for value in [Value::Null, json!("-1"), json!("1e1"), json!("0.0000001")] {
            let mut l = loan.clone();
            l["data"][field] = value;
            assert!(parse(&settings, &l).is_err());
        }
    }
    let mut l = loan.clone();
    l["data"] = Value::Null;
    assert!(parse(&settings, &l).is_err());
    for value in [Value::Null, json!(0), json!(-1), json!("100")] {
        let mut l = loan.clone();
        l["data"]["updated_at"] = value;
        assert!(parse(&settings, &l).is_err());
    }
    let mut l = loan.clone();
    l["data"].as_object_mut().unwrap().remove("updated_at");
    assert!(parse(&settings, &l).is_err());
    for (field, value) in [("error", json!("setup incomplete")), ("code", json!(400))] {
        let mut s = settings.clone();
        s["data"][field] = value;
        assert!(parse(&s, &loan).is_err());
    }
    let mut s = settings.clone();
    s["success"] = json!(false);
    assert!(parse(&s, &loan).is_err());
    let mut l = loan.clone();
    l["error"] = json!("missing cache");
    assert!(parse(&settings, &l).is_err());
}
