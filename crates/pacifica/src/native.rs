//! Bounded typed wire schemas. Known duplicate fields/aliases reject in serde.
use serde::Deserialize;
#[derive(Clone, Deserialize)]
pub(crate) struct Rest<T> {
    pub success: bool,
    pub data: T,
    pub error: Option<String>,
    pub has_more: Option<bool>,
    pub next_cursor: Option<String>,
    pub last_order_id: Option<u64>,
}
#[derive(Clone, Deserialize)]
pub(crate) struct Ws<T> {
    pub channel: String,
    pub data: T,
    pub li: Option<u64>,
}
#[derive(Clone, Deserialize)]
pub(crate) struct Trade {
    #[serde(alias = "h")]
    pub history_id: u64,
    #[serde(alias = "i")]
    pub order_id: u64,
    #[serde(alias = "I")]
    pub client_order_id: Option<String>,
    #[serde(alias = "u")]
    pub account: Option<String>,
    #[serde(alias = "s")]
    pub symbol: String,
    #[serde(alias = "p")]
    pub price: String,
    #[serde(alias = "o")]
    pub entry_price: String,
    #[serde(alias = "a")]
    pub amount: String,
    #[serde(alias = "f")]
    pub fee: String,
    #[serde(alias = "n")]
    pub pnl: String,
    #[serde(alias = "te")]
    pub event_type: String,
    #[serde(alias = "ts")]
    pub side: String,
    #[serde(alias = "tc")]
    pub cause: String,
    #[serde(alias = "t")]
    pub created_at: u64,
    pub li: Option<u64>,
    #[serde(alias = "it")]
    pub instrument_type: Option<u64>,
    pub spot_fee: Option<String>,
}
#[derive(Clone, Deserialize)]
pub(crate) struct NativePosition {
    #[serde(alias = "s")]
    pub symbol: String,
    #[serde(alias = "d")]
    pub side: String,
    #[serde(alias = "a")]
    pub amount: String,
    #[serde(alias = "p")]
    pub entry_price: String,
    #[serde(alias = "f")]
    pub funding: String,
    #[serde(alias = "i")]
    pub isolated: bool,
}
#[derive(Clone, Deserialize)]
pub(crate) struct Funding {
    pub history_id: u64,
    pub symbol: String,
    pub side: String,
    pub amount: String,
    pub payout: String,
    pub rate: String,
    pub created_at: u64,
}
#[derive(Clone, Deserialize)]
pub(crate) struct Account {
    pub balance: String,
    pub account_equity: String,
    pub available_to_spend: String,
    pub available_to_withdraw: String,
    pub pending_balance: String,
    pub updated_at: u64,
}
#[derive(Clone, Deserialize)]
pub(crate) struct Order {
    pub order_id: u64,
    pub client_order_id: Option<String>,
    pub symbol: String,
    pub side: String,
    pub amount: String,
    pub filled_amount: String,
    pub order_status: String,
    pub updated_at: u64,
}
