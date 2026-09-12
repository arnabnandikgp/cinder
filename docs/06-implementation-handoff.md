# Implementation tracker

Statuses: `open` | `in_progress` | `closed`.

Update this table when you start or finish a stage. After `closed`, fill that stage’s **Post-implementation comments**.

| ID | Stage | Deliverables | Status | Success markers |
|---|---|---|---|---|
| S0 | Repo + toolchain | Workspace, pins, reference dumps, scripts stubs | open | `anchor --version` is 1.0.2; `anchor build` of skeleton programs succeeds; `docs/reference/mb-docs.md` and `phoenix-docs.md` present |
| S1 | Ledger + vault accounts | Anchor accounts and ixs from freeze, no cluster privacy yet | open | `anchor test` inits Config, UserLedger, Book, FeeAccrual; seeds match freeze |
| S2 | PER privacy (stage C ACL) | Delegate, EphemeralPermission, QFS tokens | open | User A token reads A; user B token cannot read A; adapter token reads both; traffic on `:6699` |
| S3 | Order machine without Phoenix | place / dummy ack / fail / nonce / halt bits | open | Fail-ack restores lots and free; double-nonce rejected; HALT_ENTRIES blocks place |
| S4 | Adapter skeleton | Operator token, halt mirror, in-flight registry, I1/I2 checker (Phoenix mocked) | open | Mock fill path updates Book; mock I1 break sets INVARIANT_BROKEN on Book + Config |
| S5 | Surfpool venue boot | Fork, register trader 128, delegate position_authority, Ember post/pull | open | Rise trader-state shows collateral after post; pull returns USDC to vault ATA |
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

_(fill when closed)_

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

_(fill when closed)_

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

_(fill when closed)_

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

_(fill when closed)_

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

_(fill when closed)_

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

_(fill when closed)_

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
