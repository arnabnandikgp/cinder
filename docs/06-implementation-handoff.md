# Implementation tracker

Statuses: `open` | `in_progress` | `closed`.

Update this table when you start or finish a stage. After `closed`, fill that stage’s **Post-implementation comments**.

| ID | Stage | Deliverables | Status | Success markers |
|---|---|---|---|---|
| S0 | Repo + toolchain | Workspace, pins, reference dumps, scripts stubs | closed | `anchor --version` is 1.0.2; `anchor build` of skeleton programs succeeds; `docs/reference/mb-docs.md` and `phoenix-docs.md` present |
| S1 | Ledger + vault accounts | Anchor accounts and ixs from freeze, no cluster privacy yet | closed | `anchor test` inits Config, UserLedger, Book, FeeAccrual; seeds match freeze |
| S2 | PER privacy (stage C ACL) | Delegate, EphemeralPermission, QFS tokens | closed | User A token reads A; user B token cannot read A; adapter token reads both; traffic on `:6699` |
| S3 | Order machine without Phoenix | place / dummy ack / fail / nonce / halt bits | closed | Fail-ack restores lots and free; double-nonce rejected; HALT_ENTRIES blocks place |
| S4 | Adapter skeleton | Operator token, halt mirror, in-flight registry, I1/I2 checker (Phoenix mocked) | closed | Mock fill path updates Book; mock I1 break sets INVARIANT_BROKEN on Book + Config |
| S5 | Surfpool venue boot | Fork, register trader 128, delegate position_authority, Ember post/pull | closed | Rise trader-state shows collateral after post; pull returns USDC to vault ATA |
| S6 | One-user residual hedge | Window=0 market/IOC on one allowlisted asset | open | After ack, Book lots == Phoenix lots; user reserved ≥ Cinder IM |
| S7 | Two-user net demo | Offsetting users, QFS isolation still holds | open | User A +x, user B −x; Phoenix net equals A+B; neither user reads the other |
| S8 | Cash out + reserve root | request/complete withdraw; write_reserve_root crank | open | Flat user withdraws USDC; ReserveRoot epoch increments; escape ix still errors unsupported |
| S9 | Vault PDA + Magic Action withdraw | PDA is Phoenix authority; settle via action | open | Deposit/withdraw CPI signed by vault seeds; action pay path works; stand-in key retired |

S9 stays `open` until S8 is `closed`. Windowed residual (window > 0) is **not** a tracker stage. Do not implement it.

Suggested layout:

```
cinder/
  AGENTS.md
  docs/                      # this spec pack
    reference/
      mb-docs.md
      phoenix-docs.md
  programs/cinder_vault/
  programs/cinder_ledger/
  adapter/
  scripts/stack-c.sh
  scripts/stack-a.sh
  tests/
```

Toolchain: Anchor **1.0.2**, `ephemeral-rollups-sdk` **0.16.2** (`anchor`, `access-control`), Solana **3.1.9**, Rust **1.89**, Node 24. See `AGENTS.md`.

---

## S0 — Repo + toolchain

**Do**

- Copy this spec pack to `docs/`. Put `AGENTS.md` at repo root.
- Put vendor dumps at `docs/reference/mb-docs.md` and `docs/reference/phoenix-docs.md`.
- `avm use 1.0.2`. Pin `anchor-lang = "1.0.2"` and sdk `0.16.2`.
- Two Anchor programs: `cinder_vault`, `cinder_ledger`.
- Script stubs for `mb-stack` and Surfpool.

**Tests**

- `scripts/check-toolchain.sh` asserts Anchor 1.0.2.
- `anchor build` exits 0.

**Post-implementation comments**

- Workspace: `programs/cinder_vault`, `programs/cinder_ledger`, `crates/cinder-common`, `adapter/` stub, `tests/`, `scripts/{check-toolchain,stack-c,stack-a,test}.sh`. Spec pack and `docs/reference/{mb,phoenix}-docs.md` were already in the repo.
- Pins: `Anchor.toml` `anchor_version = "1.0.2"`; `anchor-lang` / `anchor-spl` **`=1.0.2`** (bare `1.0.2` is `^1.0.2` and resolved to 1.2.0); `ephemeral-rollups-sdk =0.16.2` with `anchor` + `access-control` on the ledger; `rust-toolchain.toml` channel `1.89.0`.
- Verified: `./scripts/check-toolchain.sh` → `anchor-cli 1.0.2`, `rustc 1.89.0`. Host Solana CLI is **3.1.10** (spec 3.1.9). Did not downgrade; patch-level, `anchor build` succeeded.
- Program ids (committed under `keys/`, copied to `target/deploy/` by check-toolchain): vault `9zhBFVgk13gnYT6iVuKPGfQiAvVfr6cYQq2bY2QUzXmg`, ledger `h3Bw2xjj69JssRkaxr8Jxh6TtamvrjSxASXbfLeWyPg`.
- `anchor test` on this CLI defaults to Surfpool. S0/S1 use `anchor test --validator legacy` (see `scripts/test.sh`). Do not install Surfpool until S5.
- TS client is `@coral-xyz/anchor` 0.32.1 (MagicBlock private-counter pattern), not `@anchor-lang/core`.

---

## S1 — Accounts and instructions (no PER yet)

**Do**

Implement layouts in `docs/03-phase4-freeze.md` exactly: Config, ReserveRoot, UserLedger, Book, FeeAccrual, Position, OpenOid, HaltFlags.

Ixs that can run on a normal validator first: `initialize`, `set_adapter`, `set_halt`, `set_allowlist`, `init_user` (without permission CPI), `credit_deposit`, `place_order`, `ack_phoenix_fill`, `ack_phoenix_fail`, `request_withdraw`, `complete_withdraw`, `set_book_halt`, `update_book_collateral`, stub `escape_withdraw` → error.

Leave Delegation / EphemeralPermission CPIs to S2.

**Tests**

- Init → credit 100 USDC → place +10 lots → fail-ack → free and lots back.
- Place → fill-ack → position and reserved IM updated.
- Halt entries → place fails.

**Post-implementation comments**

- Layouts match freeze: Config / ReserveRoot (`cinder_vault`); UserLedger / Book / FeeAccrual + Position / OpenOid / Residual (`cinder_ledger`). Seeds: `config`, `vault-authority`, `reserve`, `user`+pubkey, `book`, `fees`.
- Extra vs freeze ix table: `cinder_ledger::initialize` creates Book + FeeAccrual (needed; freeze has no other creator). Adapter-signed ixs authenticate by reading vault `Config` (owner = vault program, discriminator `account:Config`).
- Ixs live: vault `initialize`, `set_adapter`, `set_halt`, `set_allowlist`, `escape_withdraw` → `Unsupported`. Ledger `init_user` (no permission CPI), `credit_deposit`, `place_order`, `ack_phoenix_fill`, `ack_phoenix_fail`, `request_withdraw`, `complete_withdraw`, `set_book_halt`, `update_book_collateral`. Delegation / EphemeralPermission left to S2.
- `place_order` is tentative; Book moves only on `ack_phoenix_fill`. Fail-ack restores lots + free. HALT_ENTRIES (and UNSAFE_POOL / INVARIANT_BROKEN / OPERATOR_DOWN) blocks place.
- IM until S6: stub notional = 1 lot → 1 USDC, Cinder IM = 1.25 × notional / 10 (`STUB_NOTIONAL_PER_LOT` in `cinder-common`). 10 lots ⇒ 1_250_000 native reserved. Replace with Phoenix mark in S6; do not invent a mark PDA.
- BPF stack: `UserLedger` / `Book` are `Box<Account<...>>` so `try_accounts` stays under 4096.
- Tests (`scripts/test.sh` → `anchor test --validator legacy`): init PDAs; credit 100 USDC; place +10 → fail-ack restores; place → fill-ack updates position, reserved IM, Book residual; HALT_ENTRIES blocks place; `escape_withdraw` unsupported. 5 passing.

---

## S2 — Stage C privacy

**Do**

- `mb-stack --reset` (or manual validator + ER + QFS).
- Delegate UserLedger / Book / FeeAccrual to local ER id `mAGicPQYBMvcYveUZA5F5UNNwyHvfYh5xkLS2Fr1mev`.
- `init_permission` on ER: user view flags; adapter AUTHORITY+view. Book = adapter only.
- Clients: `getAuthToken` against `http://127.0.0.1:6699`. Skip `verifyTeeRpcIntegrity` locally.
- Pre-fund PDAs so permission rent can be paid.

Copy patterns from `magicblock-engine-examples/private-counter/anchor`. If CPI names differ, search `docs/reference/mb-docs.md` for `CreateEphemeralPermission`.

**Tests**

- A reads A on QFS.
- B cannot `getAccountInfo` A’s ledger on QFS.
- Same read **succeeds** on raw `:7799` (proves the filter is QFS).
- Adapter token reads A and Book.

**Post-implementation comments**

- `#[ephemeral]` on `cinder_ledger`. Delegate + permission ixs live on the ledger program (it owns the PDAs). Freeze listed `delegate_user` under vault L1 because it is an L1 tx; CPI must come from the owner program.
- Split vs freeze `init_user`: L1 `init_user` creates the ledger and pre-funds `ephemeral_accounts::rent(EphemeralPermission::size_of(n))`. ER `init_user_permission` / `init_book_permission` / `init_fees_permission` run `CreateEphemeralPermissionCpi` (PDA-signed, PDA pays). Same split as private-counter.
- ACL hardcoded in the program, not client-supplied: UserLedger members = user (`TX_BALANCES|TX_LOGS|TX_MESSAGE|ACCOUNT_SIGNATURES`) + adapter (`AUTHORITY` + view). Book and FeeAccrual = adapter only. `is_private: true`. Validator must be `Config.er_validator` (`mAGicPQYBMvcYveUZA5F5UNNwyHvfYh5xkLS2Fr1mev` locally).
- Stack: `@magicblock-labs/ephemeral-validator@0.14.10` `mb-stack --reset` → base `:8899`, ER `:7799`, QFS `:6699`. `./scripts/stack-c.sh` starts it; `./scripts/test-s2.sh` deploys and runs mocha. TS client `@magicblock-labs/ephemeral-rollups-sdk@0.14.3` + `getAuthToken` against `:6699`; `verifyTeeRpcIntegrity` skipped.
- Tests (`tests/s2-privacy.ts`): A reads A on QFS; B cannot `getAccountInfo` A on QFS; same read succeeds on raw `:7799`; adapter token reads A and Book. 5 passing. S1 suite stays on `tests/s1-*.ts` + `--validator legacy` so it does not require QFS.

---

## S3 — Order machine on ER

**Do**

Enforce nonce, oid slot cap (8), allowlist, reduce-only, tentative lots, pending state 0/1/2/3, HALT bits on Book.

Still no Phoenix. Tests may call ack directly.

**Tests**

- Replay nonce fails.
- Ninth concurrent oid fails.
- Reduce-only that would increase exposure fails.
- Liquidate ix flattens one asset and is adapter-signed only.

**Post-implementation comments**

- S1 already had nonce (monotonic, must equal `ledger.nonce`), oid cap 8, allowlist, reduce-only (same sign and strictly smaller abs, or flat), tentative lots, pending 0/1/2, HALT bits, fail-ack restore. S3 adds `liquidate_user` and the remaining tests. Still no Phoenix; acks are called directly.
- `liquidate_user(asset_id)` is adapter-only (`Config.adapter`). Reverts pending oids on that asset (state → 3 liquidating), then zeros remaining lots, releases IM, and `Book -=` those lots. User signer is rejected. Halt does not block liquidation.
- Tests nested under S1 local suite (`tests/s1-ledger.ts`, `./scripts/test.sh`): replay nonce → `BadNonce`; reduce-only +1 on a long → `ReduceOnlyIncrease`; user cannot liquidate, adapter flatten zeros position + Book; ninth concurrent oid → `OidCap`. 9 passing with S1.

---

## S4 — Adapter skeleton

**Do**

Rust adapter:

- Operator `getAuthToken`
- Intended residual = Book + pending
- In-flight table with TTL
- Halt writer to Config + Book
- Phoenix client **trait** with a mock

Do not store user tokens.

**Tests**

- Mock reject → adapter sends `ack_phoenix_fail`
- Mock I1 mismatch → `INVARIANT_BROKEN`
- OID older than 15s → fail-ack

**Post-implementation comments**

- Crate `adapter/` (`cinder-adapter`): `PhoenixVenue` trait + `MockPhoenix`; `LedgerPort` + `MemoryLedger`; `OperatorAuth` holds one QFS token (`TeeAuth` / `MockTeeAuth`); no user-token map. Intended residual = Book + pending. In-flight table TTL `IN_FLIGHT_TTL_MS` (30s). Halt writer mirrors flags onto Config and Book.
- Hedge path (window=0): stale oid (> `OID_TTL_MS` 15s) → `ack_fail` without a venue call; mock reject → `ack_fail`; mock fill → `ack_fill` then I1 (`Book == Phoenix lots`). I1 miss → `INVARIANT_BROKEN` on both halt bytes and `invariant_ok = 0`. I2 helper: user cash vs vault ATA + phoenix collateral ± in-flight.
- Tests: `cargo test -p cinder-adapter` — 8 passing (fill updates Book, reject fail-acks, I1 break, oid TTL, halt mirror, operator token, intended residual, I2). Real Rise / Surfpool is S5. Live `getAuthToken` against QFS stays on the TS side until the adapter grows an HTTP client.

---

## S5 — Surfpool + Phoenix boot

**Do**

```bash
surfpool start --rpc-url https://api.mainnet-beta.solana.com
# ER + QFS pointed at localhost 8899/8900
```

Register via no-referral `build-register-ixs` / `send-register-ixs`, `maxPositions=128`, indexes 0/0. Fee payer ≠ onboarder. Authority may be the stand-in keypair.

`DelegateTrader` to `position_authority`. Rise against fork RPC. `post_collateral` / `pull_collateral` using Rise account lists (see `docs/04`; search `buildDepositIxs` in `docs/reference/phoenix-docs.md`).

**Tests**

- Trader PDA exists; snapshot collateral after post.
- Pull decreases Phoenix collateral and increases vault ATA, or queued status is explicit (no silent debit).

**Post-implementation comments**

- Stand-in vault path (`tests/s5-collateral.ts`): `post_collateral` / `pull_collateral` move USDC vault ATA ↔ stand-in ATA. 3 passing on a legacy validator.
- Fork path (`scripts/venue-boot.ts`, `CINDER_S5=1`): Rise `buildRegisterTrader` (cross, 0/0, max 128), `buildDelegateTrader` to the adapter key, `surfnet_setTokenAccount` for wallet USDC, `buildDepositIxs` / `buildWithdrawIxs`. Verified on Surfpool 1.5.0: trader PDA exists; on-chain `quoteLotCollateral` 0 → 25_000_000 → 0 after pull.
- Do **not** use `send-register-ixs` (broadcasts to Phoenix mainnet). Local fork uses `--skip-signature-verification` so the Phoenix onboarder can be dummy-signed. That flag is fork-only (stack-a starts Surfpool locally).
- `OnboardTraderDelegated` is best-effort; a second run may skip it if already onboarded. Pull that hits the global queue is treated as explicit queued (no silent debit).

---

## S6 — One-user hedge

**Do**

Allowlist one asset (SOL if the fork has it). Market/IOC only.

Flow: ER place → pool Safe check via Rise calculator → post if needed → Phoenix order signed by `position_authority` → ack fill/fail.

Cinder IM = 1.25× Phoenix IM on the **user** size. Cap leverage at 10x unless the market’s first tier is lower.

**Tests**

- Happy path: I1 after ack.
- Phoenix 0-fill: user restored, Book unchanged.
- Partial IOC: user and Book use filled lots only.
- Stale mark (>2s): HALT_ENTRIES or reject.

**Post-implementation comments**

_(fill when closed)_

---

## S7 — Two-user net (the demo)

**Do**

Two ledgers, two QFS tokens. Offsetting market orders. Phoenix position equals sum of acked lots (window=0 still sends both hedges; net may be ~0 if equal size).

**Tests**

- Phoenix lots == lots_A + lots_B.
- QFS isolation still holds after trades.
- I2: sum(user cash) == vault ATA + phoenix_collateral ± in_flight.

This is the local win.

**Post-implementation comments**

_(fill when closed)_

---

## S8 — Withdraw + reserve root

**Do**

First win: withdraw only if flat. `request_withdraw` then L1 pay then `complete_withdraw`. Pull from Phoenix if vault ATA short.

Commit crank: `write_reserve_root` every 20 fills or 30s. Delegated fee payer + `magic_fee_vault` before the 10th commit.

`escape_withdraw` remains `unsupported`.

**Tests**

- Flat withdraw credits user ATA and zeros withdrawable.
- Open position withdraw rejected.
- Root epoch bumps; hash changes after a credit.

**Post-implementation comments**

_(fill when closed)_

---

## S9 — Vault PDA (after S8)

Replace stand-in authority with `vault_authority` PDA. Re-register if required. CPI Ember/Phoenix. Magic Action `settle_user_withdraw` with `escrow_auth` bound to that PDA. Retire adapter-signed `user_withdraw_l1` as the only path (keep as admin fallback if needed).

**Success markers:** PDA-signed post/pull; user withdraw via commit+action; stand-in key cannot withdraw.

**Post-implementation comments**

_(fill when closed)_
