//! Reductions are not entries. Preserve uninvolved claims and test every
//! integer partial fill against a bounded close cost, even during insolvency.
use crate::rise::RiseView;
use crate::rpc::{Result, RuntimeError};
use crate::{CollateralRequirement, ExecutionPolicy, Operation, OrderKind, SolvencyPolicy};
use cinder_common as cc;
use cinder_ledger::UserLedger;
use phoenix_rise_math::{self as math, LimitOrderMarginState, SignedBaseLots, TraderPosition};

pub(crate) fn close_cost(v: &RiseView, op: &Operation) -> Result<i128> {
    let m = v
        .markets
        .get(&op.intent.asset_id)
        .ok_or(RuntimeError::Incomplete)?;
    let adverse = if op.intent.requested_lots > 0 {
        op.intent
            .limit_price_ticks
            .saturating_sub(m.mark_price.as_inner())
    } else {
        m.mark_price
            .as_inner()
            .saturating_sub(op.intent.limit_price_ticks)
    };
    i128::from(adverse)
        .checked_mul(i128::from(m.tick_size.as_inner()))
        .and_then(|n| n.checked_mul(i128::from(op.intent.requested_lots.unsigned_abs())))
        .and_then(|n| n.checked_add(i128::from(op.execution_budget?.max_fee_usdc)))
        .ok_or(RuntimeError::Decode)
}

pub(crate) fn assess(
    v: &RiseView,
    users: &[([u8; 32], UserLedger)],
    op: &Operation,
    execution: &ExecutionPolicy,
    policy: &SolvencyPolicy,
    now: u64,
) -> Result<u64> {
    execution.validate(&v.markets.keys().copied().collect())?;
    if !v.markets.contains_key(&op.intent.asset_id) || users.is_empty() {
        return Err(RuntimeError::Incomplete);
    }
    let b = op.execution_budget.ok_or(RuntimeError::Incomplete)?;
    b.validate().map_err(|_| RuntimeError::Configuration)?;
    let asset = op.intent.asset_id;
    let q = op.intent.requested_lots.unsigned_abs();
    if op.intent.identity.kind != OrderKind::Liquidation
        || q == 0
        || q > u64::from(execution.max_admission_lots)
        || u128::from(q + 1) * users.len() as u128 * (policy.scenarios.len() as u128 + 1) > 65_536
        || v.slot > op.intent.last_valid_slot
        || b.max_quote_lots > execution.market_quote_limits_usdc[&asset]
        || b.max_fee_usdc > execution.market_fee_limits_usdc[&asset]
    {
        return Err(RuntimeError::Incomplete);
    }
    let mut ls: Vec<_> = users.iter().map(|(a, l)| (*a, l.clone())).collect();
    for (_, l) in &mut ls {
        if usize::from(l.positions_len) > l.positions.len() {
            return Err(RuntimeError::Decode);
        }
        for p in &mut l.positions[..usize::from(l.positions_len)] {
            p.lots = crate::ledger::confirmed_position(p, &l.open_oids)?;
        }
        l.open_oids = Default::default();
        l.pending_oid_count = 0;
    }
    let index = ls
        .iter()
        .position(|(a, l)| {
            *a == op.intent.identity.user_ledger
                && l.user.to_bytes() == op.intent.identity.user_pubkey
        })
        .ok_or(RuntimeError::Identity)?;
    let start = ls[index].1.clone();
    let pi = start.positions[..usize::from(start.positions_len)]
        .iter()
        .position(|p| p.asset_id == asset)
        .ok_or(RuntimeError::Identity)?;
    let p = start.positions[pi];
    if p.lots == 0
        || op.intent.requested_lots.signum() == p.lots.signum()
        || q > p.lots.unsigned_abs()
    {
        return Err(RuntimeError::Identity);
    }
    let pre = crate::solvency::evaluate(v, &ls, Some(policy), now)?;
    let cost = close_cost(v, op)?;
    let native_lots = *v.positions.get(&asset).unwrap_or(&0);
    let native_entry = i64::try_from(
        *v.entry_quote_lots
            .get(&asset)
            .ok_or(RuntimeError::Incomplete)?,
    )
    .map_err(|_| RuntimeError::Decode)?;
    let unit = op
        .intent
        .limit_price_ticks
        .checked_mul(v.markets[&asset].tick_size.as_inner())
        .ok_or(RuntimeError::Decode)?;
    let mut shortfall = 0;
    for n in 0..=q {
        let delta =
            i64::try_from(n).map_err(|_| RuntimeError::Decode)? * op.intent.requested_lots.signum();
        let quote = i64::try_from(i128::from(delta) * i128::from(unit))
            .map_err(|_| RuntimeError::Decode)?;
        let realize = |lots, entry| {
            if delta == 0 {
                Some(cc::RealizeFill {
                    realized_usdc: 0,
                    new_entry_quote: entry,
                })
            } else {
                cc::realize_on_fill(lots, delta, entry, quote)
            }
        };
        let private = realize(p.lots, p.entry_quote_lots).ok_or(RuntimeError::Decode)?;
        let native = realize(native_lots, native_entry).ok_or(RuntimeError::Decode)?;
        let cash = i128::from(start.free) + i128::from(start.reserved)
            - i128::from(start.bad_debt_usdc)
            + i128::from(private.realized_usdc)
            - i128::from(b.max_fee_usdc);
        let l = &mut ls[index].1;
        *l = start.clone();
        l.free = u64::try_from(cash.max(0)).map_err(|_| RuntimeError::Decode)?;
        l.reserved = 0;
        l.bad_debt_usdc = u64::try_from((-cash).max(0)).map_err(|_| RuntimeError::Decode)?;
        l.positions[pi].lots = p.lots.checked_add(delta).ok_or(RuntimeError::Decode)?;
        l.positions[pi].entry_quote_lots = private.new_entry_quote;
        let mut post = v.clone();
        post.positions.insert(
            asset,
            native_lots.checked_add(delta).ok_or(RuntimeError::Decode)?,
        );
        post.entry_quote_lots
            .insert(asset, i128::from(native.new_entry_quote));
        let funding = *v
            .asset_funding
            .get(&asset)
            .ok_or(RuntimeError::Incomplete)?;
        let funding = if n == 0 { funding.min(0) } else { funding };
        post.funding = post
            .funding
            .checked_sub(i128::from(funding))
            .ok_or(RuntimeError::Decode)?;
        let native_cash =
            i128::from(v.collateral) + i128::from(native.realized_usdc) + i128::from(funding)
                - i128::from(b.max_fee_usdc);
        let mut im = 0u64;
        for (a, m) in &post.markets {
            let mut pos = TraderPosition::new();
            pos.base_lot_position = SignedBaseLots::new(*post.positions.get(a).unwrap_or(&0));
            im = im
                .checked_add(
                    math::initial_margin_for_asset(
                        m,
                        &pos,
                        &LimitOrderMarginState::empty(),
                        math::risk::RiskAction::View,
                    )
                    .map_err(|_| RuntimeError::Decode)?
                    .as_inner(),
                )
                .ok_or(RuntimeError::Decode)?;
        }
        let target = CollateralRequirement::for_hedge(im, 0)?.required_posted_usdc();
        let needed = u64::try_from((i128::from(target) - native_cash).max(0))
            .map_err(|_| RuntimeError::Decode)?;
        // Unsafe native books may close without reaching entry IM, but actual
        // MM and risk-tier bounds remain enforced atomically on the native IOC.
        let deposit = if v.safe { needed } else { 0 };
        if deposit > v.vault_balance || native_cash + i128::from(deposit) < 0 {
            return Err(RuntimeError::Incomplete);
        }
        shortfall = shortfall.max(deposit);
        post.collateral =
            u64::try_from(native_cash + i128::from(deposit)).map_err(|_| RuntimeError::Decode)?;
        post.vault_balance = v.vault_balance - deposit;
        let after = crate::solvency::evaluate(&post, &ls, Some(policy), now)?;
        if after.gross_notional_usdc > pre.gross_notional_usdc
            || after.backing_surplus_usdc
                < pre
                    .backing_surplus_usdc
                    .checked_sub(cost)
                    .ok_or(RuntimeError::Decode)?
            || after
                .worst_stress_surplus_usdc
                .ok_or(RuntimeError::Incomplete)?
                < pre
                    .worst_stress_surplus_usdc
                    .ok_or(RuntimeError::Incomplete)?
                    .checked_sub(cost)
                    .ok_or(RuntimeError::Decode)?
        {
            return Err(RuntimeError::Incomplete);
        }
    }
    Ok(shortfall)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvency::tests::{policy, user, view};
    use crate::{BoundedIntent, Journal, OrderIdentity};
    use std::collections::BTreeMap;
    const U: u64 = 1_000_000;
    type Fixture = (
        tempfile::TempDir,
        Operation,
        RiseView,
        Vec<([u8; 32], UserLedger)>,
        ExecutionPolicy,
    );
    fn setup(mark: u64, quantity: i64) -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let mut j = Journal::open(dir.path().join("private/journal.sqlite")).unwrap();
        let op = j
            .prepare_intent(BoundedIntent {
                identity: OrderIdentity {
                    user_ledger: [1; 32],
                    user_pubkey: [0; 32],
                    user_nonce: 0,
                    client_oid: [7; 16],
                    kind: OrderKind::Liquidation,
                },
                asset_id: 1,
                requested_lots: quantity,
                limit_price_ticks: mark,
                last_valid_slot: 100,
                post_fail_position_im_usdc: 0,
                created_at_ms: 1000,
            })
            .unwrap();
        let mut v = view(mark, 1000);
        v.positions.insert(1, 10);
        v.entry_quote_lots.insert(1, 1000 * i128::from(U));
        v.collateral = 1000 * U;
        v.vault_balance = 0;
        let mut l = user(10, 5);
        l.positions[0].lots += quantity;
        l.pending_oid_count = 1;
        l.open_oids[0] = cinder_ledger::OpenOid {
            client_oid: [7; 16],
            asset_id: 1,
            lots_delta: quantity,
            state: cc::OID_LIQUIDATING,
            limit_price_ticks: mark,
            last_valid_slot: 100,
        };
        let mut other = user(0, 995);
        other.user = anchor_lang::prelude::Pubkey::new_from_array([2; 32]);
        let execution = ExecutionPolicy {
            quote_headroom_bps: 2000,
            market_quote_limits_usdc: BTreeMap::from([(1, 10_000 * U)]),
            market_fee_limits_usdc: BTreeMap::from([(1, U)]),
            max_admission_lots: 32,
        };
        let op = j
            .record_execution_budget(&op.operation_id, execution.budget(&v, &op).unwrap())
            .unwrap();
        (dir, op, v, vec![([1; 32], l), ([2; 32], other)], execution)
    }
    #[test]
    fn reduction_can_recognize_loser_debt_without_spending_other_users_claims() {
        let (_dir, op, v, users, e) = setup(99, -10);
        let before = users[1].1.clone();
        assert_eq!(
            assess(&v, &users, &op, &e, &policy(11000, 9000, 10), 1000).unwrap(),
            0
        );
        assert_eq!(users[1].1.free, before.free);
        assert_eq!(users[1].1.bad_debt_usdc, 0);
        assert!(
            crate::admission::assess(&v, &users, &op, &e, &policy(11000, 9000, 10), 1000).is_err()
        );
    }
    #[test]
    fn unsafe_native_mm_books_use_reducing_path_not_entry_im_topup() {
        let (_dir, op, mut v, users, e) = setup(100, -5);
        v.safe = false;
        v.risk_tier = 2;
        v.mm_surplus = -45 * U as i64;
        v.collateral = 5 * U;
        v.vault_balance = 995 * U;
        assert_eq!(
            assess(&v, &users, &op, &e, &policy(11000, 9000, 10), 1000).unwrap(),
            0
        );
        assert!(
            crate::admission::assess(&v, &users, &op, &e, &policy(11000, 9000, 10), 1000).is_err()
        );
    }
    #[test]
    fn rejects_increase_flip_expired_wrong_identity_and_unbounded_close() {
        let (_dir, mut op, v, users, e) = setup(100, -5);
        let p = policy(11000, 9000, 10);
        assert!(assess(&v, &users, &op, &e, &p, 1000).is_ok());
        op.intent.requested_lots = 5;
        assert!(assess(&v, &users, &op, &e, &p, 1000).is_err());
        op.intent.requested_lots = -11;
        assert!(assess(&v, &users, &op, &e, &p, 1000).is_err());
        op.intent.requested_lots = -5;
        op.intent.last_valid_slot = 0;
        assert!(assess(&v, &users, &op, &e, &p, 1000).is_err());
        op.intent.last_valid_slot = 100;
        op.intent.identity.kind = OrderKind::User;
        assert!(assess(&v, &users, &op, &e, &p, 1000).is_err());
        op.intent.identity.kind = OrderKind::Liquidation;
        op.intent.asset_id = 2;
        assert!(assess(&v, &users, &op, &e, &p, 1000).is_err());
    }
    #[test]
    fn current_allocated_native_funding_settles_in_partial_fill_projection() {
        let (_dir, op, mut v, mut users, e) = setup(100, -5);
        v.funding = -i128::from(U);
        v.asset_funding.insert(1, -(U as i64));
        users[0].1.positions[0].unsettled_funding = -(U as i64);
        assert!(assess(&v, &users, &op, &e, &policy(11000, 9000, 10), 1000).is_ok());
    }
}
