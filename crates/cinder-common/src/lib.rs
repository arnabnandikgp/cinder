//! Shared protocol constants and pure accounting helpers.
//! Account layouts live in the owning programs so Anchor discriminators stay correct.

pub const SEED_CONFIG: &[u8] = b"config";
pub const SEED_VAULT_AUTHORITY: &[u8] = b"vault-authority";
pub const SEED_RESERVE: &[u8] = b"reserve";
pub const SEED_USER: &[u8] = b"user";
pub const SEED_BOOK: &[u8] = b"book";
pub const SEED_FEES: &[u8] = b"fees";

pub const MAX_USER_POSITIONS: usize = 16;
pub const MAX_BOOK_MARKETS: usize = 32;
pub const MAX_OPEN_OIDS_PER_USER: usize = 8;
pub const MAX_ALLOWLIST: usize = 32;

/// Current serialized layout for versioned Cinder protocol accounts.
pub const ACCOUNT_SCHEMA_VERSION: u8 = 1;

/// UserLedger permission: user (view) + adapter (AUTHORITY + view).
pub const MAX_USER_PERMISSION_MEMBERS: usize = 2;
/// Book / FeeAccrual permission: adapter only.
pub const MAX_POOL_PERMISSION_MEMBERS: usize = 1;

pub const USER_IM_MULT_BPS: u16 = 12_500;
pub const USER_MM_MULT_BPS: u16 = 12_500;
pub const MAX_USER_LEVERAGE: u16 = 10;
pub const BUFFER_MIN_BPS: u16 = 2_000;
pub const BUFFER_FLOOR_USDC: u64 = 50_000_000;
pub const MARK_STALE_MS: u64 = 2_000;
pub const TRADER_STATE_STALE_MS: u64 = 2_000;
/// Minimum in-process liquidation scan interval.
pub const SCAN_INTERVAL_MS: u64 = 50;
/// `Book.last_scan_ms` write cadence. Not an ER-fee rule.
pub const HEARTBEAT_MS: u64 = 1_000;
/// Missed in-process scan → OPERATOR_DOWN.
pub const SCAN_DEAD_MS: u64 = 2_000;
/// Hard-stale mark → OPERATOR_DOWN. Soft stale is MARK_STALE_MS.
pub const MARK_DEAD_MS: u64 = 10_000;
/// If Rise uPnL risk factor is missing, haircut gains at 50%.
pub const UPNL_GAIN_HAIRCUT_BPS: u16 = 5_000;
pub const OID_TTL_MS: u64 = 15_000;
pub const IN_FLIGHT_TTL_MS: u64 = 30_000;
pub const COMMIT_EVERY_FILLS: u64 = 20;
pub const COMMIT_EVERY_MS: u64 = 30_000;
pub const CINDER_FEE_BPS: u16 = 0;
/// |sum(user funding deltas) − pool delta| above this logs; do not halt.
pub const FUNDING_DUST_CAP: u64 = 1_000;
pub const USDC_DECIMALS: u32 = 6;

/// Local ER validator identity. Never delegate local PDAs to mainnet/devnet TEE ids.
pub const LOCAL_ER_VALIDATOR_STR: &str = "mAGicPQYBMvcYveUZA5F5UNNwyHvfYh5xkLS2Fr1mev";

/// Local-test pricing model: one Phoenix lot equals one USDC of notional.
pub const STUB_NOTIONAL_PER_LOT: u64 = 1_000_000;

pub const BPS_DENOM: u64 = 10_000;

pub const HALT_ENTRIES: u8 = 1 << 0;
pub const HALT_WITHDRAW: u8 = 1 << 1;
pub const HALT_DEPOSIT: u8 = 1 << 2;
pub const UNSAFE_POOL: u8 = 1 << 3;
pub const INVARIANT_BROKEN: u8 = 1 << 4;
pub const OPERATOR_DOWN: u8 = 1 << 5;
pub const BAD_DEBT: u8 = 1 << 6;
pub const VENUE_BREACH: u8 = 1 << 7;

pub const OID_PENDING: u8 = 0;
pub const OID_ACKED: u8 = 1;
pub const OID_FAILED: u8 = 2;
pub const OID_LIQUIDATING: u8 = 3;

/// Apply an external cash credit. Existing debt is always retired before any
/// amount becomes withdrawable free collateral.
pub fn credit_cash(free: &mut u64, bad_debt: &mut u64, amount: u64) -> Option<()> {
    let debt_payment = amount.min(*bad_debt);
    let free_credit = amount.checked_sub(debt_payment)?;
    let next_free = free.checked_add(free_credit)?;
    *bad_debt -= debt_payment;
    *free = next_free;
    Some(())
}

/// Apply an irreversible cash debit. A debit consumes free collateral, then
/// reserved collateral, and records any remainder as explicit bad debt.
pub fn debit_cash(
    free: &mut u64,
    reserved: &mut u64,
    bad_debt: &mut u64,
    amount: u64,
) -> Option<()> {
    let free_debit = amount.min(*free);
    let after_free = amount.checked_sub(free_debit)?;
    let reserved_debit = after_free.min(*reserved);
    let debt_increase = after_free.checked_sub(reserved_debit)?;
    let next_debt = bad_debt.checked_add(debt_increase)?;

    *free -= free_debit;
    *reserved -= reserved_debit;
    *bad_debt = next_debt;
    Some(())
}

/// Apply signed native-USDC cash through the same debt-aware path used by
/// fills and funding folds. Positive values are credits; negative values are
/// debits. `i64::MIN` is handled through `unsigned_abs` without overflow.
pub fn apply_signed_cash(
    free: &mut u64,
    reserved: &mut u64,
    bad_debt: &mut u64,
    delta: i64,
) -> Option<()> {
    if delta >= 0 {
        credit_cash(free, bad_debt, delta as u64)
    } else {
        debit_cash(free, reserved, bad_debt, delta.unsigned_abs())
    }
}

pub fn entries_blocked(flags: u8) -> bool {
    flags
        & (HALT_ENTRIES | UNSAFE_POOL | INVARIANT_BROKEN | OPERATOR_DOWN | BAD_DEBT | VENUE_BREACH)
        != 0
}

pub fn withdraw_blocked(flags: u8) -> bool {
    flags & (HALT_WITHDRAW | INVARIANT_BROKEN | OPERATOR_DOWN | BAD_DEBT) != 0
}

pub fn deposit_blocked(flags: u8) -> bool {
    flags & (HALT_DEPOSIT | INVARIANT_BROKEN | OPERATOR_DOWN) != 0
}

/// Cinder IM = 1.25 × Phoenix IM on the user's own size.
/// Phoenix IM stub = notional / MAX_USER_LEVERAGE, notional = |lots| × STUB_NOTIONAL_PER_LOT.
pub fn stub_cinder_im(abs_lots: u64) -> Option<u64> {
    abs_lots
        .checked_mul(STUB_NOTIONAL_PER_LOT)?
        .checked_mul(USER_IM_MULT_BPS as u64)?
        .checked_div(MAX_USER_LEVERAGE as u64 * BPS_DENOM)
}

/// Stub Phoenix MM is half of stub Phoenix IM, then × 1.25. Local only; fork uses Rise.
pub fn stub_cinder_mm(abs_lots: u64) -> Option<u64> {
    stub_cinder_im(abs_lots)?.checked_div(2)
}

pub fn stub_notional(abs_lots: u64) -> Option<u64> {
    abs_lots.checked_mul(STUB_NOTIONAL_PER_LOT)
}

/// Losses in full; positive uPnL × `UPNL_GAIN_HAIRCUT_BPS`.
pub fn haircut_upnl(upnl: i128) -> i128 {
    if upnl >= 0 {
        upnl.saturating_mul(UPNL_GAIN_HAIRCUT_BPS as i128) / BPS_DENOM as i128
    } else {
        upnl
    }
}

/// `free + reserved + unsettled + haircut(uPnL)`.
pub fn cinder_equity(free: u64, reserved: u64, unsettled: i64, upnl: i128) -> i128 {
    free as i128 + reserved as i128 + unsettled as i128 + haircut_upnl(upnl)
}

/// Native USDC uPnL. `entry_quote == 0` means no basis → treat as 0 (do not invent profit).
pub fn upnl_usdc(lots: i64, mark_usdc_per_lot: i64, entry_quote: i64) -> i128 {
    if entry_quote == 0 {
        0
    } else {
        (lots as i128) * (mark_usdc_per_lot as i128) - (entry_quote as i128)
    }
}

/// Result of applying a fill to basis. `fill_quote == 0` never invents PnL.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RealizeFill {
    pub realized_usdc: i64,
    pub new_entry_quote: i64,
}

/// Same-side fills add exact quote value to basis (a weighted-average entry,
/// without rounding a stored average price). Opposite-side fills realize the
/// proportional closed basis; a reversal starts a new opposite-side basis.
/// Fail-ack must not call this.
pub fn realize_on_fill(
    lots_before: i64,
    filled: i64,
    entry_quote: i64,
    fill_quote: i64,
) -> Option<RealizeFill> {
    if filled == 0 {
        return None;
    }
    if lots_before == 0 || lots_before.signum() == filled.signum() {
        let new_entry = entry_quote.checked_add(fill_quote)?;
        return Some(RealizeFill {
            realized_usdc: 0,
            new_entry_quote: new_entry,
        });
    }
    if fill_quote == 0 {
        return Some(RealizeFill {
            realized_usdc: 0,
            new_entry_quote: entry_quote,
        });
    }
    let closed = lots_before.unsigned_abs().min(filled.unsigned_abs());
    let filled_abs = filled.unsigned_abs();
    let denom = lots_before.unsigned_abs() as i128;
    if denom == 0 || filled_abs == 0 {
        return None;
    }
    let entry_closed = (entry_quote as i128).checked_mul(closed as i128)? / denom;
    let closed_quote = if filled_abs > closed {
        (fill_quote as i128).checked_mul(closed as i128)? / (filled_abs as i128)
    } else {
        fill_quote as i128
    };
    let residual_quote = (fill_quote as i128).checked_sub(closed_quote)?;
    let realized = (-(entry_closed.checked_add(closed_quote)?))
        .try_into()
        .ok()?;
    let new_entry = if filled_abs > closed {
        residual_quote.try_into().ok()?
    } else {
        (entry_quote as i128)
            .checked_sub(entry_closed)?
            .try_into()
            .ok()?
    };
    Some(RealizeFill {
        realized_usdc: realized,
        new_entry_quote: new_entry,
    })
}

#[cfg(test)]
mod realize_tests {
    use super::*;

    #[test]
    fn open_adds_basis_no_pnl() {
        let r = realize_on_fill(0, 10, 0, 10_000_000).unwrap();
        assert_eq!(r.realized_usdc, 0);
        assert_eq!(r.new_entry_quote, 10_000_000);
    }

    #[test]
    fn long_close_above_entry_is_profit() {
        let r = realize_on_fill(10, -4, 10_000_000, -4_400_000).unwrap();
        assert_eq!(r.realized_usdc, 400_000);
        assert_eq!(r.new_entry_quote, 6_000_000);
    }

    #[test]
    fn short_cover_above_entry_is_loss() {
        let r = realize_on_fill(-10, 4, -10_000_000, 4_400_000).unwrap();
        assert_eq!(r.realized_usdc, -400_000);
        assert_eq!(r.new_entry_quote, -6_000_000);
    }

    #[test]
    fn zero_vwap_does_not_invent_pnl() {
        let r = realize_on_fill(10, -4, 10_000_000, 0).unwrap();
        assert_eq!(r.realized_usdc, 0);
        assert_eq!(r.new_entry_quote, 10_000_000);
    }

    #[test]
    fn flip_through_zero_splits_quote() {
        // Long 10 @ 1.0; sell 15 @ 1.1. Close 10, open short 5.
        let r = realize_on_fill(10, -15, 10_000_000, -16_500_000).unwrap();
        assert_eq!(r.realized_usdc, 1_000_000);
        assert_eq!(r.new_entry_quote, -5_500_000);
    }

    #[test]
    fn scale_ins_preserve_weighted_basis_for_longs_and_shorts() {
        for sign in [1i64, -1] {
            let first = realize_on_fill(0, 2 * sign, 0, 200_000_000 * sign).unwrap();
            let added =
                realize_on_fill(2 * sign, sign, first.new_entry_quote, 120_000_000 * sign).unwrap();
            assert_eq!(added.realized_usdc, 0);
            assert_eq!(added.new_entry_quote, 320_000_000 * sign);
            // Keep the exact total, not a rounded average price of 106.666667.
            let reduced =
                realize_on_fill(3 * sign, -sign, added.new_entry_quote, -110_000_000 * sign)
                    .unwrap();
            assert_eq!(reduced.realized_usdc, 3_333_334 * sign);
            assert_eq!(reduced.new_entry_quote, 213_333_334 * sign);
            let closed = realize_on_fill(
                2 * sign,
                -2 * sign,
                reduced.new_entry_quote,
                -220_000_000 * sign,
            )
            .unwrap();
            assert_eq!(closed.new_entry_quote, 0);
            assert_eq!(
                reduced.realized_usdc + closed.realized_usdc,
                10_000_000 * sign
            );
        }
    }

    #[test]
    fn reversing_an_averaged_position_starts_a_new_opposite_basis() {
        for sign in [1i64, -1] {
            let result =
                realize_on_fill(3 * sign, -5 * sign, 320_000_000 * sign, -550_000_000 * sign)
                    .unwrap();
            assert_eq!(result.realized_usdc, 10_000_000 * sign);
            assert_eq!(result.new_entry_quote, -220_000_000 * sign);
        }
    }

    #[test]
    fn partial_basis_rounding_preserves_exact_cash_minus_basis() {
        for sign in [1i64, -1] {
            for filled in [sign, -sign, -2 * sign, -3 * sign, -5 * sign] {
                let entry = 10 * sign; // Deliberately indivisible by 3 lots.
                let quote = filled * 4;
                let result = realize_on_fill(3 * sign, filled, entry, quote).unwrap();
                assert_eq!(
                    i128::from(result.realized_usdc) - i128::from(result.new_entry_quote),
                    -i128::from(entry) - i128::from(quote)
                );
            }
        }
    }
}

#[cfg(test)]
mod halt_tests {
    use super::*;

    #[test]
    fn unresolved_operator_state_blocks_new_withdrawal_requests() {
        assert!(entries_blocked(OPERATOR_DOWN));
        assert!(withdraw_blocked(OPERATOR_DOWN));
        assert!(!withdraw_blocked(0));
    }

    #[test]
    fn bad_debt_blocks_entries_and_withdrawals_but_allows_deposits() {
        assert!(entries_blocked(BAD_DEBT));
        assert!(withdraw_blocked(BAD_DEBT));
        assert!(!deposit_blocked(BAD_DEBT));
    }

    #[test]
    fn venue_breach_blocks_new_entries_only() {
        assert!(entries_blocked(VENUE_BREACH));
        assert!(!withdraw_blocked(VENUE_BREACH));
        assert!(!deposit_blocked(VENUE_BREACH));
    }
}

#[cfg(test)]
mod cash_tests {
    use super::*;

    #[test]
    fn debit_drains_free_then_reserved_then_records_debt() {
        let mut free = 5;
        let mut reserved = 7;
        let mut debt = 2;
        debit_cash(&mut free, &mut reserved, &mut debt, 20).unwrap();
        assert_eq!((free, reserved, debt), (0, 0, 10));
    }

    #[test]
    fn credits_retire_debt_before_increasing_free() {
        let mut free = 3;
        let mut debt = 8;
        credit_cash(&mut free, &mut debt, 5).unwrap();
        assert_eq!((free, debt), (3, 3));
        credit_cash(&mut free, &mut debt, 7).unwrap();
        assert_eq!((free, debt), (7, 0));
    }

    #[test]
    fn signed_minimum_becomes_debt_without_negation_overflow() {
        let mut free = 0;
        let mut reserved = 0;
        let mut debt = 0;
        apply_signed_cash(&mut free, &mut reserved, &mut debt, i64::MIN).unwrap();
        assert_eq!(debt, 1u64 << 63);
    }

    #[test]
    fn overflow_does_not_partially_mutate_cash() {
        let mut free = 1;
        let mut reserved = 2;
        let mut debt = u64::MAX;
        assert!(debit_cash(&mut free, &mut reserved, &mut debt, 4).is_none());
        assert_eq!((free, reserved, debt), (1, 2, u64::MAX));
    }
}
