//! Invoked by the opt-in localhost TS harness, never by normal offline CI.
use super::*;
use crate::{BoundedIntent, Journal, OrderIdentity, OrderKind};

#[test]
#[ignore = "requires evidence from the approved disposable localhost fork"]
fn native_multi_price_reversal_proof() {
    let path = std::env::var("CINDER_R5_NATIVE_EVIDENCE").expect("native fork evidence required");
    let mut evidence: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let endpoint = reqwest::Url::parse(evidence["endpoint"].as_str().unwrap()).unwrap();
    assert_eq!(endpoint.scheme(), "http");
    assert!(matches!(
        endpoint.host_str(),
        Some("127.0.0.1" | "localhost")
    ));
    let integer = |v: &Value| v.as_str().unwrap().parse::<i64>().unwrap();
    let operator = bytes(evidence["operator"].as_str().unwrap()).unwrap();
    let trader = bytes(evidence["trader"].as_str().unwrap()).unwrap();
    let program = bytes(evidence["program"].as_str().unwrap()).unwrap();
    let sig = signature(evidence["signature"].as_str().unwrap()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let mut journal = Journal::open(dir.path().join("private/journal.sqlite")).unwrap();
    let op = journal
        .prepare_intent(BoundedIntent {
            identity: OrderIdentity {
                user_ledger: [2; 32],
                user_pubkey: bytes(evidence["user"].as_str().unwrap()).unwrap(),
                user_nonce: 1,
                client_oid: serde_json::from_value(evidence["client_oid"].clone()).unwrap(),
                kind: OrderKind::User,
            },
            asset_id: 1,
            requested_lots: -integer(&evidence["quantity"]),
            limit_price_ticks: integer(&evidence["bound"]) as u64,
            last_valid_slot: integer(&evidence["deadline"]) as u64,
            post_fail_position_im_usdc: 0,
            created_at_ms: 1,
        })
        .unwrap();
    let outcome = decode_outcome(
        &op,
        &trader,
        &program,
        &operator,
        evidence["asset_id"].as_u64().unwrap() as u32,
        &sig,
        &evidence["receipt"],
    )
    .unwrap()
    .unwrap();
    let fills = match outcome {
        VenueObservation::Filled { fills } | VenueObservation::Rejected { fills } => fills,
        _ => panic!("native IOC must have a terminal, attributable outcome"),
    };
    assert!(!fills.is_empty());
    let lots = fills.iter().map(|f| f.filled_lots).sum::<i64>();
    let quote = fills.iter().map(|f| f.vwap_quote_lots).sum::<i64>();
    let fee = fills.iter().map(|f| f.fee_usdc).sum::<u64>();
    let pre = &evidence["pre_asset"];
    let post = &evidence["post_asset"];
    let before_lots = integer(&pre["baseLots"]);
    let before_basis = -integer(&pre["virtualQuoteLots"]);
    let realized = cinder_common::realize_on_fill(before_lots, lots, before_basis, quote).unwrap();
    assert_eq!(integer(&post["baseLots"]), before_lots + lots);
    assert_eq!(
        realized.new_entry_quote,
        -integer(&post["virtualQuoteLots"]),
        "aggregate acknowledgement basis must match the real native taker update"
    );
    let pre_margin = &evidence["pre_margin"];
    let post_margin = &evidence["post_margin"];
    let funding_settled = integer(&pre_margin["unsettledFundingQuoteLots"])
        - integer(&post_margin["unsettledFundingQuoteLots"]);
    assert_eq!(
        integer(&post_margin["collateralQuoteLots"]),
        integer(&pre_margin["collateralQuoteLots"]) + realized.realized_usdc + funding_settled
            - i64::try_from(fee).unwrap(),
        "native realized cash, funding and actual fees must conserve"
    );
    // Confirm the fixture really reached different maker prices; a single-
    // price reversal cannot distinguish the competing averaging algorithms.
    let receipt = Receipt::decode(&evidence["receipt"], &sig).unwrap();
    let mut raw = Vec::new();
    for group in array(&evidence["receipt"]["meta"]["innerInstructions"]).unwrap() {
        for ix in array(&group["instructions"]).unwrap() {
            if receipt.program(ix).unwrap() == program {
                let data = instruction_data(ix).unwrap();
                if PhoenixLogInstruction::from_instruction_data(&data).is_some() {
                    raw.push(data);
                }
            }
        }
    }
    let parsed = parse_with_errors(
        raw.iter()
            .filter_map(|b| PhoenixLogInstruction::from_instruction_data(b)),
    );
    assert!(parsed.failures.is_empty() && parsed.skipped_event_bytes.is_empty());
    let prices = parsed
        .events
        .iter()
        .filter_map(|e| match e {
            MarketEvent::OrderFilled(f) => Some(f.price.as_inner()),
            MarketEvent::SplineFilled(f) => Some(f.price.as_inner()),
            _ => None,
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert!(
        prices.len() > 1,
        "proof must actually fill across multiple price levels: prices={prices:?}, lots={lots}, requested={}",op.intent.requested_lots
    );
    assert!(
        before_lots > 0 && integer(&post["baseLots"]) < 0,
        "proof must actually cross zero"
    );
    evidence["native_facts"] = serde_json::json!({"lots":lots.to_string(),"quote":quote.to_string(),"fee":fee.to_string(),"fill_price":fills.iter().map(|f|f.fill_price_ticks).min().unwrap().to_string(),"realized":realized.realized_usdc.to_string(),"basis":realized.new_entry_quote.to_string()});
    // Exercise the PRODUCTION builder with persisted plans. The TS harness
    // simulates these exact instructions, rather than a second SDK builder.
    let mut config: Value = serde_json::from_str(include_str!(
        "../../../../docs/operator-config.example.json"
    ))
    .unwrap();
    config["phoenix_trader"] = evidence["trader"].clone();
    config["global_trader_index"] = evidence["indexes"].clone();
    config["active_trader_buffer"] = evidence["buffers"].clone();
    let config: crate::RuntimeConfig = serde_json::from_value(config).unwrap();
    let market = crate::IocMarketAccounts::from_asset_map(
        &config,
        1,
        bytes(&config.phoenix_asset_map).unwrap(),
        program,
        &crate::rpc::data(&evidence["asset_map_account"], &config.phoenix_program).unwrap(),
    )
    .unwrap();
    let mut instructions = Vec::new();
    for (index, direction) in [1i64, -1].into_iter().enumerate() {
        let mut intent = op.intent.clone();
        intent.identity.user_nonce += 1 + index as u64;
        intent.requested_lots = direction * 100;
        intent.limit_price_ticks = integer(
            &evidence[if direction > 0 {
                "budget_bid_price"
            } else {
                "budget_ask_price"
            }],
        ) as u64;
        let operation = journal.prepare_intent(intent).unwrap();
        let operation = journal
            .record_execution_budget(
                &operation.operation_id,
                crate::ExecutionBudget {
                    max_quote_lots: integer(&evidence["quote_budget"]) as u64,
                    // Generous fixture-only fee allowance: these simulations prove
                    // quantity/notional limits, not production fee authentication.
                    max_fee_usdc: u64::MAX,
                },
            )
            .unwrap();
        let ix =
            crate::build_bounded_ioc(&config, &operation, operator, &market, receipt.slot).unwrap();
        instructions.push(serde_json::json!({
            "programAddress": crate::runtime::text(&ix.program_id.to_bytes()),
            "data": ix.data,
            "accounts": ix.accounts.into_iter().map(|account| serde_json::json!({
                "address": crate::runtime::text(&account.pubkey.to_bytes()),
                "role": u8::from(account.is_writable) + 2 * u8::from(account.is_signer),
            })).collect::<Vec<_>>(),
        }));
    }
    evidence["budget_instructions"] = Value::Array(instructions);
    std::fs::write(path, serde_json::to_vec(&evidence).unwrap()).unwrap();
}
