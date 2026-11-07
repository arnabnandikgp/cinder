//! Account-local and aggregate reports, deliberately not cash or basis setters.
use super::*;
use crate::{
    address,
    fills::MAX_ROWS,
    raw::{Object, decode},
    symbol,
};
use serde_json::value::RawValue;
use std::collections::BTreeSet;

/// Native quote-valued metric. Different meanings never combine implicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum Metric {
    /// Reported account-local margin balance, not a physical-token receipt.
    Total,
    /// Native available margin, not customer withdrawal capacity.
    Available,
    /// Impacted versus marked PnL difference.
    ExecutionImpact,
    /// Venue-reported transfer eligibility.
    Transferable,
    /// Native maintenance margin in use.
    Maintenance,
    /// Native maintenance requirement plus buffer.
    Buffered,
    /// Native reported notional.
    Notional,
    /// Cumulative native realized PnL, not a new settlement.
    RealizedPnl,
    /// Native marked unrealized PnL.
    UnrealizedPnl,
    /// Native reported cumulative fees; retain its sign unchanged.
    Fees,
    /// Native reported cumulative funding, not another payment.
    Funding,
}
const NAMES: [(&str, &str); 11] = [
    ("totalMargin", "aggTotalMargin"),
    ("availableMargin", "aggAvailableMargin"),
    ("executionImpact", "aggExecutionImpact"),
    ("transferableBalance", "aggTransferableBalance"),
    ("marginUsed", "aggMarginUsed"),
    ("bufferedMargin", "aggBufferedMargin"),
    ("notional", "aggNotional"),
    ("realizedPnl", "aggRealizedPnl"),
    ("unrealizedPnl", "aggUnrealizedPnl"),
    ("fees", "aggFees"),
    ("funding", "aggFunding"),
];
/// Reported metrics; missing is unknown, not zero, and aggregate is not additive.
#[derive(Clone, PartialEq, Eq)]
pub struct Margin {
    values: [Option<QuoteAtoms>; 11],
}
impl Margin {
    /// Exact reported atoms for this metric, or absent information.
    pub fn get(&self, metric: Metric) -> Option<QuoteAtoms> {
        self.values[metric as usize]
    }
}
impl std::fmt::Debug for Margin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Margin([PRIVATE])")
    }
}
fn margin(profile: &Profile, data: &Object, aggregate: bool) -> Result<Margin, Error> {
    let mut values = [None; 11];
    for (i, &(local, total)) in NAMES.iter().enumerate() {
        values[i] = data
            .optional_amount(if aggregate { total } else { local })?
            .map(|v| profile.amount(&v))
            .transpose()?;
    }
    if !aggregate
        && [0, 1, 4, 6, 7, 8, 9, 10]
            .iter()
            .any(|i| values[*i].is_none())
    {
        return Err(Error::Invalid);
    }
    Ok(Margin { values })
}
/// Current position report. Display prices and cumulative metrics are not basis.
#[derive(Clone, PartialEq, Eq)]
pub struct Position {
    /// Configured signed lots; zero explicitly represents a native flat row.
    pub quantity: QuantityLots,
    /// Original reported average, not rounded to a resting tick or multiplied into basis.
    pub entry: ReportedDecimal,
    /// Optional native fair price; no freshness/mark authority is implied.
    pub fair: Option<ReportedDecimal>,
    /// Optional native cumulative metrics, in configured quote atoms.
    pub metrics: Margin,
    /// Optional native entry time, retained as u64 without a JS Number conversion.
    pub average_entry_time: Option<u64>,
    /// Raw protection information, not a claim that protective orders are absent.
    pub protection: Option<RawRow>,
}
impl std::fmt::Debug for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Position([PRIVATE])")
    }
}
/// Response bound to trusted request/profile, not an independently authenticated cut.
#[derive(Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// Exact interpretation profile; current-state bodies do not echo account identity.
    pub context: [u8; 32],
    /// SHA-256 of original response bytes, for private provenance retention.
    pub body_hash: [u8; 32],
    /// Native account-local metrics only.
    pub local: Margin,
    /// Separate optional parent-plus-isolated totals; partial fields stay unknown.
    pub aggregate: Option<Margin>,
    /// Optional reported buffer ratio, not a qualified Cinder risk parameter.
    pub buffer_rate: Option<ReportedDecimal>,
    /// Configured direct-account positions, not a new position ledger.
    pub positions: Vec<Position>,
    /// Original unsupported-market/isolated positions, not dropped or deemed flat.
    pub held: Vec<HeldRow>,
    /// Original bounded resting-order inventory; no terminal/hold-release authority.
    pub open_orders: Vec<RawRow>,
    /// Optional queued on-fill plans; not live orders, and missing is not empty.
    pub pending_on_fill: Option<Vec<RawRow>>,
    /// Reported child accounts, not implicitly added collateral or authority.
    pub children: Option<Vec<[u8; 32]>>,
    /// Native locks, retained separately. Missing is not substituted with empty.
    pub locks: Option<Vec<RawRow>>,
}
impl std::fmt::Debug for Snapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("FinancialSnapshot([PRIVATE])")
    }
}
fn rows(value: Vec<Box<RawValue>>) -> Result<Vec<RawRow>, Error> {
    if value.len() > MAX_ROWS {
        return Err(Error::Limit);
    }
    value
        .into_iter()
        .map(|v| {
            let _: Object = decode(v.get().as_bytes())?;
            Ok(RawRow(v))
        })
        .collect()
}
/// Decode exactly one tagged fullAccount reply. Parent and kind must match the
/// trusted target; a request-bound body is still not proof of source authenticity.
pub fn decode_account(profile: &Profile, body: &[u8]) -> Result<Snapshot, Error> {
    let envelope: Vec<Object> = decode(body)?;
    if envelope.len() != 1 {
        return Err(Error::Invalid);
    }
    let account: Object = envelope[0].get("fullAccount")?;
    let kind: String = account.get("kind")?;
    let parent = account
        .optional::<String>("parent")?
        .map(|p| address(&p))
        .transpose()?;
    match &profile.target {
        Target::Master if kind == "MasterEOA" && parent.is_none() => {}
        Target::SubAccount { parent: expected }
            if kind == "SubAccount" && parent == Some(*expected) => {}
        _ => return Err(Error::Invalid),
    }
    let native: Object = account.get("margin")?;
    let local = margin(profile, &native, false)?;
    let aggregate = margin(profile, &native, true)?;
    let aggregate = aggregate
        .values
        .iter()
        .any(Option::is_some)
        .then_some(aggregate);
    let buffer_rate = native
        .optional_amount("marginBufferRate")?
        .map(ReportedDecimal::new)
        .transpose()?;
    let native_positions: Vec<Box<RawValue>> = account.get("positions")?;
    if native_positions.len() > MAX_ROWS {
        return Err(Error::Limit);
    }
    let mut positions = Vec::new();
    let mut held = Vec::new();
    let mut seen = BTreeSet::new();
    for raw in native_positions {
        let row: Object = decode(raw.get().as_bytes())?;
        let name: String = row.get("symbol")?;
        symbol(&name)?;
        let isolated: bool = row.get("iso")?;
        let iso_key = row.optional::<Option<String>>("isoPubkey")?.flatten();
        if !isolated && iso_key.is_some() {
            return Err(Error::Conflict);
        }
        let mapped = profile.mapping(&name);
        if isolated || mapped.is_none() {
            if let Some(key) = iso_key {
                address(&key)?;
            }
            held.push(HeldRow {
                symbol: name,
                reason: if isolated {
                    HeldReason::IsolatedLane
                } else {
                    HeldReason::UnmappedMarket
                },
                raw: RawRow(raw),
            });
            continue;
        }
        if !seen.insert(name) {
            return Err(Error::Conflict);
        }
        let mapping = mapped.ok_or(Error::Invalid)?;
        let quantity = mapping.quantity(&row.amount("size")?)?;
        let entry = ReportedDecimal::new(row.amount("price")?)?;
        if quantity.lots() != 0 && !entry.positive() {
            return Err(Error::Invalid);
        }
        // Position metrics use different maintenance naming; no account subtotal is inferred.
        let mut metrics = Margin { values: [None; 11] };
        for (i, &(name, _)) in NAMES.iter().enumerate() {
            metrics.values[i] = row
                .optional_amount(if i == 4 { "maintenanceMargin" } else { name })?
                .map(|v| profile.amount(&v))
                .transpose()?;
        }
        positions.push(Position {
            quantity,
            entry,
            fair: row
                .optional_amount("fairPrice")?
                .map(ReportedDecimal::new)
                .transpose()?,
            metrics,
            average_entry_time: row.optional("averageEntryTime")?,
            protection: row.optional::<Box<RawValue>>("protection")?.map(RawRow),
        });
    }
    let open_orders = rows(account.get("openOrders")?)?;
    let pending_on_fill = account
        .optional::<Vec<Box<RawValue>>>("pendingOnFill")?
        .map(rows)
        .transpose()?;
    let child_rows = account.optional("subAccounts")?.map(rows).transpose()?;
    let children_reported = child_rows.is_some();
    let mut children = Vec::new();
    for child in child_rows.into_iter().flatten() {
        let child: Object = decode(child.body().as_bytes())?;
        let key = address(&child.get::<String>("pubkey")?)?;
        if children.contains(&key) || key == profile.scope.account() {
            return Err(Error::Conflict);
        }
        children.push(key);
    }
    let locks = account
        .optional::<Vec<Box<RawValue>>>("lockedAmounts")?
        .map(rows)
        .transpose()?;
    Ok(Snapshot {
        context: profile.commitment(),
        body_hash: Sha256::digest(body).into(),
        local,
        aggregate,
        buffer_rate,
        positions,
        held,
        open_orders,
        pending_on_fill,
        children: children_reported.then_some(children),
        locks,
    })
}
