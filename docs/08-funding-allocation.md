# Funding allocation — proposal (not freeze)

Status: **agreed** (P-F2 two-phase health/cash, Book-owned epoch, I2 amendment).  
Freeze patched in `03-phase4-freeze.md`. Tracker slice in `06` (not S10).

Phoenix is the venue clock. Cinder does not invent a funding rate. Cinder **attributes** Phoenix’s accumulator across private user books so longs and shorts pay each other on **gross** size, while Phoenix only ever funds Cinder’s **residual**.

`allocate_funding` and `bump_funding_epoch` are implemented. Adapter `crank_funding` accrues every Phoenix interval and folds when venue collateral actually moves.

---

## 0. What changed vs the first draft

Another review accepted the overlay and rejected **hourly `free` moves**. That draft’s worked example treated Phoenix collateral as moving in the same breath as user cash. That is only true when the **venue actually settles**, not at each hourly snapshot. Hourly `free` debits would falsify frozen I2 until the 24h (or interact-triggered) collateral update.

This file is the plan to implement. Tracker: treat as a **slice after S9**, not a new “S10” (S10 was the removed window>0 slot).

---

## 1. Phoenix facts (vendor) — unchanged, still correct

From `docs/reference/phoenix-docs.md` (`phoenix/margin-and-risk/funding-rate.md`):

| Item | Phoenix |
|---|---|
| Sign | mark > index → **longs pay shorts**; opposite → shorts pay longs |
| Accrual | Continuous, **hourly snapshots** (`fundingIntervalSeconds`, typically 3600) |
| Collateral settle | **~24h** (`fundingPeriodSeconds`) **or** when the trader interacts enough that pending is realized into the collateral view |
| Pending | Hits **health** immediately; `unsettledFundingOwed` (+ receive / − owe) |
| Payment | `(current accumulator − snapshot accumulator) × position size` |
| Who is charged | The one Cinder trader PDA `(0,0)` |

Adapter converts Phoenix **quote lots** → **native USDC i64** once, using that market’s quote decimals. On-chain stores USDC only. Do not mix lots and USDC in the program.

---

## 2. Overlay identity — unchanged

```
sum_users (acked_lots_i × Δacc)  ==  residual × Δacc  ==  Phoenix funding on the pool
```

I1 on **acked** lots makes this arithmetic. Pending `place_order` size does not pay (P-F1).

Fully offset (+10 / −10): venue residual 0, Alice pays Bob **inside Cinder**, Phoenix collateral unchanged until/unless residual is nonzero.

Partial net (+10 / −4): users net −6×rate; Phoenix takes 6×rate from the trader **when collateral settles**. Until then, both sides hold the same *unsettled* number (see I2).

---

## 3. Policy (P-F*)

| ID | Policy | Notes |
|---|---|---|
| P-F1 | **Acked lots only** | Unchanged |
| P-F2 | **Two-phase.** Each interval: write `unsettled_funding` only (health). **Do not** fold into `free`/`reserved` until Phoenix collateral / accumulated funding actually moves. Then fold (P-F7) and `update_book_collateral`. | **Amended.** This is the design bug in the first draft. |
| P-F3 | `unsettled_funding` is signed native USDC, + receive / − owe, **the health term**, not a debug leftover | Amended vs “cash is only in free” |
| P-F4 | No multi-epoch catch-up in one ix. Adapter loops. Gaps rejected **after** users are aligned to `Book.funding_epoch` | See §4 |
| P-F5 | Adapter-signed only | Unchanged |
| P-F6 | One ix per user per epoch, `Vec<{asset_id, delta_usdc}>`, cap 16 | Unchanged |
| P-F7 | Drain `free` then `reserved` on the **fold** only. Hourly accrue only moves `unsettled_funding`. If equity < Cinder MM → `liquidate_user` that asset. Do not halt the pool. | Amended timing |
| P-F8 | After **fold**, recompute Cinder IM = 1.25 × Phoenix IM(**user** size) via Rise `createMarginCalculator`. Stub IM is not legal on a live fork. Shuffle `free`↔`reserved`. | Amended |
| P-F9 | Run under `HALT_ENTRIES` and `UNSAFE_POOL`. Skip only `INVARIANT_BROKEN`. | Unchanged |
| P-F10 | Independent of windowed netting | Unchanged |
| P-F11 | Withdraw (flat) requires all `unsettled_funding == 0` (fold first if needed) | New |
| P-F12 | `init_user` copies `Book.funding_epoch`. Flat users still get a bump-only allocate so they do not skip epochs. | New |

**Not in scope:** Cinder-native rate, vault ATA payout (would double-count venue settlement), Magic Action, on-chain residual sum (cannot check `sum(users)==pool` in a per-user ix). Trust model = `credit_deposit` (A+C): adapter deltas + off-chain recon + I2.

---

## 4. Epoch clock on `Book`

`last_funding_epoch` **only** on the user plus “reject gaps” breaks:

- `init_user` mid-stream (`last == 0`, live epoch 47)
- skipping a flat user for a few intervals

**Freeze patch:** add `Book.funding_epoch: u64` (starts at 0).

**Owner:** adapter advances Book with a tiny `bump_funding_epoch { epoch }` requiring `epoch == Book.funding_epoch + 1`, then every live user (including flat, empty `entries`) catches up with `allocate_funding`.

Program checks on allocate:

- `epoch == user.last_funding_epoch + 1`
- `epoch == Book.funding_epoch`

`init_user`: `UserLedger.last_funding_epoch = Book.funding_epoch` (no historical funding for a book that did not exist).

Do not let allocate itself bump Book (two writers). Do not invent extra PDAs.

---

## 5. Health vs cash vs I2

Phoenix: pending hits **health** now; **collateral** later.

Cinder equity for MM / liq (adapter; on-chain liq remains adapter-triggered):

```
free + reserved + sum(unsettled_funding) + uPnL
```

`uPnL` is off-chain (mark vs `entry_quote_lots`) until we store a mark. Do not invent a mark PDA.

**I2 between settles** (amend freeze):

```
sum(free + reserved + withdrawable + unsettled_funding)
  == vault_ATA + phoenix_collateral + pool_unsettled_funding ± in_flight
```

`pool_unsettled_funding` = Rise/Hawkeye `unsettledFundingOwed` on the Cinder trader, same sign, native USDC. After fold, extra terms go to 0 and I2 collapses to the original cash equation.

**Fold trigger:** adapter sees Phoenix collateral / accumulated funding **actually change**, not a wall-clock 24h. Then:

1. Fold each user’s `unsettled_funding` into `free` then `reserved` (P-F7).
2. `update_book_collateral`.
3. Optional: a `fold_funding` flag on allocate or a small `fold_funding` ix so accrue and fold cannot be confused. Prefer a **flag on `allocate_funding`** (`fold: bool`) over a third ix unless bump+allocate+fold gets too messy.

Hourly accrue: `fold = false`, only `unsettled_funding += delta`.  
Settle: `fold = true`, `delta` may be 0 if already accrued, just fold.

---

## 6. Instruction (freeze patch when agreed)

```
bump_funding_epoch(epoch: u64)           // adapter, Book mut; epoch == Book.funding_epoch + 1

allocate_funding(
  epoch: u64,
  fold: bool,
  entries: Vec<{ asset_id: u16, delta_usdc: i64 }>,  // max 16
)
```

Signed by adapter. Accounts: Config, UserLedger, Book.

Accrue (`fold = false`): apply `delta_usdc` to `unsettled_funding` only; bump user epoch.  
Fold (`fold = true`): for each position, move `unsettled_funding` into cash (P-F7), zero (or leave dust 0), recompute IM (P-F8), bump user epoch if this interval was not yet applied.

`request_withdraw`: also require every position `unsettled_funding == 0`.

---

## 7. Adapter crank

1. Read Phoenix Δacc / `unsettledFundingOwed` (quote lots → native USDC).
2. `bump_funding_epoch`.
3. For each user (including flat): `allocate_funding(epoch, fold=false, entries)`.
4. Dust: truncate toward zero per user; `|sum(user deltas) − pool_delta| ≤ 1000` native; log; do not halt on 1-unit dust.
5. If equity < Cinder MM (including unsettled): `liquidate_user`.
6. When venue collateral actually moves: `allocate_funding(..., fold=true)` for each user, then `update_book_collateral`.

Cadence: at least once per Phoenix interval (~1h), and again when a hedge makes Phoenix apply pending.

---

## 8. Worked example (mark > index, longs pay)

`1 lot × Δacc = 1_000` native this interval.

**Fully offset, hourly:** Alice unsettled −10_000, Bob +10_000. `free` unchanged. Phoenix collateral unchanged. I2: both sides include ±10_000 unsettled and `pool_unsettled = 0`.

**Partial net +6, hourly:** Alice −10_000, Bob +4_000. User unsettled net −6_000. `pool_unsettled_funding = −6_000`. Collateral not moved yet. I2 holds.

**Same book, fold:** Alice `free −= 10_000` (or reserved), Bob `free += 4_000`, unsettled zeros. `phoenix_collateral −= 6_000`. I2 original cash form.

---

## 9. Tests (when implementing)

- Accrue does not change `free`; fold does.
- I2 with unsettled terms between fold; original I2 after fold.
- Two users +10/−10 accrue: sum free unchanged, unsettled opposite.
- Replay epoch fail; gap fail; new user cloned to `Book.funding_epoch`.
- Flat user bump-only.
- Withdraw rejected while unsettled ≠ 0.
- Fold drain free then reserved; IM reshuffle with **user-size** Cinder IM (calculator, not stub, in fork tests).
- Dust under 1000 does not halt.

---

## 10. Build slices (after **agreed**)

1. Patch `03` (Book.funding_epoch, I2, allocate/bump args, withdraw gate, init_user). Note in `06` as a follow-on slice, **not S10**.
2. Program: `bump_funding_epoch`, `allocate_funding` accrue+fold, tests on local validator.
3. Adapter: `crank_funding` + `quote_lots_to_usdc` + I2 recon (canned Δacc in unit tests; Rise fills `FundingInterval` at the process edge).
4. Fork smoke: `CINDER_FUNDING=1 ./scripts/test-funding.sh` (`loadFundingInterval` vs a funded fork trader).

---

## 11. Consensus calls — **answered**

| Q | Call |
|---|---|
| Immediate vs two-phase | **Health immediate, cash on venue settle** (P-F2 amended). |
| Trusted deltas vs on-chain residual | **A+C.** |
| Gap policy | **Reject gaps**, users aligned to `Book.funding_epoch`, crank flat users. |
| Dust cap 1000 native | **Yes.** |
| Halt | Funding under `UNSAFE_POOL` and `HALT_ENTRIES`. Skip only `INVARIANT_BROKEN`. |

Mark **agreed** at the top of this file when you want implementation.
