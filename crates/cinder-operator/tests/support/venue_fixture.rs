//! Synthetic finalized RPC receipt, encoded by the official Rise SDK. Only
//! test processes use this module; it is not a production finality source.
use phoenix_rise_events::{
    market_events::*, MarketEvent, OffChainMarketEvent, OffChainMarketEventLengths,
    LOG_EVENT_LENGTHS_INSTRUCTION_TAG, LOG_INSTRUCTION_TAG,
};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize, Clone)]
pub struct VenueFixture {
    pub trader: String,
    pub operator: String,
    pub program: String,
    pub oid: [u8; 16],
    pub native_asset_id: u32,
    pub lots: i64,
    #[serde(default)]
    pub filled_lots: Option<i64>,
    #[serde(default)]
    pub max_quote_lots: Option<u64>,
    pub ticks: u64,
    pub bound: u64,
    pub deadline: u64,
    pub fee: u64,
    pub time: u64,
    pub slot: u64,
    pub signature: String,
    pub rejected: bool,
}
fn key(value: &str) -> [u8; 32] {
    bs58::decode(value).into_vec().unwrap().try_into().unwrap()
}
pub fn receipt(f: &VenueFixture) -> Value {
    let trader = key(&f.trader);
    let operator = key(&f.operator);
    let side = if f.lots > 0 { Side::Bid } else { Side::Ask };
    let opposite = if f.lots > 0 { Side::Ask } else { Side::Bid };
    let filled = f.filled_lots.unwrap_or(f.lots);
    let quote = filled
        .unsigned_abs()
        .checked_mul(f.ticks)
        .unwrap()
        .checked_mul(100)
        .unwrap();
    let mut events = vec![
        MarketEvent::SlotContext(SlotContextEvent {
            timestamp: f.time,
            slot: f.slot,
        }),
        MarketEvent::Header(MarketEventHeader {
            sequence_number: 1,
            prev_sequence_number_slot: 0,
            asset_symbol: Symbol::new("SOL").unwrap(),
            asset_id: f.native_asset_id,
            tick_size: 100,
            base_lot_decimals: 2,
            quote_lot_decimals: 6,
            signer: operator,
            trader_account: trader,
        }),
        MarketEvent::OrderPacket(OrderPacketEvent {
            trader,
            next_order_sequence_number: 1,
            order_packet: OrderPacket::new(OrderPacketKind::ImmediateOrCancel {
                side,
                price_in_ticks: Some(Ticks::new(f.bound)),
                num_base_lots: BaseLots::new(f.lots.unsigned_abs()),
                num_quote_lots: f.max_quote_lots.map(QuoteLots::new),
                min_base_lots_to_fill: BaseLots::new(0),
                min_quote_lots_to_fill: QuoteLots::new(0),
                self_trade_behavior: SelfTradeBehavior::Abort,
                match_limit: None,
                client_order_id: f.oid,
                last_valid_slot: Some(f.deadline),
                order_flags: OrderFlags::from_bits(0),
                cancel_existing: false,
            }),
        }),
    ];
    if !f.rejected {
        events.push(MarketEvent::OrderFilled(OrderFilledEvent {
            order_sequence_number: 1,
            side: opposite,
            price: Ticks::new(f.ticks),
            base_lots_filled: BaseLots::new(filled.unsigned_abs()),
            quote_lots_filled: QuoteLots::new(quote),
            quantity_remaining: BaseLots::new(0),
            maker: [99; 32],
            maker_fee_rate: SignedFeeRateMicro::new(0),
            maker_base_lot_position: SignedBaseLots::new(0),
            maker_virtual_quote_lot_position: SignedQuoteLots::new(0),
            maker_quote_lot_collateral: SignedQuoteLots::new(0),
            maker_cumulative_funding_snapshot: SignedQuoteLotsPerBaseLot::new(0),
        }));
        events.push(MarketEvent::TradeSummary(TradeSummaryEvent {
            trader,
            trade_sequence_number: 1,
            prev_trade_sequence_number_slot: 0,
            side,
            base_lots_filled: BaseLots::new(filled.unsigned_abs()),
            quote_lots_filled: QuoteLots::new(quote),
            fee_in_quote_lots: QuoteLots::new(f.fee),
            base_lot_position: SignedBaseLots::new(filled),
            virtual_quote_lot_position: SignedQuoteLots::new(0),
            quote_lot_collateral: SignedQuoteLots::new(0),
            cumulative_funding_snapshot: SignedQuoteLotsPerBaseLot::new(0),
        }));
    }
    let lengths = events
        .iter()
        .map(|e| u16::try_from(borsh::to_vec(e).unwrap().len()).unwrap())
        .collect();
    let mut log = LOG_INSTRUCTION_TAG.to_le_bytes().to_vec();
    log.extend(
        borsh::to_vec(&OffChainMarketEvent {
            batch_index: 0,
            events,
        })
        .unwrap(),
    );
    let mut len = LOG_EVENT_LENGTHS_INSTRUCTION_TAG.to_le_bytes().to_vec();
    len.extend(
        borsh::to_vec(&OffChainMarketEventLengths {
            batch_index: 0,
            lengths,
        })
        .unwrap(),
    );
    json!({"slot":f.slot,"blockTime":f.time,"transaction":{"signatures":[f.signature],"message":{"accountKeys":[f.operator,f.trader,f.program],"header":{"numRequiredSignatures":1,"numReadonlySignedAccounts":0,"numReadonlyUnsignedAccounts":1},"instructions":[{"programIdIndex":2,"accounts":[0,1],"data":bs58::encode([1u8;8]).into_string()}]}},"meta":{"err":null,"innerInstructions":[{"index":0,"instructions":[{"programIdIndex":2,"accounts":[],"data":bs58::encode(len).into_string()},{"programIdIndex":2,"accounts":[],"data":bs58::encode(log).into_string()}]}],"loadedAddresses":{"writable":[],"readonly":[]}}})
}
