# Phase 4 freeze — accounts, instructions, halt

Source of truth for program code. Discriminators: Anchor `sha256("global:<ix_name>")[0..8]`.
Do not invent extra PDAs. If a field is here, implement it.

## Stack

- Anchor workspace, programs `cinder_vault` (L1) and `cinder_ledger` (ER). Same program id may be deployed on both layers; instructions are gated by where accounts live.
- `ephemeral-rollups-sdk` 0.16.2 (`anchor`, `access-control`); 0.17.0 only if 0.16.2 cannot express a needed CPI.
- Adapter uses `phoenix-rise` against L1 only.
- Local ER validator id: `mAGicPQYBMvcYveUZA5F5UNNwyHvfYh5xkLS2Fr1mev`.

## PDA / account seed table

| Account | Layer | Seeds | Owner | Delegated to ER? | Private? |
|---|---|---|---|---|---|
| `Config` | L1 | `["config"]` | `cinder_vault` | no | no (halt is public) |
| `vault_authority` | L1 | `["vault-authority"]` | `cinder_vault` | no | no |
| Vault USDC ATA | L1 | ATA(`vault_authority`, USDC mint) | Token program | no | no |
| `ReserveRoot` | L1 | `["reserve"]` | `cinder_vault` | no | no |
| Phoenix trader | L1 | Phoenix seeds `(authority=vault_authority or stand-in, pda_index=0, subaccount_index=0)` | Phoenix | no | no |
| `UserLedger` | ER | `["user", user_pubkey]` | `cinder_ledger` | yes | yes |
| `Book` | ER | `["book"]` | `cinder_ledger` | yes | yes (adapter only) |
| `FeeAccrual` | ER | `["fees"]` | `cinder_ledger` | yes | yes (adapter only) |
| `EphemeralPermission` | ER | Permission program PDA for the data account | `ACLseoPoyC3cBqoUtkbjZ4aDrkurZW86v19pXz2XQnp1` | n/a (ER-only) | ACL |

`vault_authority` is the Phoenix `authority` once S9 lands. Until then a keypair stand-in is stored in `Config.vault_authority` and signs L1 ixs.

Do not delegate the vault ATA, Config, ReserveRoot, or the Phoenix trader.

## Types

| Quantity | Type | Unit |
|---|---|---|
| USDC cash | `u64` | native (`1 USDC = 1_000_000`) |
| Signed USDC (PnL, funding) | `i64` | native |
| Position | `i64` | Phoenix base lots (+ long / − short) |
| Asset | `u16` | Phoenix `assetId` |
| Slot | `u64` | named in field |
| Nonce | `u64` | per user, monotonic |
| Bps | `u16` | 10_000 = 100% |
| Client oid | `[u8; 16]` | user-supplied |

Caps: `MAX_USER_POSITIONS = 16`, `MAX_BOOK_MARKETS = 32`, `MAX_OPEN_OIDS_PER_USER = 8`. First allowlist: 1 market.

```
USER_IM_MULT_BPS      = 12500     # Cinder IM = 1.25 × Phoenix IM(user size)
USER_MM_MULT_BPS      = 12500     # Cinder MM = 1.25 × Phoenix MM(user size)
MAX_USER_LEVERAGE     = 10        # hard cap when Phoenix first tier is ~15x; never above venue first tier
BUFFER_MIN_BPS        = 2000      # extra posted on Phoenix vs Phoenix IM of residual
BUFFER_FLOOR_USDC     = 50_000_000
MARK_STALE_MS         = 2000
TRADER_STATE_STALE_MS = 2000
OID_TTL_MS            = 15000
IN_FLIGHT_TTL_MS      = 30000
COMMIT_EVERY_FILLS    = 20
COMMIT_EVERY_MS       = 30000
CINDER_FEE_BPS        = 0
```

Leverage rule (HyperLink-style overlay): user effective leverage ≤ `min(MAX_USER_LEVERAGE, phoenix_first_tier_max_leverage)`. Adapter rejects `place_order` that would exceed that on the **user** book, even if Phoenix would accept the residual.

## HaltFlags (`u8`)

| Bit | Name | Effect |
|---|---|---|
| 0 | `HALT_ENTRIES` | no `place_order` |
| 1 | `HALT_WITHDRAW` | no new withdraw |
| 2 | `HALT_DEPOSIT` | no `credit_deposit` |
| 3 | `UNSAFE_POOL` | Phoenix tier ≥ Cancellable |
| 4 | `INVARIANT_BROKEN` | I1/I2 failed |
| 5 | `OPERATOR_DOWN` | admin |

Entries blocked by bits 0, 3, 4, 5. Withdraw blocked by 1, 4.
Mirror on L1 `Config.paused` (public) and ER `Book.halt` (adapter-only). Users read halt from Config.

---

## L1 structs (`cinder_vault`)

```
Config {                                // seeds ["config"]
  admin:                 Pubkey,
  adapter:               Pubkey,        // operator / position_authority stand-in
  vault_authority:       Pubkey,        // PDA or stand-in keypair
  phoenix_trader:        Pubkey,        // trader PDA (0,0)
  usdc_mint:             Pubkey,
  vault_usdc_ata:        Pubkey,
  er_validator:          Pubkey,
  paused:                u8,            // HaltFlags
  user_im_mult_bps:      u16,
  user_mm_mult_bps:      u16,
  max_user_leverage:     u16,
  buffer_min_bps:        u16,
  buffer_floor_usdc:     u64,
  allowlist_len:         u8,
  allowlist_assets:      [u16; 32],
  bump_config:           u8,
  bump_vault_authority:  u8,
}

ReserveRoot {                           // seeds ["reserve"]
  epoch:                     u64,
  root:                      [u8; 32],  // sha256 of canonical claim list
  user_count:                u32,
  total_free:                u64,
  total_reserved:            u64,
  book_hash:                 [u8; 32],
  committed_at_base_slot:    u64,
}
```

Claim row bytes: `pubkey || free || reserved || n_pos || repeating (asset_id, lots)`.
`escape_withdraw` stub returns `unsupported` until a later spec.

`vault_authority` has no extra data; it is a program-derived signer.

---

## ER structs (`cinder_ledger`)

```
Position {
  asset_id:           u16,
  lots:               i64,
  entry_quote_lots:   i64,
  unsettled_funding:  i64,
  reserved_im:        u64,
}

OpenOid {
  client_oid:   [u8; 16],
  asset_id:     u16,
  lots_delta:   i64,
  state:        u8,          // 0 pending, 1 acked, 2 failed, 3 liquidating
}

Residual {
  asset_id: u16,
  lots:     i64,
}

UserLedger {                            // seeds ["user", user_pubkey]
  user:                Pubkey,
  free:                u64,
  reserved:            u64,
  withdrawable:        u64,
  pending_oid_count:   u8,
  nonce:               u64,
  last_funding_epoch:  u64,
  positions_len:       u8,
  positions:           [Position; 16],
  open_oids:           [OpenOid; 8],
  bump:                u8,
}

Book {                                  // seeds ["book"]
  residual_len:         u8,
  residuals:            [Residual; 32],
  phoenix_collateral:   u64,
  last_ack_slot_er:     u64,
  invariant_ok:         u8,             // 1 or 0
  halt:                 u8,
  funding_epoch:        u64,            // adapter bump_funding_epoch; users clone on init
  bump:                 u8,
}

FeeAccrual {                            // seeds ["fees"]
  phoenix_fees_paid:      u64,
  cinder_fees_accrued:    u64,
  bump:                   u8,
}
```

Empty position = `lots == 0`. Compact on close.

Permission members:

- `UserLedger`: user = `TX_BALANCES | TX_LOGS | TX_MESSAGE | ACCOUNT_SIGNATURES` (no AUTHORITY). Adapter = AUTHORITY + all view.
- `Book`, `FeeAccrual`: adapter only.

---

## L1 instructions

| Ix | Signed by | Does |
|---|---|---|
| `initialize` | admin | Config, vault_authority PDA, ReserveRoot, ATA |
| `set_adapter` | admin | rotate adapter / position_authority pubkey |
| `set_halt` | admin | write HaltFlags |
| `set_allowlist` | admin | assets |
| `post_collateral` | adapter or vault PDA | Ember wrap + Phoenix deposit `amount` |
| `pull_collateral` | adapter or vault PDA | Phoenix withdraw + Ember unwrap `amount` |
| `user_withdraw_l1` | adapter or vault PDA | pay user ATA; ER withdrawable already set |
| `write_reserve_root` | adapter or action escrow | write ReserveRoot |
| `delegate_user` | user + adapter | Delegation program CPI; validator = Config.er_validator |
| `escape_withdraw` | user | stub: error `unsupported` |

Stand-in era: adapter key **is** Phoenix authority. S9: same ixs, `invoke_signed` with `["vault-authority"]`.

---

## ER instructions

| Ix | Signed by | State |
|---|---|---|
| `init_user` | adapter + user | zero ledger; `last_funding_epoch = Book.funding_epoch`; create EphemeralPermission |
| `credit_deposit` | adapter | `free += amount` unless HALT_DEPOSIT |
| `place_order` | user | tentative lots + reserved IM + pending oid; **Book does not move** |
| `ack_phoenix_fill` | adapter | Book += filled; fee/slippage; oid acked |
| `ack_phoenix_fail` | adapter | revert tentative; oid failed |
| `bump_funding_epoch` | adapter | `Book.funding_epoch += 1` (must equal arg) |
| `allocate_funding` | adapter | accrue `unsettled_funding` or fold into free/reserved; see args |
| `liquidate_user` | adapter | flatten asset; Book -= user lots |
| `request_withdraw` | user | flat, no pending, all `unsettled_funding == 0`; free → withdrawable |
| `complete_withdraw` | adapter | after L1 pay; withdrawable -= |
| `update_book_collateral` | adapter | `phoenix_collateral = x` |
| `set_book_halt` | adapter | Book.halt |
| `close_permission` / undelegate | adapter | flat + withdrawable 0 |

### `place_order` args

`asset_id: u16`, `lots_delta: i64`, `client_oid: [u8; 16]`, `max_slippage_bps: u16`, `reduce_only: bool`, `nonce: u64`

On success the ledger tentatively applies `lots_delta` and reserves Cinder IM. Book waits for ack.

Adapter pre-trade: `intended[asset] = Book.residuals[asset] + sum(pending.lots_delta for asset)`. Rise pool checks use `intended`. Also reject if user leverage or Cinder IM would fail.

### `ack_phoenix_fill` args

`client_oid: [u8; 16]`, `filled_lots: i64`, `fee_usdc: u64`, `vwap_quote_lots: i64`

If `filled_lots != requested`, position and reserved IM use filled size.

### `bump_funding_epoch` args

`epoch: u64` — must equal `Book.funding_epoch + 1`.

### `allocate_funding` args

`epoch: u64`, `fold: bool`, `entries: Vec<{ asset_id: u16, delta_usdc: i64 }>` (max 16).

- `epoch == user.last_funding_epoch + 1` and `epoch == Book.funding_epoch`.
- `fold = false`: `unsettled_funding += delta` only (health). Empty entries = bump-only (flat user).
- `fold = true`: fold each position’s `unsettled_funding` into `free` then `reserved`; recompute Cinder IM. `delta` may be 0 if already accrued.
- Skip if `INVARIANT_BROKEN`. Still run under `HALT_ENTRIES` / `UNSAFE_POOL`.
- Acked lots only; adapter converts Phoenix quote lots → native USDC i64.

See `docs/08-funding-allocation.md`.

---

## Invariants

- I1 after ack: `Book.residuals[a] == Phoenix.base_lots[a]`
- I1 live: `Book[a] + pending[a] == Phoenix[a]`
- I2 (cash, after funding fold): `sum(free+reserved+withdrawable) == vault_ata + phoenix_collateral ± in_flight`
- I2 (between funding settles): `sum(free+reserved+withdrawable+unsettled_funding) == vault_ata + phoenix_collateral + pool_unsettled_funding ± in_flight`
- I3: pending oid < OID_TTL or fail-ack
- I4: user `reserved` ≥ Cinder IM after ack

I1/I2 fail → `INVARIANT_BROKEN`. No silent repair.

## Halt machine

- Stale mark or trader-state (>2s) → HALT_ENTRIES
- Pre-trade pool would leave `Safe` → reject that order (not always global halt)
- Phoenix `Cancellable`+ → UNSAFE_POOL, liquidate worst users first
- Venue liquidates Cinder → freeze; flatten users at venue avg; socialize shortfall from those users’ free then reserved; Book residual 0 on that asset
- Admin OPERATOR_DOWN

## Commit

Commit `ReserveRoot` (± Book), not every UserLedger every fill.
Delegated fee payer + `magic_fee_vault` before the 10th commit.
Never Magic-Action a Phoenix order.

## Out of this implementation tracker

Windowed matcher, Cinder fee, working `escape_withdraw`, session keys, isolatedOnly markets, stealth eSPL.
S9 (vault PDA + Magic Action settle) is in the tracker but after S8.
