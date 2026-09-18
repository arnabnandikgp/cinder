//! Backing and gross private-book risk, separate from net accounting identity.
//! These checks do not enable execution or replace post-intent admission,
//! continuous liquidation, or a deficit-resolution policy.
use crate::ledger::confirmed_position;
use crate::rise::{position_risk, RiseView};
use crate::rpc::{Result, RuntimeError};
use cinder_common as cc;
use cinder_ledger::UserLedger;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

/// Explicit operator policy, not guessed production risk parameters. Every
/// configured market needs a gross cap and adverse up/down scenario coverage.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SolvencyPolicy {
    pub max_gross_notional_usdc: u64,
    pub market_gross_limits_usdc: BTreeMap<u16, u64>,
    pub scenarios: Vec<StressScenario>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StressScenario {
    /// Complete market map: 10_000 means unchanged, not an omitted market.
    pub mark_factors_bps: BTreeMap<u16, u32>,
    /// Conservative aggregate close-cost allowance, including venue fees and
    /// slippage. It is reserved against both backing and the liable user's
    /// claim; it is not Cinder fee revenue or an assumed collectible debt.
    pub close_cost_bps: u16,
}

impl SolvencyPolicy {
    pub(crate) fn validate(&self, assets: &BTreeSet<u16>) -> Result<()> {
        let same_assets =
            |map: &BTreeMap<u16, _>| map.keys().copied().collect::<BTreeSet<_>>() == *assets;
        if assets.is_empty()
            || assets.contains(&0)
            || assets.len() > cc::MAX_BOOK_MARKETS
            || self.max_gross_notional_usdc == 0
            || !same_assets(&self.market_gross_limits_usdc)
            || self.market_gross_limits_usdc.values().any(|cap| *cap == 0)
            || self.scenarios.len() < 2
            || self.scenarios.len() > 64
        {
            return Err(RuntimeError::Configuration);
        }
        for scenario in &self.scenarios {
            if scenario
                .mark_factors_bps
                .keys()
                .copied()
                .collect::<BTreeSet<_>>()
                != *assets
                || scenario
                    .mark_factors_bps
                    .values()
                    .any(|factor| *factor == 0 || *factor > 100_000)
                || scenario.close_cost_bps == 0
                || u64::from(scenario.close_cost_bps) > cc::BPS_DENOM
            {
                return Err(RuntimeError::Configuration);
            }
        }
        if assets.iter().any(|asset| {
            !self
                .scenarios
                .iter()
                .any(|s| s.mark_factors_bps[asset] < 10_000)
                || !self
                    .scenarios
                    .iter()
                    .any(|s| s.mark_factors_bps[asset] > 10_000)
        }) {
            return Err(RuntimeError::Configuration);
        }
        Ok(())
    }
}

/// Aggregate-only result. Negative user equity never reduces other users'
/// positive claims. An existing debt halts even if collateral covers claims.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SolvencyReport {
    pub user_margin_safe: bool,
    pub positive_claims_usdc: i128,
    pub backing_equity_usdc: i128,
    pub backing_surplus_usdc: i128,
    pub bad_debt_usdc: u128,
    pub gross_notional_usdc: u128,
    pub gross_limits_safe: Option<bool>,
    pub worst_stress_surplus_usdc: Option<i128>,
}

impl SolvencyReport {
    /// Without configured scenarios, only current recovery health is known.
    /// Neither this nor successful reconciliation grants dispatch permission.
    pub fn recovery_safe(&self) -> bool {
        self.user_margin_safe
            && self.bad_debt_usdc == 0
            && self.positive_claims_usdc >= 0
            && self.backing_equity_usdc >= 0
            && self.backing_surplus_usdc >= 0
            && self
                .backing_equity_usdc
                .checked_sub(self.positive_claims_usdc)
                == Some(self.backing_surplus_usdc)
            && (self.gross_limits_safe.is_some() == self.worst_stress_surplus_usdc.is_some())
            && self.gross_limits_safe.is_none_or(|safe| safe)
            && self
                .worst_stress_surplus_usdc
                .is_none_or(|surplus| surplus >= 0)
    }

    pub fn configured_checks_pass(&self) -> bool {
        self.recovery_safe()
            && self.gross_limits_safe == Some(true)
            && self.worst_stress_surplus_usdc.is_some()
    }
}

pub(crate) struct UserValue {
    raw_equity: i128,
    pub effective_equity: i128,
    pub initial_margin: u64,
    pub maintenance_margin: u64,
    pub confirmed_positions: Vec<(u16, i64)>,
    notional: u64,
    market_notional: BTreeMap<u16, u64>,
}

fn notional(view: &RiseView, asset: u16, lots: i64) -> Result<u64> {
    let market = view.markets.get(&asset).ok_or(RuntimeError::Incomplete)?;
    lots.unsigned_abs()
        .checked_mul(market.mark_price.as_inner())
        .and_then(|n| n.checked_mul(market.tick_size.as_inner()))
        .ok_or(RuntimeError::Decode)
}

pub(crate) fn value_user(view: &RiseView, ledger: &UserLedger, now_ms: u64) -> Result<UserValue> {
    if ledger.positions_len as usize > ledger.positions.len() || ledger.withdrawable != 0 {
        return Err(RuntimeError::Unsupported);
    }
    let cash =
        i128::from(ledger.free) + i128::from(ledger.reserved) - i128::from(ledger.bad_debt_usdc);
    let mut value = UserValue {
        raw_equity: cash,
        effective_equity: cash,
        initial_margin: 0,
        maintenance_margin: 0,
        confirmed_positions: Vec::new(),
        notional: 0,
        market_notional: BTreeMap::new(),
    };
    for position in &ledger.positions[..ledger.positions_len as usize] {
        let lots = confirmed_position(position, &ledger.open_oids)?;
        let asset = position.asset_id;
        let n = notional(view, asset, lots)?;
        if value.market_notional.insert(asset, n).is_some() {
            return Err(RuntimeError::Identity);
        }
        value.notional = value.notional.checked_add(n).ok_or(RuntimeError::Decode)?;
        value.confirmed_positions.push((asset, lots));
    }
    for (position, &(asset, lots)) in ledger.positions[..ledger.positions_len as usize]
        .iter()
        .zip(&value.confirmed_positions)
    {
        let quote = position_risk(
            view,
            asset,
            lots,
            position.entry_quote_lots,
            value.notional,
            0,
            now_ms,
        )?;
        let market = &view.markets[&asset];
        let raw_upnl = i128::from(lots)
            .checked_mul(i128::from(market.mark_price.as_inner()))
            .and_then(|p| p.checked_mul(i128::from(market.tick_size.as_inner())))
            .and_then(|p| p.checked_sub(i128::from(position.entry_quote_lots)))
            .ok_or(RuntimeError::Decode)?;
        let funding = i128::from(position.unsettled_funding);
        value.raw_equity = value
            .raw_equity
            .checked_add(raw_upnl)
            .and_then(|e| e.checked_add(funding))
            .ok_or(RuntimeError::Decode)?;
        value.effective_equity = value
            .effective_equity
            .checked_add(quote.effective_equity_usdc)
            .and_then(|e| e.checked_add(funding))
            .ok_or(RuntimeError::Decode)?;
        value.initial_margin = value
            .initial_margin
            .checked_add(quote.post_position_im_usdc)
            .ok_or(RuntimeError::Decode)?;
        value.maintenance_margin = value
            .maintenance_margin
            .checked_add(quote.post_position_mm_usdc)
            .ok_or(RuntimeError::Decode)?;
    }
    Ok(value)
}

fn backing(view: &RiseView) -> Result<i128> {
    let mut equity = i128::from(view.vault_balance)
        .checked_add(i128::from(view.collateral))
        .and_then(|e| e.checked_add(view.funding))
        .ok_or(RuntimeError::Decode)?;
    for (asset, lots) in &view.positions {
        let market = view.markets.get(asset).ok_or(RuntimeError::Incomplete)?;
        let basis = view
            .entry_quote_lots
            .get(asset)
            .ok_or(RuntimeError::Incomplete)?;
        let upnl = i128::from(*lots)
            .checked_mul(i128::from(market.mark_price.as_inner()))
            .and_then(|p| p.checked_mul(i128::from(market.tick_size.as_inner())))
            .and_then(|p| p.checked_sub(*basis))
            .ok_or(RuntimeError::Decode)?;
        equity = equity.checked_add(upnl).ok_or(RuntimeError::Decode)?;
    }
    Ok(equity)
}

fn close_cost(notional: u64, bps: u16) -> Result<i128> {
    let numerator = u128::from(notional)
        .checked_mul(u128::from(bps))
        .and_then(|n| n.checked_add(u128::from(cc::BPS_DENOM - 1)))
        .ok_or(RuntimeError::Decode)?;
    i128::try_from(numerator / u128::from(cc::BPS_DENOM)).map_err(|_| RuntimeError::Decode)
}

pub(crate) fn evaluate(
    view: &RiseView,
    ledgers: &[([u8; 32], UserLedger)],
    policy: Option<&SolvencyPolicy>,
    now_ms: u64,
) -> Result<SolvencyReport> {
    let mut report = SolvencyReport {
        user_margin_safe: true,
        positive_claims_usdc: 0,
        backing_equity_usdc: backing(view)?,
        backing_surplus_usdc: 0,
        bad_debt_usdc: 0,
        gross_notional_usdc: 0,
        gross_limits_safe: None,
        worst_stress_surplus_usdc: None,
    };
    let mut gross_by_market = BTreeMap::new();
    let mut users = BTreeSet::new();
    for (address, ledger) in ledgers {
        if !users.insert(*address) {
            return Err(RuntimeError::Identity);
        }
        let value = value_user(view, ledger, now_ms)?;
        report.user_margin_safe &= value.effective_equity >= i128::from(value.initial_margin)
            && value
                .effective_equity
                .checked_mul(i128::from(cc::MAX_USER_LEVERAGE))
                .is_some_and(|capacity| capacity >= i128::from(value.notional));
        report.bad_debt_usdc = report
            .bad_debt_usdc
            .checked_add(u128::from(ledger.bad_debt_usdc))
            .ok_or(RuntimeError::Decode)?;
        report.positive_claims_usdc = report
            .positive_claims_usdc
            .checked_add(value.raw_equity.max(0))
            .ok_or(RuntimeError::Decode)?;
        report.gross_notional_usdc = report
            .gross_notional_usdc
            .checked_add(u128::from(value.notional))
            .ok_or(RuntimeError::Decode)?;
        for (asset, n) in value.market_notional {
            let gross = gross_by_market.entry(asset).or_insert(0u128);
            *gross = gross
                .checked_add(u128::from(n))
                .ok_or(RuntimeError::Decode)?;
        }
    }
    report.backing_surplus_usdc = report
        .backing_equity_usdc
        .checked_sub(report.positive_claims_usdc)
        .ok_or(RuntimeError::Decode)?;
    if let Some(policy) = policy {
        policy.validate(&view.markets.keys().copied().collect())?;
        report.gross_limits_safe = Some(
            report.gross_notional_usdc <= u128::from(policy.max_gross_notional_usdc)
                && gross_by_market
                    .iter()
                    .all(|(asset, n)| *n <= u128::from(policy.market_gross_limits_usdc[asset])),
        );
        let mut worst = i128::MAX;
        for scenario in &policy.scenarios {
            let mut stressed = view.clone();
            for (asset, market) in &mut stressed.markets {
                let numerator = u128::from(market.mark_price.as_inner())
                    .checked_mul(u128::from(scenario.mark_factors_bps[asset]))
                    .ok_or(RuntimeError::Decode)?;
                let ticks = u64::try_from(numerator / u128::from(cc::BPS_DENOM))
                    .map_err(|_| RuntimeError::Decode)?;
                if ticks == 0 {
                    return Err(RuntimeError::Configuration);
                }
                market.mark_price = phoenix_rise_math::Ticks::new(ticks);
            }
            let mut claims = 0i128;
            let mut costs = 0i128;
            for (_, ledger) in ledgers {
                let user = value_user(&stressed, ledger, now_ms)?;
                let cost = close_cost(user.notional, scenario.close_cost_bps)?;
                costs = costs.checked_add(cost).ok_or(RuntimeError::Decode)?;
                // Negative/defaulting claims cannot finance a winner. Reserve
                // every user's close cost on backing, including a defaulter's.
                claims = claims
                    .checked_add(
                        user.raw_equity
                            .checked_sub(cost)
                            .ok_or(RuntimeError::Decode)?
                            .max(0),
                    )
                    .ok_or(RuntimeError::Decode)?;
            }
            let surplus = backing(&stressed)?
                .checked_sub(costs)
                .and_then(|assets| assets.checked_sub(claims))
                .ok_or(RuntimeError::Decode)?;
            worst = worst.min(surplus);
        }
        report.worst_stress_surplus_usdc = Some(worst);
    }
    Ok(report)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use cinder_ledger::{OpenOid, Position};
    use phoenix_rise_math::*;

    const USDC: u64 = 1_000_000;

    pub(crate) fn view(mark: u64, cash: u64) -> RiseView {
        let tiers = LeverageTiers::new(std::array::from_fn(|i| LeverageTier {
            upper_bound_size: BaseLots::new((i as u64 + 1) * 1_000_000),
            max_leverage: Constant::new(10),
            limit_order_risk_factor: BasisPoints::new(10_000),
        }))
        .unwrap();
        RiseView {
            observed_ms: 1000,
            mark_ms: 1000,
            slot: 1,
            positions: BTreeMap::new(),
            entry_quote_lots: BTreeMap::from([(1, 0)]),
            collateral: 0,
            funding: 0,
            asset_funding: BTreeMap::from([(1, 0)]),
            snapshot_hash: [0; 32],
            vault_balance: cash * USDC,
            halt: 0,
            safe: true,
            markets: BTreeMap::from([(
                1,
                PerpAssetMetadata::new(
                    "SOL".into(),
                    1,
                    0,
                    Ticks::new(mark),
                    QuoteLotsPerBaseLotPerTick::new(USDC),
                    tiers,
                    [5000, 2000, 1000],
                    7500,
                    5000,
                    5000,
                ),
            )]),
        }
    }

    pub(crate) fn user(lots: i64, cash: u64) -> UserLedger {
        let mut ledger = UserLedger {
            schema_version: cc::ACCOUNT_SCHEMA_VERSION,
            user: anchor_lang::prelude::Pubkey::default(),
            free: cash * USDC,
            reserved: 0,
            withdrawable: 0,
            bad_debt_usdc: 0,
            pending_oid_count: 0,
            nonce: 0,
            last_funding_epoch: 0,
            positions_len: u8::from(lots != 0),
            positions: [Position::default(); cc::MAX_USER_POSITIONS],
            open_oids: [OpenOid::default(); cc::MAX_OPEN_OIDS_PER_USER],
            bump: 0,
        };
        if lots != 0 {
            ledger.positions[0] = Position {
                asset_id: 1,
                lots,
                entry_quote_lots: lots * 100 * USDC as i64,
                ..Default::default()
            };
        }
        ledger
    }

    pub(crate) fn policy(up: u32, down: u32, cost_bps: u16) -> SolvencyPolicy {
        SolvencyPolicy {
            max_gross_notional_usdc: 10_000 * USDC,
            market_gross_limits_usdc: BTreeMap::from([(1, 10_000 * USDC)]),
            scenarios: [up, down]
                .map(|factor| StressScenario {
                    mark_factors_bps: BTreeMap::from([(1, factor)]),
                    close_cost_bps: cost_bps,
                })
                .to_vec(),
        }
    }

    #[test]
    fn flat_native_pool_does_not_hide_gross_user_default_risk() {
        let view = view(100, 400);
        let users = [([1; 32], user(10, 200)), ([2; 32], user(-10, 200))];
        let current = evaluate(&view, &users, None, 1000).unwrap();
        assert!(view.positions.is_empty());
        assert!(current.recovery_safe());
        assert_eq!(current.gross_notional_usdc, 2000 * u128::from(USDC));
        assert!(
            !current.configured_checks_pass(),
            "current health is not stress proof"
        );
        let stressed = evaluate(&view, &users, Some(&policy(13_000, 7000, 10)), 1000).unwrap();
        assert!(stressed.user_margin_safe);
        assert_eq!(stressed.backing_surplus_usdc, 0);
        assert!(stressed.worst_stress_surplus_usdc.unwrap() < 0);
        assert!(!stressed.recovery_safe());
    }

    #[test]
    fn negative_user_equity_is_not_backing_for_positive_claims() {
        let users = [([1; 32], user(10, 200)), ([2; 32], user(-10, 200))];
        let current = evaluate(&view(130, 400), &users, None, 1000).unwrap();
        // Raw signed equity still sums to 400. Only 400 actual assets back
        // the winner's 500 claim; the loser's -100 is not an asset.
        assert_eq!(current.positive_claims_usdc, 500 * i128::from(USDC));
        assert_eq!(current.backing_surplus_usdc, -100 * i128::from(USDC));
        assert!(!current.recovery_safe());
    }

    #[test]
    fn gross_caps_bind_even_when_native_exposure_is_zero() {
        let users = [([1; 32], user(10, 200)), ([2; 32], user(-10, 200))];
        let mut policy = policy(11_000, 9000, 10);
        let safe = evaluate(&view(100, 400), &users, Some(&policy), 1000).unwrap();
        assert_eq!(safe.worst_stress_surplus_usdc, Some(0));
        assert!(safe.configured_checks_pass());
        for market_cap in [false, true] {
            policy.max_gross_notional_usdc = if market_cap { 10_000 } else { 1999 } * USDC;
            policy
                .market_gross_limits_usdc
                .insert(1, if market_cap { 1999 } else { 10_000 } * USDC);
            let report = evaluate(&view(100, 400), &users, Some(&policy), 1000).unwrap();
            assert_eq!(report.gross_limits_safe, Some(false));
            assert!(!report.configured_checks_pass());
        }
    }

    #[test]
    fn close_costs_of_a_defaulting_user_are_reserved_not_assumed_collectible() {
        let users = [([1; 32], user(10, 200)), ([2; 32], user(-10, 126))];
        let view = view(100, 326);
        let low_cost = evaluate(&view, &users, Some(&policy(11_000, 9000, 10)), 1000).unwrap();
        assert!(low_cost.configured_checks_pass());
        let high_cost = evaluate(&view, &users, Some(&policy(11_000, 9000, 300)), 1000).unwrap();
        assert!(high_cost.user_margin_safe);
        assert_eq!(
            high_cost.worst_stress_surplus_usdc,
            Some(-7 * i128::from(USDC))
        );
        assert!(!high_cost.configured_checks_pass());
    }

    #[test]
    fn debt_blocks_even_a_fully_backed_flat_book() {
        let mut ledger = user(0, 200);
        ledger.bad_debt_usdc = 10 * USDC;
        let report = evaluate(&view(100, 190), &[([1; 32], ledger)], None, 1000).unwrap();
        assert_eq!(report.backing_surplus_usdc, 0);
        assert_eq!(report.bad_debt_usdc, 10 * u128::from(USDC));
        assert!(!report.recovery_safe());
    }

    #[test]
    fn tentative_close_cannot_disguise_confirmed_private_risk() {
        let mut ledger = user(10, 200);
        ledger.positions[0].lots = 0;
        ledger.open_oids[0] = OpenOid {
            asset_id: 1,
            lots_delta: -10,
            state: cc::OID_PENDING,
            ..Default::default()
        };
        let mut view = view(100, 0);
        view.collateral = 200 * USDC;
        view.positions.insert(1, 10);
        view.entry_quote_lots.insert(1, 1000 * i128::from(USDC));
        let report = evaluate(&view, &[([1; 32], ledger)], None, 1000).unwrap();
        assert_eq!(report.gross_notional_usdc, 1000 * u128::from(USDC));
        assert_eq!(report.backing_surplus_usdc, 0);
        assert!(report.user_margin_safe);
    }

    #[test]
    fn funding_and_gain_haircuts_are_distinct_from_raw_claim_backing() {
        let mut ledger = user(10, 200);
        ledger.positions[0].unsettled_funding = -20 * USDC as i64;
        let mut view = view(110, 0);
        view.positions.insert(1, 10);
        view.entry_quote_lots.insert(1, 1000 * i128::from(USDC));
        view.collateral = 200 * USDC;
        view.funding = -20 * i128::from(USDC);
        let value = value_user(&view, &ledger, 1000).unwrap();
        assert_eq!(value.raw_equity, 280 * i128::from(USDC));
        assert_eq!(value.effective_equity, 230 * i128::from(USDC));
        let report = evaluate(&view, &[([1; 32], ledger)], None, 1000).unwrap();
        assert_eq!(report.backing_surplus_usdc, 0);
        assert!(report.recovery_safe());
    }

    #[test]
    fn cross_market_gain_haircuts_do_not_net_away_full_losses() {
        let mut view = view(150, 20);
        let mut btc = view.markets[&1].clone();
        btc.symbol = "BTC".into();
        btc.asset_id = 2;
        btc.mark_price = Ticks::new(50);
        view.markets.insert(2, btc);
        let mut ledger = user(1, 20);
        ledger.positions_len = 2;
        ledger.positions[1] = Position {
            asset_id: 2,
            ..ledger.positions[0]
        };
        let value = value_user(&view, &ledger, 1000).unwrap();
        assert_eq!(value.notional, 200 * USDC);
        assert_eq!(value.raw_equity, 20 * i128::from(USDC));
        assert_eq!(value.effective_equity, -5 * i128::from(USDC));
    }

    #[test]
    fn explicit_cross_market_stress_detects_defaults_hidden_by_correlated_moves() {
        let mut view = view(100, 100);
        let mut btc = view.markets[&1].clone();
        btc.symbol = "BTC".into();
        btc.asset_id = 2;
        view.markets.insert(2, btc);
        view.entry_quote_lots.insert(2, 0);
        let mut a = user(1, 50);
        a.positions_len = 2;
        a.positions[1] = Position {
            asset_id: 2,
            lots: -1,
            entry_quote_lots: -(100 * USDC as i64),
            ..Default::default()
        };
        let mut b = user(-1, 50);
        b.positions_len = 2;
        b.positions[1] = Position {
            asset_id: 2,
            lots: 1,
            entry_quote_lots: 100 * USDC as i64,
            ..Default::default()
        };
        let users = [([1; 32], a), ([2; 32], b)];
        assert!(evaluate(&view, &users, None, 1000).unwrap().recovery_safe());
        let mut policy = policy(20_000, 5000, 10);
        policy.market_gross_limits_usdc.insert(2, 10_000 * USDC);
        policy.scenarios[0].mark_factors_bps.insert(2, 20_000);
        policy.scenarios[1].mark_factors_bps.insert(2, 5000);
        let correlated = evaluate(&view, &users, Some(&policy), 1000).unwrap();
        assert!(correlated.configured_checks_pass());
        policy.scenarios[0].mark_factors_bps.insert(2, 5000);
        policy.scenarios[1].mark_factors_bps.insert(2, 20_000);
        let spread = evaluate(&view, &users, Some(&policy), 1000).unwrap();
        assert!(spread.worst_stress_surplus_usdc.unwrap() < 0);
        assert!(!spread.configured_checks_pass());
    }

    #[test]
    fn policy_rejects_incomplete_one_sided_unpriced_or_unbounded_scenarios() {
        let assets = BTreeSet::from([1]);
        for change in 0..7 {
            let mut p = policy(11_000, 9000, 10);
            match change {
                0 => p.market_gross_limits_usdc.clear(),
                1 => p.scenarios[0].mark_factors_bps.clear(),
                2 => p.scenarios[1]
                    .mark_factors_bps
                    .insert(1, 10_000)
                    .map(|_| ())
                    .unwrap(),
                3 => p.scenarios[0].close_cost_bps = 0,
                4 => p.scenarios[0]
                    .mark_factors_bps
                    .insert(1, 0)
                    .map(|_| ())
                    .unwrap(),
                5 => p.max_gross_notional_usdc = 0,
                _ => p.scenarios.clear(),
            }
            assert!(p.validate(&assets).is_err(), "mutation {change}");
        }
        assert!(policy(11_000, 9000, 10)
            .validate(&BTreeSet::from([1, 2]))
            .is_err());
    }

    #[test]
    fn invalid_basis_duplicate_users_stale_quotes_and_overflow_fail_closed() {
        let mut ledger = user(10, 200);
        ledger.positions[0].entry_quote_lots = 0;
        assert!(evaluate(&view(100, 400), &[([1; 32], ledger)], None, 1000).is_err());
        let users = [([1; 32], user(10, 200)), ([1; 32], user(-10, 200))];
        assert!(evaluate(&view(100, 400), &users, None, 1000).is_err());
        let users = [([1; 32], user(10, 200))];
        assert!(evaluate(&view(100, 400), &users, None, 999).is_err());
        assert!(evaluate(&view(100, 400), &users, None, 1001 + cc::MARK_STALE_MS).is_err());
        assert!(evaluate(&view(u64::MAX, 400), &users, None, 1000).is_err());
    }

    #[test]
    fn incomplete_stress_evidence_or_inconsistent_backing_report_is_not_safe() {
        let mut report = evaluate(&view(100, 400), &[], None, 1000).unwrap();
        assert!(report.recovery_safe());
        report.gross_limits_safe = Some(true);
        assert!(!report.recovery_safe());
        report.worst_stress_surplus_usdc = Some(0);
        assert!(report.configured_checks_pass());
        report.backing_surplus_usdc += 1;
        assert!(!report.recovery_safe());
    }
}
