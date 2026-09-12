//! Shared constants from `docs/03-phase4-freeze.md`.
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
/// In-process scan min interval (`docs/09`).
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

/// S1–S3 stand-in: 1 Phoenix lot = 1 USDC notional until S6 wires a real mark.
pub const STUB_NOTIONAL_PER_LOT: u64 = 1_000_000;

pub const BPS_DENOM: u64 = 10_000;

pub const HALT_ENTRIES: u8 = 1 << 0;
pub const HALT_WITHDRAW: u8 = 1 << 1;
pub const HALT_DEPOSIT: u8 = 1 << 2;
pub const UNSAFE_POOL: u8 = 1 << 3;
pub const INVARIANT_BROKEN: u8 = 1 << 4;
pub const OPERATOR_DOWN: u8 = 1 << 5;

pub const OID_PENDING: u8 = 0;
pub const OID_ACKED: u8 = 1;
pub const OID_FAILED: u8 = 2;
pub const OID_LIQUIDATING: u8 = 3;

pub fn entries_blocked(flags: u8) -> bool {
    flags & (HALT_ENTRIES | UNSAFE_POOL | INVARIANT_BROKEN | OPERATOR_DOWN) != 0
}

pub fn withdraw_blocked(flags: u8) -> bool {
    flags & (HALT_WITHDRAW | INVARIANT_BROKEN) != 0
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

/// Open: add fill quote to entry. Reduce (opposite sign): take proportional
/// basis, `realized = -(entry_closed + fill_quote)`. Fail-ack must not call this.
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
}
