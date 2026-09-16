//! The adapter's only risk boundary.
//!
//! Rise is the source of the market and pooled-trader calculations.  The
//! adapter deliberately consumes a normalized snapshot rather than trying to
//! recreate those calculations from a price feed.  This makes the values that
//! are co-signed into the ER ledger auditable and, importantly, gives the
//! scanner and funding crank the exact same health decision.

use cinder_common as cc;

use crate::phoenix::PoolHealth;
use crate::AdapterError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarketStatus {
    Active,
    Inactive,
    Unsupported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitStatus {
    /// All values are normalized to native USDC and Phoenix base lots.
    Verified,
    Unverified,
}

/// A normalized, post-simulation Rise/Phoenix observation for one asset.
///
/// The operator must build this from one coherent Rise snapshot.  In
/// particular `position_lots`, the Phoenix margins and the mark must describe
/// the same state.  We reject a snapshot rather than guessing conversions or
/// falling back to local stub math.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RiskSnapshot {
    pub asset_id: u16,
    pub position_lots: i64,
    pub mark_price_ticks: u64,
    pub observed_slot: u64,
    pub observed_at_ms: u64,
    pub market_status: MarketStatus,
    pub units: UnitStatus,
    /// Phoenix/Rise initial and maintenance margin in native USDC for
    /// `position_lots`.
    pub phoenix_initial_margin_usdc: u64,
    pub phoenix_maintenance_margin_usdc: u64,
    /// Total notional across the user's post-transition private book. This
    /// cannot be reconstructed from the single-asset snapshot alone.
    pub total_notional_usdc: u64,
    /// Rise/Phoenix unrealized PnL for this exact position in native USDC,
    /// before Cinder's conservative gain treatment.
    pub unrealized_pnl_usdc: i128,
    /// First tier leverage reported by Rise. Zero means the tier is not
    /// supported/verified.
    pub first_tier_leverage: u64,
    /// Rise discounted-uPnL factor. `None` uses Cinder's conservative gain
    /// haircut; a supplied factor must be at most 10_000.
    pub upnl_gain_factor_bps: Option<u16>,
    /// Explicit metadata used to convert Phoenix quote lots into native USDC.
    /// A zero component is rejected; it is never silently assumed to be 1.
    pub tick_size_in_quote_lots_per_base_lot: u64,
    pub quote_lot_to_usdc_numerator: u64,
    pub quote_lot_to_usdc_denominator: u64,
    /// Present only for pooled-trader simulations. User snapshots do not use
    /// this field.
    pub post_pool_health: Option<PoolHealth>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserRiskQuote {
    pub post_position_im_usdc: u64,
    pub post_position_mm_usdc: u64,
    pub effective_equity_usdc: i128,
    pub total_notional_usdc: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PoolRiskQuote {
    pub required_initial_margin_usdc: u64,
    pub required_maintenance_margin_usdc: u64,
    pub health: PoolHealth,
}

pub trait RiskEngine {
    fn quote_user_after_order(
        &self,
        snapshot: &RiskSnapshot,
        current_lots: i64,
        post_lots: i64,
        collateral_usdc: i128,
        now_ms: u64,
    ) -> Result<UserRiskQuote, AdapterError>;

    fn quote_user_health(
        &self,
        snapshot: &RiskSnapshot,
        lots: i64,
        collateral_usdc: i128,
        now_ms: u64,
    ) -> Result<UserRiskQuote, AdapterError>;

    fn quote_pool_after_residual(
        &self,
        snapshot: &RiskSnapshot,
        residual_lots: i64,
        now_ms: u64,
    ) -> Result<PoolRiskQuote, AdapterError>;
}

/// Production/fork engine.  It accepts only validated Rise/Phoenix output;
/// there is intentionally no local formula fallback in this implementation.
#[derive(Clone, Copy, Debug, Default)]
pub struct RiseRiskEngine;

impl RiseRiskEngine {
    fn quote(
        &self,
        snapshot: &RiskSnapshot,
        expected_lots: i64,
        collateral_usdc: i128,
        now_ms: u64,
    ) -> Result<UserRiskQuote, AdapterError> {
        validate_snapshot(snapshot, expected_lots, now_ms)?;
        let im = scale_up(snapshot.phoenix_initial_margin_usdc, cc::USER_IM_MULT_BPS)?;
        let mm = scale_up(
            snapshot.phoenix_maintenance_margin_usdc,
            cc::USER_MM_MULT_BPS,
        )?;
        let position_notional = checked_notional(snapshot, expected_lots)?;
        if snapshot.total_notional_usdc < position_notional {
            return Err(AdapterError::Risk(
                "total notional is below position notional".into(),
            ));
        }
        let gain_factor = snapshot
            .upnl_gain_factor_bps
            .unwrap_or(cc::UPNL_GAIN_HAIRCUT_BPS);
        if gain_factor > cc::BPS_DENOM as u16 {
            return Err(AdapterError::Risk("invalid Rise uPnL factor".into()));
        }
        let effective_equity_usdc = collateral_usdc
            .checked_add(discount_upnl(snapshot.unrealized_pnl_usdc, gain_factor)?)
            .ok_or_else(|| AdapterError::Risk("equity overflow".into()))?;
        Ok(UserRiskQuote {
            post_position_im_usdc: im,
            post_position_mm_usdc: mm,
            effective_equity_usdc,
            total_notional_usdc: snapshot.total_notional_usdc,
        })
    }
}

impl RiskEngine for RiseRiskEngine {
    fn quote_user_after_order(
        &self,
        snapshot: &RiskSnapshot,
        _current_lots: i64,
        post_lots: i64,
        collateral_usdc: i128,
        now_ms: u64,
    ) -> Result<UserRiskQuote, AdapterError> {
        self.quote(snapshot, post_lots, collateral_usdc, now_ms)
    }

    fn quote_user_health(
        &self,
        snapshot: &RiskSnapshot,
        lots: i64,
        collateral_usdc: i128,
        now_ms: u64,
    ) -> Result<UserRiskQuote, AdapterError> {
        self.quote(snapshot, lots, collateral_usdc, now_ms)
    }

    fn quote_pool_after_residual(
        &self,
        snapshot: &RiskSnapshot,
        residual_lots: i64,
        now_ms: u64,
    ) -> Result<PoolRiskQuote, AdapterError> {
        validate_snapshot(snapshot, residual_lots, now_ms)?;
        let health = snapshot
            .post_pool_health
            .ok_or_else(|| AdapterError::Risk("missing post-residual pool health".into()))?;
        Ok(PoolRiskQuote {
            required_initial_margin_usdc: snapshot.phoenix_initial_margin_usdc,
            required_maintenance_margin_usdc: snapshot.phoenix_maintenance_margin_usdc,
            health,
        })
    }
}

fn validate_snapshot(
    snapshot: &RiskSnapshot,
    expected_lots: i64,
    now_ms: u64,
) -> Result<(), AdapterError> {
    if snapshot.position_lots != expected_lots {
        return Err(AdapterError::Risk("snapshot position mismatch".into()));
    }
    if snapshot.asset_id == 0
        || snapshot.mark_price_ticks == 0
        || snapshot.observed_slot == 0
        || snapshot.first_tier_leverage == 0
        || snapshot.tick_size_in_quote_lots_per_base_lot == 0
        || snapshot.quote_lot_to_usdc_numerator == 0
        || snapshot.quote_lot_to_usdc_denominator == 0
        || snapshot.market_status != MarketStatus::Active
        || snapshot.units != UnitStatus::Verified
    {
        return Err(AdapterError::Risk(
            "invalid Rise/Phoenix risk snapshot".into(),
        ));
    }
    if snapshot.observed_at_ms > now_ms {
        return Err(AdapterError::Risk("future Rise/Phoenix snapshot".into()));
    }
    if snapshot
        .upnl_gain_factor_bps
        .is_some_and(|bps| bps > cc::BPS_DENOM as u16)
    {
        return Err(AdapterError::Risk("invalid Rise uPnL factor".into()));
    }
    let age = now_ms - snapshot.observed_at_ms;
    if age > cc::MARK_DEAD_MS {
        return Err(AdapterError::Risk("dead Rise/Phoenix snapshot".into()));
    }
    if age > cc::MARK_STALE_MS {
        return Err(AdapterError::Risk("stale Rise/Phoenix snapshot".into()));
    }
    Ok(())
}

fn scale_up(value: u64, bps: u16) -> Result<u64, AdapterError> {
    let numerator = (value as u128)
        .checked_mul(bps as u128)
        .ok_or_else(|| AdapterError::Risk("margin overflow".into()))?;
    let out = numerator
        .checked_add(cc::BPS_DENOM as u128 - 1)
        .ok_or_else(|| AdapterError::Risk("margin overflow".into()))?
        / cc::BPS_DENOM as u128;
    u64::try_from(out).map_err(|_| AdapterError::Risk("margin overflow".into()))
}

fn checked_notional(snapshot: &RiskSnapshot, lots: i64) -> Result<u64, AdapterError> {
    let quote_lots = (snapshot.mark_price_ticks as u128)
        .checked_mul(snapshot.tick_size_in_quote_lots_per_base_lot as u128)
        .and_then(|value| value.checked_mul(lots.unsigned_abs() as u128))
        .ok_or_else(|| AdapterError::Risk("notional overflow".into()))?;
    let numerator = quote_lots
        .checked_mul(snapshot.quote_lot_to_usdc_numerator as u128)
        .ok_or_else(|| AdapterError::Risk("notional overflow".into()))?;
    let denominator = snapshot.quote_lot_to_usdc_denominator as u128;
    let out = numerator
        .checked_add(denominator - 1)
        .ok_or_else(|| AdapterError::Risk("notional overflow".into()))?
        / denominator;
    u64::try_from(out).map_err(|_| AdapterError::Risk("notional overflow".into()))
}

/// Losses count in full; gains are rounded down after the Rise factor (or the
/// configured conservative fallback) is applied.
fn discount_upnl(value: i128, gain_factor_bps: u16) -> Result<i128, AdapterError> {
    if value < 0 {
        return Ok(value);
    }
    value
        .checked_mul(gain_factor_bps as i128)
        .ok_or_else(|| AdapterError::Risk("uPnL overflow".into()))
        .map(|v| v / cc::BPS_DENOM as i128)
}

/// Deterministic test-only engine.  It is deliberately not exported in a
/// default constructor and cannot become a production fallback.
#[cfg(any(test, feature = "test-utils"))]
#[derive(Clone, Copy, Debug, Default)]
pub struct StubRiskEngine;

#[cfg(any(test, feature = "test-utils"))]
impl RiskEngine for StubRiskEngine {
    fn quote_user_after_order(
        &self,
        snapshot: &RiskSnapshot,
        _current_lots: i64,
        post_lots: i64,
        collateral_usdc: i128,
        now_ms: u64,
    ) -> Result<UserRiskQuote, AdapterError> {
        let _ = now_ms;
        stub_quote(post_lots, collateral_usdc, snapshot)
    }

    fn quote_user_health(
        &self,
        snapshot: &RiskSnapshot,
        lots: i64,
        collateral_usdc: i128,
        now_ms: u64,
    ) -> Result<UserRiskQuote, AdapterError> {
        let _ = now_ms;
        stub_quote(lots, collateral_usdc, snapshot)
    }

    fn quote_pool_after_residual(
        &self,
        snapshot: &RiskSnapshot,
        residual_lots: i64,
        now_ms: u64,
    ) -> Result<PoolRiskQuote, AdapterError> {
        let _ = now_ms;
        let q = stub_quote(residual_lots, 0, snapshot)?;
        Ok(PoolRiskQuote {
            required_initial_margin_usdc: q.post_position_im_usdc,
            required_maintenance_margin_usdc: q.post_position_mm_usdc,
            health: snapshot.post_pool_health.unwrap_or(PoolHealth::Safe),
        })
    }
}

#[cfg(any(test, feature = "test-utils"))]
fn stub_quote(
    lots: i64,
    collateral_usdc: i128,
    snapshot: &RiskSnapshot,
) -> Result<UserRiskQuote, AdapterError> {
    let im = cc::stub_cinder_im(lots.unsigned_abs())
        .ok_or_else(|| AdapterError::Risk("stub margin overflow".into()))?;
    let mm = cc::stub_cinder_mm(lots.unsigned_abs())
        .ok_or_else(|| AdapterError::Risk("stub margin overflow".into()))?;
    let notional = cc::stub_notional(lots.unsigned_abs())
        .ok_or_else(|| AdapterError::Risk("stub notional overflow".into()))?;
    Ok(UserRiskQuote {
        post_position_im_usdc: im,
        post_position_mm_usdc: mm,
        effective_equity_usdc: collateral_usdc
            .checked_add(discount_upnl(
                snapshot.unrealized_pnl_usdc,
                snapshot
                    .upnl_gain_factor_bps
                    .unwrap_or(cc::UPNL_GAIN_HAIRCUT_BPS),
            )?)
            .ok_or_else(|| AdapterError::Risk("stub equity overflow".into()))?,
        total_notional_usdc: notional,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(lots: i64, now: u64) -> RiskSnapshot {
        RiskSnapshot {
            asset_id: 1,
            position_lots: lots,
            mark_price_ticks: 2_000_001,
            observed_slot: 123,
            observed_at_ms: now,
            market_status: MarketStatus::Active,
            units: UnitStatus::Verified,
            phoenix_initial_margin_usdc: 1_000_001,
            phoenix_maintenance_margin_usdc: 500_001,
            total_notional_usdc: 6_000_003,
            unrealized_pnl_usdc: 0,
            first_tier_leverage: 10,
            upnl_gain_factor_bps: Some(5_000),
            tick_size_in_quote_lots_per_base_lot: 100,
            quote_lot_to_usdc_numerator: 1,
            quote_lot_to_usdc_denominator: 100,
            post_pool_health: Some(PoolHealth::Safe),
        }
    }

    #[test]
    fn rise_values_scale_with_ceiling_like_sdk_apply_bps_ceil() {
        let q = RiseRiskEngine
            .quote_user_health(&snapshot(3, 1_000), 3, 0, 1_000)
            .unwrap();
        // 1_000_001 * 12_500 / 10_000 = 1_250_001.25, ceiling.
        assert_eq!(q.post_position_im_usdc, 1_250_002);
        assert_eq!(q.post_position_mm_usdc, 625_002);
        assert_eq!(q.total_notional_usdc, 6_000_003);
    }

    #[test]
    fn rejects_stale_wrong_position_and_unverified_units() {
        assert!(RiseRiskEngine
            .quote_user_health(&snapshot(3, 1), 3, 0, 1 + cc::MARK_STALE_MS + 1)
            .is_err());
        assert!(RiseRiskEngine
            .quote_user_health(&snapshot(3, 1), 2, 0, 1)
            .is_err());
        let mut bad = snapshot(3, 1);
        bad.units = UnitStatus::Unverified;
        assert!(RiseRiskEngine.quote_user_health(&bad, 3, 0, 1).is_err());
    }

    #[test]
    fn notional_conversion_rounds_up_without_granting_capacity() {
        let mut fixture = snapshot(2, 1_000);
        fixture.mark_price_ticks = 101;
        fixture.tick_size_in_quote_lots_per_base_lot = 3;
        fixture.quote_lot_to_usdc_denominator = 100;
        fixture.total_notional_usdc = 7;
        let quote = RiseRiskEngine
            .quote_user_health(&fixture, 2, 0, 1_000)
            .unwrap();
        assert_eq!(quote.total_notional_usdc, 7);
    }

    #[test]
    fn rejects_understated_total_notional_and_missing_pool_health() {
        let mut fixture = snapshot(3, 1_000);
        fixture.total_notional_usdc = 6_000_002;
        assert!(RiseRiskEngine
            .quote_user_health(&fixture, 3, 10_000_000, 1_000)
            .is_err());

        fixture.total_notional_usdc = 6_000_003;
        fixture.post_pool_health = None;
        assert!(RiseRiskEngine
            .quote_pool_after_residual(&fixture, 3, 1_000)
            .is_err());
    }
}
