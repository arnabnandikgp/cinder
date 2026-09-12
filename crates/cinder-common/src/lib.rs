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
pub const OID_TTL_MS: u64 = 15_000;
pub const IN_FLIGHT_TTL_MS: u64 = 30_000;
pub const COMMIT_EVERY_FILLS: u64 = 20;
pub const COMMIT_EVERY_MS: u64 = 30_000;
pub const CINDER_FEE_BPS: u16 = 0;

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

pub fn stub_notional(abs_lots: u64) -> Option<u64> {
    abs_lots.checked_mul(STUB_NOTIONAL_PER_LOT)
}
