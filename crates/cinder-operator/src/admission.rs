//! Exhaustive integer partial-fill admission. Bounds are deliberately explicit:
//! large intents are refused rather than sampling a few seemingly safe fills.
//! Improving the IOC price can only improve cash/equity; margin depends on lots
//! and native marks, not entry. Every quantity uses the worst permitted price
//! and the entire atomically enforced fee allowance, including zero fills.
use crate::rise::RiseView;
use crate::rpc::{Result, RuntimeError};
use crate::{CollateralRequirement, ExecutionBudget, Operation};
use cinder_common as cc;
use cinder_ledger::UserLedger;
use phoenix_rise_math::{self as math, LimitOrderMarginState, SignedBaseLots, TraderPosition};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionPolicy {
    pub quote_headroom_bps: u16,
    pub market_quote_limits_usdc: BTreeMap<u16, u64>,
    /// Actual charges above this ceiling roll back with the IOC. No assumed
    /// public UI rate or stale trader fee override authorizes a charge.
    pub market_fee_limits_usdc: BTreeMap<u16, u64>,
    /// Resource bound for exhaustive admission, not an automatic order split.
    pub max_admission_lots: u32,
}
impl ExecutionPolicy {
    pub(crate) fn validate(&self, assets: &BTreeSet<u16>) -> Result<()> {
        if self.quote_headroom_bps > 10_000
            || self.max_admission_lots == 0
            || self.max_admission_lots > 4096
            || self
                .market_quote_limits_usdc
                .keys()
                .copied()
                .collect::<BTreeSet<_>>()
                != *assets
            || self
                .market_fee_limits_usdc
                .keys()
                .copied()
                .collect::<BTreeSet<_>>()
                != *assets
            || self
                .market_quote_limits_usdc
                .values()
                .any(|n| *n == 0 || *n > i64::MAX as u64)
        {
            return Err(RuntimeError::Configuration);
        }
        Ok(())
    }
    pub(crate) fn budget(&self, view: &RiseView, op: &Operation) -> Result<ExecutionBudget> {
        let asset = op.intent.asset_id;
        let market = view.markets.get(&asset).ok_or(RuntimeError::Incomplete)?;
        let lots = op.intent.requested_lots.unsigned_abs();
        if lots > u64::from(self.max_admission_lots) {
            return Err(RuntimeError::Unsupported);
        }
        let quote = u128::from(lots)
            .checked_mul(u128::from(market.mark_price.as_inner()))
            .and_then(|n| n.checked_mul(u128::from(market.tick_size.as_inner())))
            .and_then(|n| {
                n.checked_mul(u128::from(cc::BPS_DENOM) + u128::from(self.quote_headroom_bps))
            })
            .ok_or(RuntimeError::Decode)?
            .div_ceil(u128::from(cc::BPS_DENOM));
        let budget = ExecutionBudget {
            max_quote_lots: u64::try_from(quote)
                .map_err(|_| RuntimeError::Decode)?
                .min(self.market_quote_limits_usdc[&asset]),
            max_fee_usdc: self.market_fee_limits_usdc[&asset],
        };
        budget.validate().map_err(|_| RuntimeError::Configuration)?;
        Ok(budget)
    }
}

/// Returns the largest posted-cash shortfall over EVERY permitted integer fill.
/// Virtual funding only moves backing from the vault to native cash; it never
/// creates equity. Native and private entry bases are realized independently.
pub(crate) fn assess(
    view: &RiseView,
    ledgers: &[([u8; 32], UserLedger)],
    op: &Operation,
    execution: &ExecutionPolicy,
    solvency: &crate::SolvencyPolicy,
    now: u64,
) -> Result<u64> {
    if ledgers.is_empty()
        || ledgers
            .iter()
            .any(|(_, l)| usize::from(l.positions_len) > l.positions.len())
    {
        return Err(RuntimeError::Incomplete);
    }
    execution.validate(&view.markets.keys().copied().collect())?;
    let budget = op.execution_budget.ok_or(RuntimeError::Incomplete)?;
    budget.validate().map_err(|_| RuntimeError::Configuration)?;
    let asset = op.intent.asset_id;
    let quantity = op.intent.requested_lots.unsigned_abs();
    let work =
        u128::from(quantity + 1) * ledgers.len() as u128 * (solvency.scenarios.len() as u128 + 1);
    if quantity > u64::from(execution.max_admission_lots)
        || work > 65_536
        || budget.max_quote_lots > execution.market_quote_limits_usdc[&asset]
        || budget.max_fee_usdc > execution.market_fee_limits_usdc[&asset]
        || !view.safe
        || view.slot > op.intent.last_valid_slot
    {
        return Err(RuntimeError::Incomplete);
    }
    let user_index = ledgers
        .iter()
        .position(|(a, l)| {
            *a == op.intent.identity.user_ledger
                && l.user.to_bytes() == op.intent.identity.user_pubkey
        })
        .ok_or(RuntimeError::Identity)?;
    // Funding allocation/folds are R6. Until that authenticated loop exists,
    // refuse new execution across a funding boundary rather than guessing how
    // native hot-state settlement will distribute it to private users.
    if view.funding != 0
        || view.asset_funding.values().any(|n| *n != 0)
        || ledgers.iter().any(|(_, l)| {
            l.positions[..usize::from(l.positions_len)]
                .iter()
                .any(|p| p.unsettled_funding != 0)
        })
    {
        return Err(RuntimeError::FundingAllocationRequired);
    }
    let mut projected: Vec<_> = ledgers.iter().map(|(a, l)| (*a, l.clone())).collect();
    // Admission considers confirmed inventory, never another intent's tentative
    // exposure as an asset or already-executed hedge. All pending identities
    // remain journaled and will receive their own fresh admission later.
    for (_, ledger) in &mut projected {
        for p in &mut ledger.positions[..usize::from(ledger.positions_len)] {
            p.lots = crate::ledger::confirmed_position(p, &ledger.open_oids)?;
        }
        ledger.open_oids = Default::default();
        ledger.pending_oid_count = 0;
    }
    let starting = projected[user_index].1.clone();
    let position_index = starting.positions[..usize::from(starting.positions_len)]
        .iter()
        .position(|p| p.asset_id == asset)
        .ok_or(RuntimeError::Identity)?;
    let p = starting.positions[position_index];
    let native_lots = view.positions.get(&asset).copied().unwrap_or(0);
    let native_entry = i64::try_from(
        *view
            .entry_quote_lots
            .get(&asset)
            .ok_or(RuntimeError::Incomplete)?,
    )
    .map_err(|_| RuntimeError::Decode)?;
    let unit_quote = op
        .intent
        .limit_price_ticks
        .checked_mul(view.markets[&asset].tick_size.as_inner())
        .ok_or(RuntimeError::Decode)?;
    let mut shortfall = 0;
    for fill_abs in 0..=quantity {
        let lots = i64::try_from(fill_abs).map_err(|_| RuntimeError::Decode)?
            * op.intent.requested_lots.signum();
        let quote = i64::try_from(i128::from(lots) * i128::from(unit_quote))
            .map_err(|_| RuntimeError::Decode)?;
        // Buy prices below the bound are safer even when they allow more lots
        // under the quote cap. Keep these quantities in the envelope; never
        // infer a maximum fill from a buy's worst-price notional.
        let realize = |before, entry| {
            if lots == 0 {
                Ok(cc::RealizeFill {
                    realized_usdc: 0,
                    new_entry_quote: entry,
                })
            } else {
                cc::realize_on_fill(before, lots, entry, quote).ok_or(RuntimeError::Decode)
            }
        };
        let private = realize(p.lots, p.entry_quote_lots)?;
        let native = realize(native_lots, native_entry)?;
        let ledger = &mut projected[user_index].1;
        *ledger = starting.clone();
        let cash = i128::from(starting.free) + i128::from(starting.reserved)
            - i128::from(starting.bad_debt_usdc)
            + i128::from(private.realized_usdc)
            - i128::from(budget.max_fee_usdc);
        if cash < 0 {
            return Err(RuntimeError::Incomplete);
        }
        ledger.free = u64::try_from(cash).map_err(|_| RuntimeError::Decode)?;
        ledger.reserved = 0;
        ledger.positions[position_index].lots =
            p.lots.checked_add(lots).ok_or(RuntimeError::Decode)?;
        ledger.positions[position_index].entry_quote_lots = private.new_entry_quote;
        let mut native_view = view.clone();
        let post_lots = native_lots.checked_add(lots).ok_or(RuntimeError::Decode)?;
        native_view.positions.insert(asset, post_lots);
        native_view
            .entry_quote_lots
            .insert(asset, i128::from(native.new_entry_quote));
        let funding = if fill_abs == 0 {
            0
        } else {
            *view
                .asset_funding
                .get(&asset)
                .ok_or(RuntimeError::Incomplete)?
        };
        let post_cash =
            i128::from(view.collateral) + i128::from(native.realized_usdc) + i128::from(funding)
                - i128::from(budget.max_fee_usdc);
        native_view.funding = view
            .funding
            .checked_sub(i128::from(funding))
            .ok_or(RuntimeError::Decode)?;
        let mut im = 0u64;
        for (id, market) in &view.markets {
            let mut position = TraderPosition::new();
            position.base_lot_position =
                SignedBaseLots::new(*native_view.positions.get(id).unwrap_or(&0));
            let margin = math::initial_margin_for_asset(
                market,
                &position,
                &LimitOrderMarginState::empty(),
                math::risk::RiskAction::View,
            )
            .map_err(|_| RuntimeError::Decode)?;
            im = im
                .checked_add(margin.as_inner())
                .ok_or(RuntimeError::Decode)?;
        }
        let target = CollateralRequirement::for_hedge(im, 0)?.required_posted_usdc();
        let deposit = u64::try_from((i128::from(target) - post_cash).max(0))
            .map_err(|_| RuntimeError::Decode)?;
        if deposit > view.vault_balance {
            return Err(RuntimeError::Incomplete);
        }
        shortfall = shortfall.max(deposit);
        native_view.vault_balance = view.vault_balance - deposit;
        native_view.collateral =
            u64::try_from(post_cash + i128::from(deposit)).map_err(|_| RuntimeError::Decode)?;
        if !crate::solvency::evaluate(&native_view, &projected, Some(solvency), now)?
            .configured_checks_pass()
        {
            return Err(RuntimeError::Incomplete);
        }
    }
    Ok(shortfall)
}

/// Recheck accounting on the SAME fresh view used for admission, not only on
/// the coordinator's earlier startup snapshot. No pending cash is fabricated.
pub(crate) fn accounting(
    view: &RiseView,
    book: &cinder_ledger::Book,
    ledgers: &[([u8; 32], UserLedger)],
) -> Result<()> {
    if usize::from(book.residual_len) > book.residuals.len() {
        return Err(RuntimeError::Decode);
    }
    let mut book_lots = BTreeMap::new();
    for p in &book.residuals[..usize::from(book.residual_len)] {
        if p.asset_id == 0 || book_lots.insert(p.asset_id, p.lots).is_some() {
            return Err(RuntimeError::Identity);
        }
    }
    let mut lots = BTreeMap::<u16, i64>::new();
    let mut cash = 0i128;
    let mut basis = 0i128;
    for (_, l) in ledgers {
        cash = cash
            .checked_add(i128::from(l.free) + i128::from(l.reserved) - i128::from(l.bad_debt_usdc))
            .ok_or(RuntimeError::Decode)?;
        for p in &l.positions[..usize::from(l.positions_len)] {
            let q = crate::ledger::confirmed_position(p, &l.open_oids)?;
            let total = lots.entry(p.asset_id).or_default();
            *total = total.checked_add(q).ok_or(RuntimeError::Decode)?;
            basis = basis
                .checked_add(i128::from(p.entry_quote_lots))
                .ok_or(RuntimeError::Decode)?;
            cash = cash
                .checked_add(i128::from(p.unsettled_funding))
                .ok_or(RuntimeError::Decode)?;
        }
    }
    for asset in lots
        .keys()
        .chain(book_lots.keys())
        .chain(view.positions.keys())
    {
        let confirmed = *lots.get(asset).unwrap_or(&0);
        if confirmed != *book_lots.get(asset).unwrap_or(&0)
            || confirmed != *view.positions.get(asset).unwrap_or(&0)
        {
            return Err(RuntimeError::Incomplete);
        }
    }
    let native_basis = view
        .entry_quote_lots
        .values()
        .try_fold(0i128, |n, b| n.checked_add(*b))
        .ok_or(RuntimeError::Decode)?;
    let backing_cash = i128::from(view.vault_balance)
        .checked_add(i128::from(view.collateral))
        .and_then(|n| n.checked_add(view.funding))
        .and_then(|n| n.checked_add(basis))
        .and_then(|n| n.checked_sub(native_basis))
        .ok_or(RuntimeError::Decode)?;
    if cash < 0 || cash != backing_cash {
        return Err(RuntimeError::Incomplete);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvency::tests::{policy, user, view};
    use crate::{BoundedIntent, Journal, OrderIdentity, OrderKind};
    const USDC: u64 = 1_000_000;
    type Setup = (
        tempfile::TempDir,
        Operation,
        RiseView,
        Vec<([u8; 32], UserLedger)>,
        ExecutionPolicy,
    );
    fn setup(request: i64, bound: u64, starting_lots: i64, cash: u64) -> Setup {
        let dir = tempfile::tempdir().unwrap();
        let mut journal = Journal::open(dir.path().join("private/journal.sqlite")).unwrap();
        let intent = BoundedIntent {
            identity: OrderIdentity {
                user_ledger: [1; 32],
                user_pubkey: [0; 32],
                user_nonce: 1,
                client_oid: [7; 16],
                kind: OrderKind::User,
            },
            asset_id: 1,
            requested_lots: request,
            limit_price_ticks: bound,
            last_valid_slot: 100,
            post_fail_position_im_usdc: 0,
            created_at_ms: 1000,
        };
        let op = journal.prepare_intent(intent).unwrap();
        let mut v = view(100, cash);
        v.positions.insert(1, starting_lots);
        v.entry_quote_lots
            .insert(1, i128::from(starting_lots) * 100 * i128::from(USDC));
        let mut l = user(starting_lots, cash);
        if starting_lots == 0 {
            l.positions_len = 1;
            l.positions[0].asset_id = 1;
        }
        l.positions[0].lots += request;
        l.open_oids[0] = cinder_ledger::OpenOid {
            client_oid: [7; 16],
            asset_id: 1,
            lots_delta: request,
            state: cc::OID_PENDING,
            limit_price_ticks: bound,
            last_valid_slot: 100,
        };
        l.pending_oid_count = 1;
        let execution = ExecutionPolicy {
            quote_headroom_bps: 2000,
            market_quote_limits_usdc: BTreeMap::from([(1, 10000 * USDC)]),
            market_fee_limits_usdc: BTreeMap::from([(1, USDC)]),
            max_admission_lots: 32,
        };
        let budget = execution.budget(&v, &op).unwrap();
        let op = journal
            .record_execution_budget(&op.operation_id, budget)
            .unwrap();
        (dir, op, v, vec![([1; 32], l)], execution)
    }
    #[test]
    fn funds_cash_floor_and_fee_without_creating_backing() {
        let (_dir, op, v, users, execution) = setup(1, 100, 0, 200);
        assert_eq!(
            assess(&v, &users, &op, &execution, &policy(11000, 9000, 10), 1000).unwrap(),
            51 * USDC
        );
        let mut funded = v.clone();
        funded.collateral = 51 * USDC;
        funded.vault_balance -= 51 * USDC;
        assert_eq!(
            assess(
                &funded,
                &users,
                &op,
                &execution,
                &policy(11000, 9000, 10),
                1000
            )
            .unwrap(),
            0
        );
        funded.vault_balance = 0;
        assert!(assess(
            &funded,
            &users,
            &op,
            &execution,
            &policy(11000, 9000, 10),
            1000
        )
        .is_err());
    }
    #[test]
    fn admits_long_and_short_reversal_with_independent_entry_bases() {
        for (lots, request) in [(3, -5), (-3, 5)] {
            let (_dir, op, mut v, users, execution) = setup(request, 100, lots, 1000);
            // A user's DCA basis need not match the venue's net basis. Move
            // native cash by the opposite basis difference to preserve I2.
            v.entry_quote_lots
                .insert(1, i128::from(lots) * 90 * i128::from(USDC));
            v.collateral = (600i64 - lots * 10) as u64 * USDC;
            v.vault_balance -= 600 * USDC;
            assert_eq!(
                assess(&v, &users, &op, &execution, &policy(11000, 9000, 10), 1000).unwrap(),
                0
            );
        }
    }
    #[test]
    fn rejects_unsafe_price_cost_gross_or_unknown_funding() {
        let (_dir, mut op, mut v, users, execution) = setup(10, 100, 0, 200);
        let stress = policy(11000, 9000, 10);
        assert!(assess(&v, &users, &op, &execution, &stress, 1000).is_ok());
        op.intent.limit_price_ticks = 130;
        assert!(assess(&v, &users, &op, &execution, &stress, 1000).is_err());
        op.intent.limit_price_ticks = 100;
        let mut tight = stress.clone();
        tight.max_gross_notional_usdc = 500 * USDC;
        assert!(assess(&v, &users, &op, &execution, &tight, 1000).is_err());
        v.asset_funding.insert(1, -1);
        assert!(assess(&v, &users, &op, &execution, &stress, 1000).is_err());
    }
    #[test]
    fn never_samples_or_silently_splits_large_intents() {
        let (_dir, mut op, v, users, execution) = setup(1, 100, 0, 200);
        op.intent.requested_lots = 33;
        assert!(execution.budget(&v, &op).is_err());
        assert!(assess(&v, &users, &op, &execution, &policy(11000, 9000, 10), 1000).is_err());
    }
}
