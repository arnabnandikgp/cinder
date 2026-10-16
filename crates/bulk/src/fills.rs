//! Exact scoped fills; page coverage metadata never certifies finality or credit.
use crate::{Error, Scope, address, precision::Market, symbol};
use serde::Deserialize;
use serde_json::value::RawValue;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Work bound for these observation codecs, independent of upstream page defaults.
pub const MAX_BODY: usize = 256 * 1024;
/// Maximum normalized executions in one bounded page.
pub const MAX_ROWS: usize = 128;
/// Bound the configured symbol lookup independently of response size.
pub const MAX_MARKETS: usize = 128;

/// Network-local execution identity; gaps in an account's sequence are legitimate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TradeId {
    /// Native slot.
    pub slot: u64,
    /// Native event sequence, not a contiguous account sequence.
    pub sequence: u64,
}
impl TradeId {
    /// Require lossless canonical u64:u64 text; never derive an absent ID.
    pub fn parse(text: &str) -> Result<Self, Error> {
        let (a, b) = text.split_once(':').ok_or(Error::Invalid)?;
        let number = |s: &str| {
            if s.is_empty()
                || (s.len() > 1 && s.starts_with('0'))
                || !s.bytes().all(|b| b.is_ascii_digit())
            {
                return Err(Error::Invalid);
            }
            s.parse::<u64>().map_err(|_| Error::Limit)
        };
        Ok(Self {
            slot: number(a)?,
            sequence: number(b)?,
        })
    }
    /// Namespace an ID across Cinder source/deployment, venue, account and network.
    pub fn scoped(&self, scope: &Scope) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(b"CINDER-BULK-FILL-1\0");
        h.update(scope.commitment());
        h.update(self.slot.to_le_bytes());
        h.update(self.sequence.to_le_bytes());
        h.finalize().into()
    }
}

/// Native reason only; a code does not itself authorize Cinder ADL/restoration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// Normal execution.
    Trade,
    /// Native liquidation.
    Liquidation,
    /// Native ADL; financial mapping still requires qualified evidence.
    Adl,
    /// Native liquidation sweep.
    LiquidationSweep,
}
fn reason(code: u8, label: Option<&str>) -> Result<Reason, Error> {
    let (value, expected) = match code {
        0 => (Reason::Trade, None),
        1 => (Reason::Liquidation, Some("liquidation")),
        2 => (Reason::Adl, Some("adl")),
        3 => (Reason::LiquidationSweep, Some("liquidation_sweep")),
        _ => return Err(Error::Unsupported),
    };
    if label.is_some() && label != expected {
        return Err(Error::Conflict);
    }
    Ok(value)
}

/// Native execution view, never a financial posting or independent balance book.
#[derive(Clone, PartialEq, Eq)]
pub struct Fill {
    /// Exact instrument/grids/quote precision used for normalization. A changed
    /// mapping cannot conceal itself as an equal observation under the same ID.
    pub market: Market,
    /// Exact network-local ID.
    pub id: TradeId,
    /// Scoped semantic deduplication identity.
    pub key: [u8; 32],
    /// This account's native order, not the counterparty order.
    pub order: [u8; 32],
    /// Account direction, normalized independently for REST versus WS.
    pub buy: bool,
    /// Positive configured quantity lots.
    pub quantity: u64,
    /// Positive exact execution ticks.
    pub price: u64,
    /// Signed effective fee; negative is a rebate. No PnL calculation is implied.
    pub fee: i128,
    /// Native event classification.
    pub reason: Reason,
    /// Native nanosecond timestamp, not a Cinder clock or finality frontier.
    pub timestamp: u64,
}
impl std::fmt::Debug for Fill {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Fill([PRIVATE])")
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Row {
    trade_id: String,
    maker: String,
    taker: String,
    order_id_maker: String,
    order_id_taker: String,
    is_buy: bool,
    symbol: String,
    amount: Box<RawValue>,
    price: Box<RawValue>,
    maker_fee: Box<RawValue>,
    taker_fee: Box<RawValue>,
    fee: Box<RawValue>,
    reason_code: u8,
    reason: Option<String>,
    iso: bool,
    iso_pubkey: Option<String>,
    slot: u64,
    sequence: u64,
    timestamp: u64,
}
fn scalar(raw: &RawValue) -> Result<String, Error> {
    if raw.get().starts_with('"') {
        serde_json::from_str(raw.get()).map_err(|_| Error::Invalid)
    } else {
        Ok(raw.get().into())
    }
}
fn check_body(body: &[u8]) -> Result<(), Error> {
    if body.is_empty() || body.len() > MAX_BODY {
        Err(Error::Limit)
    } else {
        Ok(())
    }
}
/// Validate source identity even when economic units are not configured.
fn row_identity(scope: &Scope, r: &Row) -> Result<(TradeId, bool), Error> {
    symbol(&r.symbol)?;
    let maker = address(&r.maker)?;
    let taker = address(&r.taker)?;
    if (maker == scope.account) == (taker == scope.account) {
        return Err(Error::Invalid);
    }
    let id = TradeId::parse(&r.trade_id)?;
    if id.slot != r.slot || id.sequence != r.sequence {
        return Err(Error::Conflict);
    }
    Ok((id, maker == scope.account))
}
fn row(scope: &Scope, market: &Market, r: Row) -> Result<Fill, Error> {
    if r.symbol != market.symbol {
        return Err(Error::Invalid);
    }
    if r.iso || r.iso_pubkey.is_some() {
        return Err(Error::Unsupported);
    }
    let (id, own_maker) = row_identity(scope, &r)?;
    let fee = market.fee(&scalar(&r.fee)?)?;
    let maker_fee = market.fee(&scalar(&r.maker_fee)?)?;
    let taker_fee = market.fee(&scalar(&r.taker_fee)?)?;
    if fee != if own_maker { maker_fee } else { taker_fee } {
        return Err(Error::Conflict);
    }
    Ok(Fill {
        market: market.clone(),
        id,
        key: id.scoped(scope),
        order: address(if own_maker {
            &r.order_id_maker
        } else {
            &r.order_id_taker
        })?,
        buy: r.is_buy,
        quantity: market.quantity(&scalar(&r.amount)?)?,
        price: market.price(&scalar(&r.price)?)?,
        fee,
        reason: reason(r.reason_code, r.reason.as_deref())?,
        timestamp: r.timestamp,
    })
}

/// Coverage reported by the venue, not independently qualified causal evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Coverage {
    /// Reported complete for the effective page range only.
    Complete,
    /// Reported partial.
    Partial,
    /// Reported unknown.
    Unknown,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageInfo {
    next_cursor: Box<RawValue>,
    has_more: bool,
    as_of_slot: u64,
    start_slot: u64,
    end_slot: u64,
    coverage: Coverage,
    min_available_slot: Option<u64>,
}
#[derive(Deserialize)]
struct NativePage {
    data: Vec<Box<RawValue>>,
    page: PageInfo,
}
/// A source-attributed row without configured economic units. It is not a fill
/// posting or proof of absence; persist it and contain dependent reconciliation.
pub struct UnmappedFill {
    /// Original network-local identity, checked against row slot/sequence.
    pub id: TradeId,
    /// Same scoped identity used when a mapping becomes available; never re-credit.
    pub key: [u8; 32],
    /// Exact native symbol whose mapping is unavailable.
    pub symbol: String,
    raw: Box<RawValue>,
}
impl UnmappedFill {
    /// Original row JSON, including numeric lexemes and unknown fields. Private
    /// evidence, not log output or an authenticated observation by itself.
    pub fn body(&self) -> &str {
        self.raw.get()
    }
}
impl std::fmt::Debug for UnmappedFill {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("UnmappedFill([PRIVATE])")
    }
}
/// Bounded page observations. Even an empty complete page cannot release a hold.
pub struct Page {
    /// Distinct exact executions in source order; equal duplicates collapse.
    pub fills: Vec<Fill>,
    /// Unmapped rows retained separately, not skipped or counted as reconciled.
    /// Nonempty inventory blocks any claim of complete economic reconciliation.
    pub unmapped: Vec<UnmappedFill>,
    /// Opaque continuation, never reconstructed from row timestamps.
    pub next_cursor: Option<String>,
    /// Native-reported range coverage, never a Cinder settlement certificate.
    pub coverage: Coverage,
    /// Native reported observation frontier, not a complete economic frontier.
    pub as_of_slot: u64,
    /// Effective inclusive range start; must still match the retained request.
    pub start_slot: u64,
    /// Effective inclusive range end.
    pub end_slot: u64,
    /// Native reported retention floor; absence remains unknown.
    pub min_available_slot: Option<u64>,
}
/// Decode an account-wide REST page using a bounded unique symbol mapping.
/// Known markets normalize independently; unmapped rows retain original bytes.
/// Caller must authenticate/retain the original body and request/range/cursor;
/// this codec does not prove that the requested scan or original funding completed.
pub fn rest(scope: &Scope, markets: &[Market], body: &[u8]) -> Result<Page, Error> {
    check_body(body)?;
    if markets.len() > MAX_MARKETS {
        return Err(Error::Limit);
    }
    let mut by_symbol = BTreeMap::new();
    for market in markets {
        if by_symbol.insert(market.symbol.as_str(), market).is_some() {
            return Err(Error::Conflict);
        }
    }
    let p: NativePage = serde_json::from_slice(body).map_err(|_| Error::Invalid)?;
    if p.data.len() > MAX_ROWS {
        return Err(Error::Limit);
    }
    let cursor: Option<String> =
        serde_json::from_str(p.page.next_cursor.get()).map_err(|_| Error::Invalid)?;
    if p.page.has_more != cursor.is_some()
        || cursor
            .as_ref()
            .is_some_and(|c| c.is_empty() || c.len() > 1024)
        || p.page.start_slot > p.page.end_slot
        || p.page.end_slot > p.page.as_of_slot
        || (p.page.coverage == Coverage::Complete
            && p.page
                .min_available_slot
                .is_some_and(|s| s > p.page.start_slot))
    {
        return Err(Error::Invalid);
    }
    let mut seen = BTreeMap::new();
    let mut seen_unmapped = BTreeMap::<TradeId, usize>::new();
    let mut fills = Vec::new();
    let mut unmapped = Vec::<UnmappedFill>::new();
    let mut previous = None;
    for raw in p.data {
        let r: Row = serde_json::from_str(raw.get()).map_err(|_| Error::Invalid)?;
        if r.slot < p.page.start_slot || r.slot > p.page.end_slot {
            return Err(Error::Invalid);
        }
        let (id, _) = row_identity(scope, &r)?;
        if previous.is_some_and(|old| id > old) {
            return Err(Error::Invalid);
        }
        previous = Some(id);
        let Some(market) = by_symbol.get(r.symbol.as_str()) else {
            if seen.contains_key(&id) {
                return Err(Error::Conflict);
            }
            if let Some(index) = seen_unmapped.get(&id) {
                if unmapped[*index].body() != raw.get() {
                    return Err(Error::Conflict);
                }
            } else {
                seen_unmapped.insert(id, unmapped.len());
                unmapped.push(UnmappedFill {
                    id,
                    key: id.scoped(scope),
                    symbol: r.symbol,
                    raw,
                });
            }
            continue;
        };
        if seen_unmapped.contains_key(&id) {
            return Err(Error::Conflict);
        }
        let fill = row(scope, market, r)?;
        match seen.get(&fill.id) {
            Some(old) if old != &fill => return Err(Error::Conflict),
            Some(_) => {}
            None => {
                seen.insert(fill.id, fill.clone());
                fills.push(fill);
            }
        }
    }
    Ok(Page {
        fills,
        unmapped,
        next_cursor: cursor,
        coverage: p.page.coverage,
        as_of_slot: p.page.as_of_slot,
        start_slot: p.page.start_slot,
        end_slot: p.page.end_slot,
        min_available_slot: p.page.min_available_slot,
    })
}

#[derive(Deserialize)]
struct Stream {
    r#type: String,
    topic: String,
    data: StreamFill,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StreamFill {
    r#type: String,
    trade_id: String,
    symbol: String,
    order_id: String,
    price: Box<RawValue>,
    size: Box<RawValue>,
    fee: Box<RawValue>,
    is_buy: bool,
    maker: bool,
    reason_code: u8,
    reason: Option<String>,
    timestamp: u64,
    iso: Option<bool>,
    iso_pubkey: Option<String>,
}
/// Decode account WebSocket fill shape. `isBuy` is taker direction here, unlike
/// REST history. Isolated rows and unknown event/reason semantics are unsupported.
pub fn stream(scope: &Scope, market: &Market, body: &[u8]) -> Result<Fill, Error> {
    check_body(body)?;
    let r: Stream = serde_json::from_slice(body).map_err(|_| Error::Invalid)?;
    if r.r#type != "account"
        || r.topic != format!("account.{}", bs58::encode(scope.account).into_string())
        || r.data.r#type != "fill"
        || r.data.symbol != market.symbol
    {
        return Err(Error::Invalid);
    }
    let d = r.data;
    if d.iso == Some(true) || d.iso_pubkey.is_some() {
        return Err(Error::Unsupported);
    }
    let id = TradeId::parse(&d.trade_id)?;
    Ok(Fill {
        market: market.clone(),
        id,
        key: id.scoped(scope),
        order: address(&d.order_id)?,
        buy: if d.maker { !d.is_buy } else { d.is_buy },
        quantity: market.quantity(&scalar(&d.size)?)?,
        price: market.price(&scalar(&d.price)?)?,
        fee: market.fee(&scalar(&d.fee)?)?,
        reason: reason(d.reason_code, d.reason.as_deref())?,
        timestamp: d.timestamp,
    })
}
