//! Native funding/risk history facts, never independent financial-posting authority.
use super::*;
use crate::{
    address,
    fills::{Coverage, MAX_ROWS, TradeId},
    raw::{Object, decode},
    symbol,
};
use serde_json::value::RawValue;
use std::collections::BTreeMap;

/// Native reported coverage. It does not certify a complete economic frontier.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cut {
    /// Original opaque continuation, not derived from timestamps or row IDs.
    pub next_cursor: Option<String>,
    /// Native observation boundary, not a Cinder settlement boundary.
    pub as_of_slot: u64,
    /// Inclusive source range.
    pub start_slot: u64,
    /// Inclusive source range end.
    pub end_slot: u64,
    /// Native reported completeness only.
    pub coverage: Coverage,
    /// Unknown retention floor remains absent.
    pub min_available_slot: Option<u64>,
}
impl Cut {
    fn parse(row: Object) -> Result<Self, Error> {
        let next_cursor: Option<String> = row.get("nextCursor")?;
        let more: bool = row.get("hasMore")?;
        let result = Self {
            next_cursor,
            as_of_slot: row.get("asOfSlot")?,
            start_slot: row.get("startSlot")?,
            end_slot: row.get("endSlot")?,
            coverage: row.get("coverage")?,
            min_available_slot: row.optional::<Option<u64>>("minAvailableSlot")?.flatten(),
        };
        if more != result.next_cursor.is_some()
            || result
                .next_cursor
                .as_ref()
                .is_some_and(|v| v.is_empty() || v.len() > 1024)
            || result.start_slot > result.end_slot
            || result.end_slot > result.as_of_slot
            || (result.coverage == Coverage::Complete
                && result
                    .min_available_slot
                    .is_some_and(|s| s > result.start_slot))
        {
            return Err(Error::Invalid);
        }
        Ok(result)
    }
}
/// Family-scoped source observation identity; not a ledger event or proof.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identity {
    /// Native slot/sequence, not a contiguous per-account event count.
    pub native: TradeId,
    /// Source/account/network/family-scoped key; mapping revisions do not re-key it.
    pub key: [u8; 32],
    /// Native nanosecond time; not the client receive clock.
    pub timestamp: u64,
}
/// Original held history row with identity retained for later resolution.
#[derive(Clone, PartialEq, Eq)]
pub struct HeldEvent {
    /// Same identity as a later correctly interpreted observation, not a new credit.
    pub identity: Identity,
    /// Exact unsupported evidence, preserved privately.
    pub row: HeldRow,
}
impl std::fmt::Debug for HeldEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HeldEvent([PRIVATE])")
    }
}
/// A history row is either understood under configured units or explicitly held.
#[derive(Clone, PartialEq, Eq)]
pub enum Observation<T> {
    /// Normalized native fact, still not a financial posting/readiness certificate.
    Known(T),
    /// Unresolved native fact, not an omitted/zero economic event.
    Held(HeldEvent),
}
impl<T> std::fmt::Debug for Observation<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("FinancialObservation([PRIVATE])")
    }
}
/// One bounded source response, preserving interleaved known and held row order.
#[derive(Clone, PartialEq, Eq)]
pub struct Page<T> {
    /// Configured interpretation commitment, not read from native JSON.
    pub context: [u8; 32],
    /// Original body commitment, independent of numeric normalization.
    pub body_hash: [u8; 32],
    /// Original row order; equal duplicates collapse without skipping conflicts.
    pub rows: Vec<Observation<T>>,
    /// Source-reported pagination/coverage; never sufficient to release a hold.
    pub cut: Cut,
}
impl<T> std::fmt::Debug for Page<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("FinancialPage([PRIVATE])")
    }
}
/// A native payment already applied to venue margin, not another funding accrual.
#[derive(Clone, PartialEq, Eq)]
pub struct Funding {
    /// Scoped native record identity, independent of current mapping precision.
    pub identity: Identity,
    /// Native signed boundary quantity under configured units, including explicit zero.
    pub quantity: QuantityLots,
    /// Actual signed USD payment: positive received, negative paid.
    pub payment: QuoteAtoms,
    /// Reported rate, not independently qualified for gross private allocation.
    pub rate: ReportedDecimal,
    /// Reported funding mark, not a current admission oracle.
    pub mark: ReportedDecimal,
}
impl std::fmt::Debug for Funding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Funding([PRIVATE])")
    }
}
/// Native risk family only, not Cinder customer liquidation versus pool distress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskKind {
    /// Native liquidation report.
    Liquidation,
    /// Native ADL report; reason/economic linkage still require qualification.
    Adl,
}
/// Reported forced reduction; no fill, cash loss or restoration is synthesized.
#[derive(Clone, PartialEq, Eq)]
pub struct Risk {
    /// Original native identity.
    pub identity: Identity,
    /// Explicit configured market denomination.
    pub market: MarketUnit,
    /// Absolute reported reduction lots, not direction inferred from a position.
    pub amount: u64,
    /// Original native side flag; no undocumented REST/WS sign conversion.
    pub native_is_buy: bool,
    /// Reported executed price; may differ from resting tick or average entry.
    pub price: ReportedDecimal,
    /// Native classification, not Cinder financial-loss eligibility.
    pub kind: RiskKind,
    /// Optional native margin before, not an independent asset receipt.
    pub margin_prior: Option<QuoteAtoms>,
    /// Optional margin after; its difference is not automatically another loss.
    pub margin_after: Option<QuoteAtoms>,
    /// Original bounded human-readable reason, not a parsed cause/certificate.
    pub reason: Option<String>,
}
impl std::fmt::Debug for Risk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Risk([PRIVATE])")
    }
}
enum Decoded<T> {
    Known(T),
    Held(HeldReason),
}
fn history<T: Clone + Eq>(
    profile: &Profile,
    body: &[u8],
    family: u8,
    normalize: impl Fn(&Object, &Mapping, Identity) -> Result<Decoded<T>, Error>,
) -> Result<Page<T>, Error> {
    let envelope: Object = decode(body)?;
    let rows: Vec<Box<RawValue>> = envelope.get("data")?;
    if rows.len() > MAX_ROWS {
        return Err(Error::Limit);
    }
    let cut = Cut::parse(envelope.get("page")?)?;
    let mut output = Vec::new();
    let mut seen = BTreeMap::new();
    let mut previous = None;
    for raw in rows {
        let row: Object = decode(raw.get().as_bytes())?;
        if address(&row.get::<String>("owner")?)? != profile.scope.account() {
            return Err(Error::Invalid);
        }
        let name: String = row.get("symbol")?;
        symbol(&name)?;
        let id = TradeId {
            slot: row.get("slot")?,
            sequence: row.get("sequence")?,
        };
        if id.slot < cut.start_slot
            || id.slot > cut.end_slot
            || previous.is_some_and(|old| id > old)
        {
            return Err(Error::Invalid);
        }
        previous = Some(id);
        let identity = Identity {
            native: id,
            key: profile.key(family, id),
            timestamp: row.get("timestamp")?,
        };
        let isolated: bool = row.get("iso")?;
        let iso_key = row.optional::<Option<String>>("isoPubkey")?.flatten();
        if let Some(key) = &iso_key {
            address(key)?;
        }
        if !isolated && iso_key.is_some() {
            return Err(Error::Conflict);
        }
        let normalized = if isolated {
            Decoded::Held(HeldReason::IsolatedLane)
        } else if let Some(mapping) = profile.mapping(&name) {
            normalize(&row, mapping, identity.clone())?
        } else {
            Decoded::Held(HeldReason::UnmappedMarket)
        };
        let observation = match normalized {
            Decoded::Known(value) => Observation::Known(value),
            Decoded::Held(reason) => Observation::Held(HeldEvent {
                identity,
                row: HeldRow {
                    symbol: name,
                    reason,
                    raw: RawRow(raw),
                },
            }),
        };
        match seen.get(&id) {
            Some(old) if old != &observation => return Err(Error::Conflict),
            Some(_) => {}
            None => {
                seen.insert(id, observation.clone());
                output.push(observation);
            }
        }
    }
    Ok(Page {
        context: profile.commitment(),
        body_hash: Sha256::digest(body).into(),
        rows: output,
        cut,
    })
}
/// Decode actual funding history facts. No rate-derived payment, settlement,
/// gross private allocation or journal event is generated from the snapshot.
pub fn funding(profile: &Profile, body: &[u8]) -> Result<Page<Funding>, Error> {
    history(profile, body, 1, |row, mapping, identity| {
        let mark = ReportedDecimal::new(row.amount("markPrice")?)?;
        if !mark.positive() {
            return Err(Error::Invalid);
        }
        Ok(Decoded::Known(Funding {
            identity,
            quantity: mapping.quantity(&row.amount("size")?)?,
            payment: profile.amount(&row.amount("payment")?)?,
            rate: ReportedDecimal::new(row.amount("fundingRate")?)?,
            mark,
        }))
    })
}
/// Decode forced-event observations without treating risk-vault amounts as lots
/// or human-readable reason text as a qualified shortfall/underfill cause.
pub fn risk(profile: &Profile, body: &[u8]) -> Result<Page<Risk>, Error> {
    history(profile, body, 2, |row, mapping, identity| {
        let kind = match row.get::<String>("eventType")?.as_str() {
            "liquidation" => RiskKind::Liquidation,
            "adl" => RiskKind::Adl,
            _ => return Ok(Decoded::Held(HeldReason::UnsupportedRisk)),
        };
        let amount = mapping.quantity(&row.amount("amount")?)?.lots();
        if amount <= 0 {
            return Err(Error::Invalid);
        }
        let price = ReportedDecimal::new(row.amount("price")?)?;
        if !price.positive() {
            return Err(Error::Invalid);
        }
        let reason: Option<String> = row.optional("reason")?;
        if reason.as_ref().is_some_and(|r| r.len() > 128) {
            return Err(Error::Limit);
        }
        Ok(Decoded::Known(Risk {
            identity,
            market: mapping.unit,
            amount: amount as u64,
            native_is_buy: row.get("isBuy")?,
            price,
            kind,
            margin_prior: row
                .optional_amount("marginPrior")?
                .map(|v| profile.amount(&v))
                .transpose()?,
            margin_after: row
                .optional_amount("marginAfter")?
                .map(|v| profile.amount(&v))
                .transpose()?,
            reason,
        }))
    })
}
