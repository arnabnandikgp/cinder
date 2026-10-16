//! Fee-tier quotes are observations, not the actual signed fee on an execution.
use super::*;
use crate::raw::{Object, decode};

/// Native resolved fee schedule scope, not an implicit collateral/risk pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeeScope {
    /// Account's global fee schedule.
    Global,
    /// Configured native instrument's fee schedule.
    Market(MarketUnit),
}
/// Native fee snapshot. No effective timestamp or guaranteed execution fee is invented.
#[derive(Clone, PartialEq, Eq)]
pub struct Quote {
    /// Configured interpretation context.
    pub context: [u8; 32],
    /// Original native response hash, not independently authenticated evidence.
    pub body_hash: [u8; 32],
    /// Native resolved schedule scope.
    pub scope: FeeScope,
    /// Account-reported rolling volume, not credited customer collateral.
    pub rolling_volume: QuoteAtoms,
    /// Native reported tier index.
    pub tier_index: u64,
    /// Native reported volume threshold, not a Cinder admission parameter.
    pub tier_threshold: QuoteAtoms,
    /// Reported maker bps; negative values retain rebates.
    pub maker_bps: ReportedDecimal,
    /// Reported taker bps, not subtracted from a fill a second time.
    pub taker_bps: ReportedDecimal,
    /// Reported fee window; no claim of current commercial eligibility.
    pub window_days: u64,
}
impl std::fmt::Debug for Quote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("FeeQuote([PRIVATE])")
    }
}
/// Decode one tagged feeTier response. Unknown market scopes cannot become a
/// default/global schedule; private quoted savings do not authorize execution.
pub fn quote(profile: &Profile, body: &[u8]) -> Result<Quote, Error> {
    let envelope: Vec<Object> = decode(body)?;
    if envelope.len() != 1 {
        return Err(Error::Invalid);
    }
    let row: Object = envelope[0].get("feeTier")?;
    let name: String = row.get("scopeInstrument")?;
    let scope = if name == "global" {
        FeeScope::Global
    } else {
        FeeScope::Market(profile.mapping(&name).ok_or(Error::Unsupported)?.unit)
    };
    let rolling_volume = profile.amount(&row.amount("rollingVolume")?)?;
    let tier_threshold = profile.amount(&row.amount("tierThreshold")?)?;
    let window_days = row.get("windowDays")?;
    if rolling_volume.atoms() < 0 || tier_threshold.atoms() < 0 || window_days == 0 {
        return Err(Error::Invalid);
    }
    Ok(Quote {
        context: profile.commitment(),
        body_hash: Sha256::digest(body).into(),
        scope,
        rolling_volume,
        tier_index: row.get("tierIndex")?,
        tier_threshold,
        maker_bps: ReportedDecimal::new(row.amount("makerBps")?)?,
        taker_bps: ReportedDecimal::new(row.amount("takerBps")?)?,
        window_days,
    })
}
