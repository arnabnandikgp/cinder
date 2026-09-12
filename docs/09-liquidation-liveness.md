# Liquidation liveness — problem and plan (not freeze)

Status: **agreed**. Freeze patched in `03-phase4-freeze.md`. Tracker follow-on in `06` (not S10).

---

## 0a. What changed vs the first draft of this file

A second planning pass agreed the gap and the architecture, then rejected two halt/scan rules that would have recreated the quiet-crash hole, plus a few implementation defaults.

| Item | First draft | Now |
|---|---|---|
| P-L4 I1-safe pending-liq | Proposed | **Locked.** Immediate `Book -=` is rejected. L2 before L3. |
| P-L7 stale mark | `HALT_ENTRIES` and **no new flatten** on stale | **Locked table:** soft-stale still drains the **already-queued** set from the last fresh print. Never halt entries **and** skip liq. |
| Heartbeat write | Implied every 50 ms scan | **Not load-bearing.** In-process scan stays 50 ms. ER write of `last_scan_ms` defaults to ~1 s so it does not contend with flatten on shared `Book`. While ER txs are fee 0, writing faster is allowed; it is not a correctness issue. |
| UNSAFE_POOL | Default: flatten everyone below Cinder IM once `Cancellable+` | **MM first**, then IM **only until** Rise says `Safe`. |
| Positive uPnL | Count in full for v0 | **Losses in full; haircut gains** (Rise `uPnL risk factor` if wired, else 50%). |
| Health helper | Same formula, two call sites | **One function** for scanner and `crank_funding`. |
| Q1–Q7 | Open | **Called** in §8. |

---

## 0. What changed vs “we already have liquidate_user”

D3 says Cinder liquidates **users** at Cinder MM, before Phoenix can liquidate the **pool**. The instruction exists. The **liveness** that makes D3 true does not. Combined with health that is not mark-driven in the running adapter, a quiet book can sit underwater until Phoenix liquidates Cinder itself. That is not a missed optimization. It is how one user’s loser becomes everyone else’s loss.

This file: (1) states the gap against the freeze and the code, (2) shows why it is disastrous on a one-cross-PDA broker, (3) plans the correct scanner so we actually use MagicBlock’s ~10 ms slots and zero-fee delegated ER transactions instead of treating liquidation as a side effect of hedge or hourly funding.

`liquidate_user` is an adapter-signed ER instruction that flattens one asset. That is a **handler**, not a **system**. Liveness is “who looks at marks, how often, with what health formula, and what happens to I1 / Phoenix residual after the flatten.”

Today the handler is wired to:

- a direct adapter call (tests / manual)
- `crank_funding` after a Phoenix **hourly** accrue, using **cash + unsettled** and **stub IM**, not mark-to-market equity vs Cinder MM

It is **not** wired to mark movement, quiet books, Phoenix `Cancellable+`, or adapter-death.

`MARK_STALE_MS = 2000` is a **hedge gate**. Stale mark → `HALT_ENTRIES` on `hedge_pending`. It is not a scan period. It does not flatten anyone.

---

## 1. Verified MagicBlock facts (do not treat as folklore)

Checked 2026-09-12 against live MagicBlock docs + the installed skill fee snapshot.

| Fact | Source | How Cinder must use it |
|---|---|---|
| ER **slot time is 10 ms** (Solana ~400 ms) | [MagicBlock FAQ](https://docs.magicblock.gg/pages/ephemeral-rollups-ers/introduction/faq), dated 2026-08-05 | Scan and `liquidate_user` can run on ER cadence. **Do not** use slot count as a wall clock. The same FAQ says slot times are **not guaranteed** and may change. |
| Marketing: “sub 10 ms latency” / skill diagram ~10–50 ms E2E | [Why ER](https://docs.magicblock.gg/pages/ephemeral-rollups-ers/introduction/why); skill architecture | Treat **10 ms as the slot**, ~tens of ms as observed E2E. Phoenix marks still live on L1 (~400 ms). |
| **Ordinary ER transaction fee = 0** in the current release | [Fees, commits, refunds](https://docs.magicblock.gg/pages/ephemeral-rollups-ers/introduction/fees-and-commit-economics); skill `fees-and-commit-economics.md` (validator `SCHEDULING_FEE = 0`) | While UserLedger / Book stay **delegated**, every scan write and every `liquidate_user` is free. **Do not undelegate to liquidate.** |
| Session + commit charges hit at **undelegation** (300k lamports session; 100k per commit after the first), capped by delegation PDA balances | Same fee doc | Committing every liquidation would be the expensive path. Keep ledgers delegated; commit `ReserveRoot` (± Book) on the existing fill/time cadence, accelerated after a liq. |
| No fee-vault path: plain commit hard-fails at nonce 10. Fee-vault path: live extra fee from commit 26 | Same | Already freeze: delegated fee payer + `magic_fee_vault` before the 10th commit. Liquidation must not invent a third commit strategy. |
| ER tx size **64 KB** (base layer 1,232 B); CU caps match Solana (200k/ix, 1.4M/tx) | FAQ / runtime limits | Batching several liquidations in **one** ER tx is possible for CU/accounts, not for fee. Book is a shared writable → parallel txs that all write Book will serialize or fail. |
| Built-in **cranks** (`ScheduleTask`, interval in ms) | Skill `cranks.md` | Use only for work the ER can see. **Phoenix mark is off-chain / L1.** A crank cannot replace the adapter ingest. Optional: ER watchdog on a timestamp the adapter already writes. |
| PER / QFS: adapter is a member on every `UserLedger` | Q9-C, freeze | The operator can read every private book and sign `liquidate_user` without the user’s client. That is the local-win trust model (same class as HyperLink’s enclave). |

**Pricing caveat:** “Free ER txs” is **current-release** behavior, not a protocol promise forever. Session/commit at undelegation still exist. Design so that **raising ER tx price later** degrades to “still cheap vs L1,” not “the scanner is economically impossible.” Do not undelegate as a liquidation step even if ER txs later cost something; undelegation is the bill.

---

## 2. What D3 actually requires

Locked (`docs/01-decisions.md` D3, `docs/07-context-for-builders.md`):

1. User IM/MM = **1.25× Phoenix** on **that user’s size**, not on the residual.
2. Liquidate the user at **Cinder MM**.
3. Pool buffer = 20% of Phoenix IM(residual), floor 50 USDC — slack for the **shared** account, not a substitute for user liq.
4. Phoenix `Cancellable+` on the pool → `UNSAFE_POOL`, **liquidate worst users first**.
5. If the **venue** liquidates Cinder → freeze, flatten at venue avg, socialize shortfall. That is the **failure** state of D3, not an alternative to D3.

Phoenix pool health (`docs/reference/phoenix-docs.md`, account-health):

| Phoenix tier | Meaning for Cinder |
|---|---|
| `Safe` | Effective collateral ≥ IM. New hedges allowed. |
| `AtRisk` | Below IM, not yet cancellable. |
| `Cancellable` | Freeze new risk (`UNSAFE_POOL`). Start worst-first user liq. |
| `Liquidatable` | Phoenix may market-liquidate **Cinder’s trader PDA**. D3 has already lost. |
| `BackstopLiquidatable` / `HighRisk` | Fire sale / ADL. Socialize. |

Phoenix health includes **deposited collateral + uPnL + unsettled funding** (`docs/reference/phoenix-docs.md`, margin math). Cinder user health must include the same classes of terms (`docs/08-funding-allocation.md` §5 already wrote this). A cash-only check is not D3.

### The only time budget we have

The 1.25× overlay is **not** a fee and **not** extra size. It is a **price gap**.

On an **unnetted** book (Alice is the residual; Bob is flat):

- Cinder must flatten Alice when `equity_Alice < 1.25 × Phoenix_MM(Alice)`.
- Phoenix may liquidate Cinder when `equity_pool < Phoenix_MM(residual)` ≈ `Phoenix_MM(Alice)`.

The entire “we go first” budget is **0.25 × Phoenix_MM(user)** of mark move on that size. Phoenix MM is **strictly below** Phoenix IM (otherwise `Liquidatable` would equal leaving `Safe`). First-tier Phoenix IM is on the order of `notional / ~15`. A 25% haircut of a number that is already a few percent of notional is **about 1% of notional or less**. SOL (and most perps) print that in seconds in a shock, sometimes in one L1 slot.

Do **not** invent a Phoenix MM constant in this file. Live MM comes from Rise `createMarginCalculator` / Hawkeye. The plan is: measure it, then require scan latency **≪ time for mark to cross 0.25 × that MM**. Hourly is not in the same universe as that budget. 2 s (stale-mark halt) is already large versus a 10 ms ER. 50–100 ms is the right order once a fresh mark exists.

On a **netted** book (Alice +10, Bob −9): Phoenix MM is on **+1**. Alice can be deep through Cinder MM while the pool is still `Safe`. Watching Phoenix trader-state is **necessary and not sufficient**. The scanner must walk **user** books. That is the privacy/risk trade we accepted in `07`.

---

## 3. Current gap (spec vs code)

### 3.1 What the freeze already says

| Item | Freeze / 08 | Code today |
|---|---|---|
| `liquidate_user` | Adapter-only; flatten asset; `Book -=` lots | Implemented (`programs/cinder_ledger`). Halt does not block it. User signer rejected. |
| Halt: stale mark / trader-state > 2 s | `HALT_ENTRIES` | `hedge_pending` only. No scan. Existing positions stay. |
| Halt: Phoenix `Cancellable+` | `UNSAFE_POOL`, liquidate worst first | `pool_health` gates **new hedges**. No worst-first walk. |
| Halt: venue liq of Cinder | Freeze; flatten at venue avg; socialize | Not implemented (terminal path). |
| Funding P-F7 | After accrue, if equity < Cinder MM → `liquidate_user` | `crank_funding` compares `user_equity` to **`stub_cinder_im`**, not Cinder MM from Rise, and **drops uPnL**. |
| 08 equity | `free + reserved + unsettled_funding + uPnL` (uPnL off-chain vs `entry_quote_lots`) | Adapter `user_equity` = `free + reserved + unsettled` only. |
| Mark | No mark PDA in 08 (“do not invent”). `entry_quote_lots` exists for uPnL. | No mark ingest loop. `mark_observed_at_ms` is a hedge timestamp. |
| I1 | After ack, `Book == Phoenix`. Live: `Book + pending == Phoenix`. | `liquidate_user` subtracts Book **immediately**, with no pending oid and no Phoenix flatten. **I1 is false until a later hedge that does not exist as a dedicated path.** |

### 3.2 Call graph (what actually fires liq)

```
user place_order  ──► hedge_pending ──► stale mark? HALT_ENTRIES (no liq)
                      └── Phoenix IOC ──► ack fill/fail

Phoenix hourly funding ──► crank_funding ──► cash+unsettled < stub IM? liquidate_user

adapter tests / manual ──► liquidate_user

mark tick, quiet book, Cancellable+, adapter crash ──► nothing
```

There is **no** `scan_users` / `crank_liquidations` / mark-driven MM walk in `adapter/`.

### 3.3 Two independent bugs (do not collapse them)

**Liveness:** even a perfect health formula is useless if it runs only when someone trades or once an hour.

**Health:** even a 10 ms loop is useless if equity ignores uPnL. A user can be insolvent on mark and still look “fine” on `free+reserved+unsettled` until funding or a flatten realizes the loss.

08 already named the health formula. The adapter did not implement that formula in the only place that currently calls `liquidate_user`.

### 3.4 I1 hole on the current flatten

`place_order` is tentative; Book moves on `ack_phoenix_*`. `liquidate_user` zeros the user and **moves Book in the same ix**. Phoenix still holds the old residual. Until the adapter sends a reducing IOC and acks it, I1 is broken. The freeze says I1 fail → `INVARIANT_BROKEN`, no silent repair.

So the current ix is the wrong shape for a live scanner. Fixing liveness without fixing this will **trip the invariant halt in the exact second we need to flatten**. That is how a “safety” path becomes a freeze.

---

## 4. How this becomes disastrous

All scenarios assume the honest-operator local win (adapter key is the liquidator). We are not talking about a malicious adapter stealing books. We are talking about **the pool eating a user loss because nobody looked in time**.

### 4.1 Quiet book, unnetted whale, mark shock (the canonical kill)

Alice deposits 1,250 USDC and is long ~10,000 USDC notional (inside the 10× cap). Bob is flat. Phoenix sees Cinder long ~10k. No further `place_order`. No funding hour for 50 minutes.

Mark dumps ~2%. Alice’s uPnL is about −200 USDC. Cash buckets on the ledger have not moved. `crank_funding` has not run. `hedge_pending` has nothing to do.

- Cinder’s intended line: equity including uPnL vs 1.25× Phoenix MM. She is through it.
- Implemented path: nobody liquidates.
- Phoenix: same mark hits the **same** residual. The 0.25× MM gap is gone in seconds. Pool enters `Cancellable` then `Liquidatable`. Phoenix liquidates **Cinder**, not Alice.

From the outside this looks like “the prime broker blew up.” Internally it was one user. Privacy (one PDA) converted Alice into a public pool liquidation.

### 4.2 Netted books: Phoenix stays Safe while Cinder is already insolvent

Alice +10, Bob −9. Phoenix residual +1, cheap IM, still `Safe`. Alice is through Cinder MM on **gross** size; Bob is healthy.

If we only watch Phoenix:

- We do not halt.
- We do not liquidate Alice.
- Bob’s uPnL is a claim on a pool whose other side is Alice’s hole.

When Alice is finally flattened (or when Phoenix eventually sees a residual after Bob exits), the hole is real cash. Freeze terminal path: socialize from Alice’s free then reserved, then everyone else. Healthy users pay for a scan we never ran. This is exactly the paragraph in `07`: “If Alice blows through Cinder’s books and we are slow, Phoenix can liquidate Cinder, which socializes Alice onto every other user.” Slow internal liq socializes even **without** a venue liq, because netting hid her from Phoenix.

### 4.3 HALT_ENTRIES is not a liquidation

Stale mark (>2 s) blocks **new** `place_order` / hedges. Open positions keep their market risk. In a feed incident we have **more** reason to flatten weak books (or freeze the pool honestly), not less. Today we do the opposite: stop the only flows that even incidentally look at health, leave leverage on.

If the adapter also treats “stale mark” as “cannot compute MM, skip scan,” we halt entries **and** skip liq. That is the worst pair.

### 4.4 Adapter process death

The liquidator is a server. PER keeps producing 10 ms slots; UserLedgers stay delegated; Phoenix keeps marking. If the process dies:

- no funding crank
- no hedge
- no liq
- no `OPERATOR_DOWN` unless something **on ER** trips it

`OPERATOR_DOWN` exists as a halt bit. Nothing sets it from missed scans. Users can still be sitting in `place_order` until something else writes halt. Phoenix does not care that our operator is down.

### 4.5 Adverse selection / option on the pool

Max leverage, pay Cinder IM, wait. If the market is quiet, the user is a free option on Cinder’s capital: downside is the deposit, upside is the perp. If we only liquidate on funding or on the next trade, a crash between those events is a transfer from the pool to the user (capped at zero on their account) and then to whoever Phoenix socializes onto.

This is how every pooled venue without a live MM engine dies: not one bug, a **strategy**.

### 4.6 Funding-hour false comfort

P-F7’s “if equity < MM, liquidate” is correct as a **funding** hook (funding **does** change health). It is not a mark engine. Using it as the de facto scanner implies “users only go bankrupt at :00.” They do not. Funding can even **help** a losing long for an hour (shorts pay) while mark has already killed them.

Implemented funding liq also uses stub IM as the threshold. Stub IM is a size×1 USDC stand-in from S1. On a live fork that number is not Phoenix MM.

### 4.7 After Phoenix has already liquidated Cinder

Freeze: flatten users at **venue average**, socialize shortfall. Healthy users get a fire-sale mark. I1 is reconstructed by zeroing the residual. Privacy is intact (still one PDA) and economically we have become a mutualized loss pool. That is the disaster D3 was written to avoid. A scanner that “usually runs sometime this hour” does not prevent it.

### 4.8 Scale: 10 ms unused is the own-goal

Suppose 80 users, one mark print, 12 underwater. ER can take 12 `liquidate_user` txs in ~120 ms at 10 ms slots, fee 0, still delegated, still private. L1 Phoenix flatten of the **net** change is one (or a few) IOC(s) at ~400 ms. That is the architecture we already paid for (PER + one residual).

What we do instead: wait for an hour or a coincidental hedge, then maybe flatten, then maybe notice I1 is broken. We bought a 10 ms private ledger and attached a 3600 s risk engine.

---

## 5. Non-negotiables for any fix

Carry the standing orders. Do not “solve” liveness by reopening them.

- One Phoenix cross PDA `(0,0)`. No isolated children. Skip `isolatedOnly`.
- Phoenix sees Cinder net only. User books stay on ER `UserLedger`.
- Window = 0. Book still moves on **Phoenix ack**, not on user place. Liquidation must obey the same ack discipline (see §6.3).
- Do not attach Phoenix flatten to a Magic Action / commit.
- Do not store user TEE/QFS tokens on the server. Adapter holds one operator token.
- Do not implement session keys, Cinder fees, invite graph, stealth eSPL, or working `escape_withdraw`.
- Do not undelegate UserLedger/Book to liquidate.
- Honest operator: adapter is the only liquidator. Permissionless liquidators would need to read private books or we would leak them.

---

## 6. Implementation plan (correct manner)

**Principle:** Phoenix mark and trader-state are L1 / Rise (~400 ms). User MM is PER (~10 ms, $0 txs). The adapter is a **hot loop** that ingests the slow clock and **executes** on the fast clock. Hedge and funding remain their own cranks. Liquidation is not a side effect of either.

Do not use MagicBlock scheduled cranks as the mark source. They cannot see Phoenix. Use them, if at all, as a **watchdog** on a timestamp the adapter writes on ER.

### 6.1 Policy table (proposed P-L*)

| ID | Policy | Notes |
|---|---|---|
| P-L1 | **Health formula is 08’s, with a gain haircut.** `equity = free + reserved + unsettled_funding + uPnL*`. Liquidate when `equity < Cinder_MM`. | `uPnL*`: losses in full; positive uPnL × Rise `uPnL risk factor` if wired, else `UPNL_GAIN_HAIRCUT_BPS = 5000`. Cinder_MM = `USER_MM_MULT_BPS` × Phoenix MM(**user** size at mark) via Rise. Stub IM is illegal on a live fork (P-F8). **One helper** for scanner and `crank_funding`. |
| P-L2 | **Independent scan loop.** Not inside `hedge_pending`, not only inside `crank_funding`. Funding still **may** liquidate after accrue (P-F7) as a second hook. | Quiet books must be first-class. |
| P-L3 | **Cadence.** On each **fresh mark print**, with a min interval of `SCAN_INTERVAL_MS = 50`. | Phoenix will not print 10 ms marks. 10 ms is for **bursting** N liquidations after one print. In-process rank is free; this is not an ER write rate. |
| P-L4 | **I1-safe flatten (locked).** User flatten is tentative (pending liq oid). `Book` moves on `ack_phoenix_fill` of the reducing hedge, same as place. Live I1 = `Book + pending_place + pending_liq`. Do **not** set `INVARIANT_BROKEN` because a liq is in flight. | **Load-bearing freeze patch.** See §6.3. Do not implement the existing ix “more often.” |
| P-L5 | **Worst-first, two lines under `UNSAFE_POOL`.** Normal: flatten `equity < Cinder_MM`. If pool is `Cancellable+`: (1) MM line, worst first; (2) if still not `Safe`, Cinder IM line, worst first, **only until** Rise says `Safe` (or no such users). Stop. | Do not flatten every user below IM the moment the pool is `Cancellable`. That socializes people who are not insolvent. |
| P-L6 | **Stay delegated.** `liquidate_user` / pending-liq / ack are ordinary ER txs (fee 0 today). Commit `ReserveRoot` on existing 20 fills / 30 s **and** after any liquidation that changed cash. | Do not commit every scan. Rank/QFS reads must not write Book. |
| P-L7 | **Stale mark does not skip queued flattens (locked).** See table in §6.7. Soft stale: `HALT_ENTRIES`, still drain the queue from the last **fresh** print; do not newly classify off the stale tick. Hard stale / scan miss: `OPERATOR_DOWN \| HALT_ENTRIES`, still try the already-queued set + ingest. | The first draft’s “no new flatten on stale” recreated §4.3. Never halt entries **and** skip liq. |
| P-L8 | **Missed scan.** If no successful **in-process** scan in `SCAN_DEAD_MS` (2000): `OPERATOR_DOWN \| HALT_ENTRIES`. Heartbeat on `Book.last_scan_ms` proves this on ER. | Adapter death becomes a public halt, not a silent hole. Write cadence is P-L9, not the scan cadence. |
| P-L9 | **No new mark PDA.** uPnL stays adapter-side vs `entry_quote_lots` (08). Heartbeat is `Book.last_scan_ms` only. Default ER write every `HEARTBEAT_MS = 1000` (or `SCAN_DEAD_MS / 2`). | **Not load-bearing.** Scan/rank every 50 ms in process. Do not let heartbeat txs starve flatten/ack on shared Book. While ER fee = 0, faster writes are allowed; they do not change the 2 s death detector much. |
| P-L10 | **Phoenix flatten is L1 IOC**, adapter `position_authority`, never a Magic Action. | Same as hedge. Residual after N user flattens may net; send **net** IOC, not N venue orders when they cancel. |
| P-L11 | Run under `HALT_ENTRIES` and `UNSAFE_POOL`. Skip only `INVARIANT_BROKEN` (same spirit as P-F9). | We must be able to flatten when entries are already halted. |
| P-L12 | First allowlist still **one** asset. Per-asset flatten is enough. Multi-asset portfolio MM is out of this slice. | Do not generalize `liquidate_user` to “all assets” until a second market exists. |

### 6.2 Maximize 10 ms and free ER txs (concrete)

**Ingest (slow clock):**

1. Subscribe Phoenix mark + trader-state (Rise WS / Hawkeye). This cannot be faster than L1.
2. On each fresh mark (or every 50 ms if the WS is hot): compute per-user uPnL + Cinder MM **off-chain** (QFS reads of UserLedgers, operator token). Reads are not ER txs.
3. Users with `equity < Cinder_MM` go on a priority queue.

**Execute (fast clock):**

4. For each queued user, submit `liquidate_user` (after §6.3: the tentative flatten ix) on the **ER / QFS** endpoint. Fee 0. ~10 ms slot. Do **not** wait for Phoenix between users except for Book lock: Book is shared, so **serialize** ER flatten txs (or batch in one 64 KB tx if CU fits).
5. **v0: one Phoenix IOC per pending liq oid**, `lots = that oid’s lots_delta` (already signed: long flatten is negative). Do not aggregate fills across users — a partial IOC maps to one oid (leftover lots restore on that user only). Ack fill/fail on ER. I1 live: `Book + filled_not_yet_acked == Phoenix`.
6. If Phoenix rejects / 0-fills: fail-ack restores user size (same as place fail). Then `UNSAFE_POOL` and retry; do not zero Book while Phoenix still has the risk.

**Why this uses PER instead of fighting it:**

- 12 underwater users: ~12 × 10 ms ≈ 120 ms of private flattens, $0, still in TEE.
- One public residual IOC on Phoenix (~400 ms) for the net.
- 100 users underwater: ~1 s of serial ER txs if one-ix-per-tx. Still better than an hour. If CU allows, a batched `liquidate_users` (review item) cuts that to a handful of 10 ms slots.
- **Do not** optimize for L1 fees by skipping users. ER flatten txs are free today. Skip only when health is fine.
- **Do not** write Book on a rank that found nobody, except the heartbeat (P-L9). Rank is QFS reads + memory.

**Heartbeat vs flatten (why this is not a fee argument):**

`SCAN_DEAD_MS` is 2 s. Writing `last_scan_ms` every 50 ms vs every 1 s changes death detection by ~1 s. That is not the quiet-crash hole. The reason **not** to heartbeat every scan is that `Book` is the same writable account as `liquidate_user` / ack — a 50 ms write loop can queue behind or delay actual flattens. Default `HEARTBEAT_MS = 1000`. While ER fee = 0 we *may* write faster; do not treat that as a spec requirement and do not let it starve liq ixs. Commit/undelegate remains the MagicBlock bill, not this write.

**What not to spam:**

- Do not `write_reserve_root` per user. Commit charges and nonce limits are the real MagicBlock bill.
- Do not undelegate/redelegate around a liq. Session fee + clone latency + privacy hole.
- Do not publish per-user liq on L1. Phoenix only sees residual change.

### 6.3 I1-safe liquidation (locked freeze amendment)

Mirror the order machine. Names are illustrative.

**Today (wrong for I1):**

```
liquidate_user(asset)
  revert pending oids on asset
  fold that asset’s unsettled into cash
  lots = position
  reserved IM → 0
  Book -= lots
```

**Proposed:**

```
liquidate_user(asset)                    # adapter, ER, fee 0
  revert user-originated pending oids on asset (state → liquidating / fail)
  fold unsettled on that asset into cash (keep current behavior)
  if lots == 0: return
  # Q1 called: adapter-trusted. Loop must only invoke when the helper trips.
  park pending_liq oid { lots_delta = −lots, state = liquidating }
  user lots tentatively 0, release IM
  # Book unchanged

hedge: Phoenix IOC with lots = pending_liq.lots_delta for that oid  # L1; v0 one IOC per oid

ack_phoenix_fill(liq_oid, filled, vwap, fee)
  Book += filled   # filled is reducing, so Book moves toward 0 on that user size
  user lots already 0 if fully filled; partial: restore leftover lots + IM

ack_phoenix_fail(liq_oid)
  restore user lots + IM  # same as place fail
```

Reuse `OpenOid` (cap 8). Liq must **win** a slot: revert the user’s own pending first (already done). Adapter-originated liq oid should not count against the user’s 8 in a way that blocks safety — if the cap is full, reverting pending already frees slots. Spec that explicitly.

Partial IOC: same as S6. I1 after ack is Book == Phoenix. Live I1 is Book + pending (including liq).

**Do not** set `INVARIANT_BROKEN` because a liq is in flight. In-flight table already exists for place hedges; extend it with a `liq` kind.

### 6.4 Scan loop (adapter)

New module, e.g. `adapter/src/liquidation.rs`, next to `funding.rs`. Not a method hidden in `hedge_pending`.

```
loop {
  now = wall clock (ms). Do not use ER slot * 10.
  mark, trader_state = ingest()
  if mark_age > MARK_STALE_MS:
      halt |= HALT_ENTRIES
  if mark_age > MARK_DEAD_MS or scan_age > SCAN_DEAD_MS:
      halt |= OPERATOR_DOWN | HALT_ENTRIES
  if invariant_broken: sleep SCAN_INTERVAL; continue   # P-L11

  if mark fresh:                         # age ≤ MARK_STALE_MS
      queue = rank(users with lots != 0) # QFS / memory; helper = 08 equity + haircut
      enqueue equity < Cinder_MM
  # soft/hard stale: do NOT newly classify off this tick; drain queue from last fresh print

  drain queue worst-first via liquidate_user (ER)     # even if mark is now stale
  if unsafe_pool and Rise still not Safe:
      flatten equity < Cinder_IM worst-first until Safe or dry
  send net Phoenix IOC if pending_liq != 0
  ack on ER
  maybe write_reserve_root

  if now - last_heartbeat_ms >= HEARTBEAT_MS:
      write Book.last_scan_ms            # ER; default 1 s, not every rank
  sleep until max(SCAN_INTERVAL, next_mark)
}
```

`trader_state` older than `TRADER_STATE_STALE_MS` → `HALT_ENTRIES` (already freeze). Same drain-queued, do-not-reclassify rule as mark.

### 6.5 Watchdog on ER (optional, maximizes 10 ms without trusting only the process)

MagicBlock crank **or** a permissionless `trip_stale_scan` ix:

- Reads `Book.last_scan_ms` (or `mark_ts` if we later cache it).
- If `Clock` (unix ms, not slots) − last_scan > `SCAN_DEAD_MS`, sets `OPERATOR_DOWN | HALT_ENTRIES`.
- Signer: **adapter-only in L3** (Q4). Permissionless later if adapter death is still too silent.

A scheduled crank that **only** calls `trip_stale_scan` is legitimate ER work: all inputs are on ER. It does **not** liquidate (no mark). Optional L6; L3 heartbeat is enough for v0.

Do **not** schedule `liquidate_user` as a crank. The crank signer is not the adapter; freeze says adapter-only; Q12 says no session keys. Do not smuggle a second authority.

### 6.6 Health details (so the loop is not a toy)

- **uPnL:** `lots` and `entry_quote_lots` already on `Position`. Convert mark to the same quote-lot units as ack VWAP. Document the conversion next to S6 lot math. Toward-zero integer, same as funding.
- **Phoenix MM:** `createMarginCalculator` on **user size**, then × `USER_MM_MULT_BPS / 10_000`. Not residual. Not stub. Cache per mark tick, not per user RPC if the calculator is local.
- **Unsettled funding:** already in 08 equity. Scan must include it or we fight the funding crank.
- **Pending place oids:** tentative lots are already on the user. Equity for MM should use **tentative** size (user is already exposed in Cinder’s promise) **or** revert them first (current liq reverts pending). Keep “revert pending then evaluate remaining lots.” A user who is only pending and never acked is not Phoenix residual yet; fail-ack is enough; do not Phoenix-hedge a liq for size that was never acked.
- **Positive uPnL:** Phoenix discounts profitable uPnL for **pool** health (`uPnL risk factor`). If Cinder counts Alice’s paper profit in full, she can look solvent on a number Phoenix would not give the pool (unnetted, she *is* the residual). v0: **losses in full; haircut gains** with Rise’s factor if the calculator exposes it, else `UPNL_GAIN_HAIRCUT_BPS = 5000`. A strongly profitable, well-capitalized user still clears MM after a 50% haircut. The people who trip are those whose solvency *is* undiscounted paper profit. Do not let withdraw ride on uPnL (already flat-only).
- **One helper:** `crank_funding` P-F7 and the scanner call the same `user_equity` / `cinder_mm` functions. No stub IM on the fork path.

### 6.7 Halt machine (additions, not rewrites)

Keep existing bits. Add behavior:

| Mark age | Entries | Flatten |
|---|---|---|
| Fresh (≤2 s) | normal | Classify and flatten `equity < Cinder MM` (P-L4). |
| Soft stale (>2 s, <10 s) | `HALT_ENTRIES` | Drain users **already queued** on the last fresh print. Do not newly classify off the stale tick. |
| Hard stale (>10 s) or scan miss (>2 s) | `OPERATOR_DOWN \| HALT_ENTRIES` | Still try the already-queued set + ingest. Do not sit on leverage because the feed died. |

| Event | Flags | Liq loop |
|---|---|---|
| Fresh mark, user < Cinder MM | none | Classify + flatten (P-L4) |
| Phoenix `Cancellable+` | `UNSAFE_POOL` | MM line first, then IM only until Rise `Safe` (P-L5) |
| Venue liq of Cinder | freeze path (already freeze) | Do not pretend D3 still holds. Halt flag only in this slice; socialize path is the failure of D3, not v0. |
| `INVARIANT_BROKEN` | bit 4 | Skip automatic liq; do not hide with silent repair |

Mirror halt to L1 `Config.paused` as today.

### 6.8 Privacy and logs

- Operator QFS token reads all UserLedgers (already).
- ER `liquidate_user` stays on PER: observers do not see which user was flattened.
- Phoenix IOC shows **net** residual change only (Q1-A).
- Adapter logs: user pubkey + reason are **operator-private**. Do not print them to a public metrics sink in v0.
- Do not add a public L1 “liquidation event” account.

### 6.9 Tests (this slice is not done without these)

Adapter unit (mock Phoenix + memory ledger), then S1-style local, then fork smoke.

1. **Quiet book:** one user, no place, no funding tick, mark moves through Cinder MM → `liquidate_user` fires, then reducing IOC, I1 holds after ack.
2. **uPnL required:** cash+unsettled still ≥ stub IM, but **negative** uPnL makes haircut-equity < Cinder MM → still liquidates. Well-capitalized + **positive** uPnL (even after 50% haircut) does **not**. Solvency that exists only if paper gains are counted at 100% **does** flatten.
3. **Netted insolvency:** A underwater, B healthy, pool `Safe` → only A flattened; Phoenix IOC equals A’s lots (residual change), not B.
4. **I1 in flight:** after tentative flatten and before ack, `Book + pending_liq == Phoenix`. Fail-ack restores A. Fill-ack moves Book.
5. **Funding hook still works:** P-F7 path liquidates after accrue when unsettled (not mark) breaches MM.
6. **Stale mark:** >2 s → `HALT_ENTRIES`; users queued on the last fresh print still flatten; a user who only goes through MM *after* the feed is stale is **not** newly classified. Heartbeat still writes `last_scan_ms` on `HEARTBEAT_MS`.
7. **Dead scan:** skip the loop > `SCAN_DEAD_MS` → `OPERATOR_DOWN`.
8. **UNSAFE_POOL:** two users below IM, only one below MM; flatten the MM-breach first; IM-breach only if mock pool still not `Safe`; stop when `Safe`.
9. **User cannot sign** `liquidate_user` (already S3; keep).
10. **No undelegate** in the path (assert accounts still delegated in PER test).
11. Fork: `CINDER_LIQ=1` analog of funding smoke — move mark on the fork (or inject calculator), quiet book, Cinder flattens while mock/fork pool is still not `Liquidatable`.

### 6.10 Suggested build order (after this doc is agreed)

Not a tracker row until you say so. Suggested cut:

| Cut | Deliverable | Success |
|---|---|---|
| L0 | Spec freeze patches (I1-safe liq, constants, `last_scan_ms`) | `03` amended; this file → agreed |
| L1 | Health + ranking in adapter (memory) | Tests 2–3 pass without chain |
| L2 | I1-safe `liquidate_user` + pending liq oid + ack | Tests 1, 4 on local validator |
| L3 | Scan loop + heartbeat + halt bits | Tests 6–8 |
| L4 | Wire Rise MM calculator (drop stub on fork) | P-F8 + P-L1 same helper |
| L5 | Fork smoke `CINDER_LIQ=1` | Test 11 |
| L6 | Optional: batched ix, ER `trip_stale_scan` crank | Only if L3 ops say adapter death is still too silent |

L2 (I1-safe ix) **before** L3 (loop). Do not start L2 by “just calling the existing ix faster.” That encodes the I1 hole. Not S10. Follow-on after the funding overlay (S0–S9 already closed).

### 6.11 What we are not doing in this slice

- Permissionless / third-party liquidators
- Session keys so a crank can `liquidate_user`
- On-chain mark oracle PDA
- Window > 0 netting as a liq substitute
- Changing 1.25× or the 10× cap
- Socialize / venue-avg flatten except as a halt flag. That path is the failure of D3, not v0. Implement only after L2/L3 exist.
- Treating Phoenix `Safe` as “no user is underwater.”
- Multi-asset portfolio MM
- Crank that calls `liquidate_user`

---

## 7. Proposed freeze knobs (not applied)

```
SCAN_INTERVAL_MS         = 50        # in-process min interval; on each fresh print
HEARTBEAT_MS             = 1000      # Book.last_scan_ms write; not a fee rule
SCAN_DEAD_MS             = 2000      # missed in-process scan → OPERATOR_DOWN
MARK_DEAD_MS             = 10000     # hard stale → OPERATOR_DOWN
UPNL_GAIN_HAIRCUT_BPS    = 5000      # if Rise factor missing
# MARK_STALE_MS          = 2000      # halt entries; still drain queued flattens
# TRADER_STATE_STALE     = 2000
```

`Book` additive field:

```
last_scan_ms: u64    # adapter heartbeat; 0 = never
```

Do **not** add `mark_px` in the first freeze patch. 08 already forbade a dedicated mark PDA. On-chain MM (Q1) is adapter-trusted.

`OpenOid` state `OID_LIQUIDATING = 3` already exists. Use it for adapter pending liq, not only as a leftover after flatten.

---

## 8. Open questions — called

| Q | Call |
|---|---|
| Q1 On-chain MM | **Adapter-trusted.** Honest-operator local win. Tests that the loop only fires when the helper trips. |
| Q2 Heartbeat | **`Book.last_scan_ms`.** In-process scan 50 ms. ER write default `HEARTBEAT_MS = 1000`. Optional `trip_stale_scan` later. No mark PDA. Faster writes while fee = 0 are allowed, not required. |
| Q3 P-L4 | **Yes.** Immediate `Book -=` rejected. Live I1 = `Book + pending_place + pending_liq`. |
| Q4 Permissionless trip | **Adapter-only** in L3. |
| Q5 IM vs MM under unsafe | **MM first, then IM only until pool `Safe`.** |
| Q6 50 vs print | **On each fresh print, min interval 50 ms.** |
| Q7 Batch ix | **Serial.** Measure before batching. |

---

## 9. Why this is the “correct manner” (short)

D3 is a **race against Phoenix’s own MM engine** on the same mark, plus a **private** race against users Phoenix cannot see because of netting. The overlay (1.25×) is a few tens of basis points of notional of time. Funding hours and coincidental hedges are not a race we can win.

MagicBlock gives us the only cheap, private, fast write path we have: delegated ER txs at 10 ms and $0. The correct system is: **compute health whenever a mark exists, flatten privately in burst, hedge net publicly once, stay delegated, commit rarely, halt loudly if we stop looking.** Anything that folds that into `crank_funding` or `hedge_pending` will fail the first quiet crash.

---

## 10. Status of this document

Agreed and implemented on `liq-liveness` (stacked on funding overlay). Freeze patched. Load-bearing locks: **P-L4** and **P-L7**. Fork mark-crash smoke (`CINDER_LIQ`) is still later.
