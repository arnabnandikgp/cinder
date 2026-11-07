//! Offline paper-qualification bridge to the actual native codecs. No keys,
//! signing, network, funds controller or production qualification authority.
use cinder_bulk::{
    Network, Scope, fills,
    precision::Market,
    wire::{Action, TimeInForce},
};
use cinder_kernel::identity::*;
use serde_json::{Value, json};
use std::io::{Read, stdin};

fn text<'a>(v: &'a Value, name: &str) -> &'a str {
    v[name].as_str().expect("required text")
}
fn address(s: &str) -> [u8; 32] {
    let bytes: [u8; 32] = bs58::decode(s)
        .into_vec()
        .expect("base58")
        .try_into()
        .expect("address");
    assert_eq!(bs58::encode(bytes).into_string(), s);
    bytes
}
fn main() {
    let mut input = Vec::new();
    stdin()
        .take(300_001)
        .read_to_end(&mut input)
        .expect("stdin");
    assert!(input.len() <= 300_000, "bounded input");
    let v: Value = serde_json::from_slice(&input).expect("input shape");
    let scope = Scope::new(
        EventScope {
            // Diagnostic namespacing only, never a loaded financial profile.
            domain: Domain {
                network: NetworkId::new([1; 32]).unwrap(),
                deployment: DeploymentId::new([2; 32]).unwrap(),
            },
            venue: VenueId::new([3; 32]).unwrap(),
            account: VenueAccountId::new([4; 32]).unwrap(),
            namespace: NamespaceId::new([5; 32]).unwrap(),
        },
        Network::Testnet,
        address(text(&v, "account")),
    )
    .unwrap();
    let market =
        Market::new("BTC-USD", text(&v, "size_step"), text(&v, "price_step"), 8).expect("grid");
    if v["kind"] == "fills" {
        let page = fills::rest(&scope, &[market], text(&v, "body").as_bytes())
            .expect("actual Cinder fill codec");
        println!(
            "{}",
            json!({"unmapped":page.unmapped.len(),"next_cursor":page.next_cursor,
            "fills":page.fills.iter().map(|f| json!({"order":bs58::encode(f.order).into_string(),
                "trade_id":format!("{}:{}",f.id.slot,f.id.sequence), "buy":f.buy,
                "quantity_lots":f.quantity.to_string(),"price_ticks":f.price.to_string(),
                "fee_atoms":f.fee.to_string(),"timestamp":f.timestamp.to_string()})).collect::<Vec<_>>()})
        );
        return;
    }
    let nonce = text(&v, "nonce").parse().expect("u64 nonce");
    let action = match text(&v, "kind") {
        "limit" => Action::limit(
            &market,
            v["buy"].as_bool().expect("side"),
            text(&v, "price"),
            text(&v, "size"),
            match text(&v, "tif") {
                "ALO" => TimeInForce::Alo,
                "IOC" => TimeInForce::Ioc,
                _ => panic!("unsupported paper TIF"),
            },
        )
        .expect("native action"),
        "cancel" => Action::cancel("BTC-USD", address(text(&v, "order"))).expect("native cancel"),
        _ => panic!("outside paper codec scope"),
    };
    println!(
        "{}",
        json!({"message":action.message(&scope,nonce).unwrap(),
        "action":serde_json::from_slice::<Value>(&action.json().unwrap()).unwrap(),
        "order":action.order_id(&scope,nonce).ok().map(|x|bs58::encode(x).into_string())})
    );
}
