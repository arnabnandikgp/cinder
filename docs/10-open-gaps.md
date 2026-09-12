# Open gaps — backlog (not freeze)

Status: **note only**. Do not implement from this file. When a row is ready for work, write a planning doc (like `08` / `09`), agree it, patch freeze, then a tracker slice. **Not S10.**

Cinder is a **private perp prime broker** on Phoenix, not a venue. Matching, mark, funding *rate*, and liquidation of the **pool PDA** stay on Phoenix. This list is “what the overlay still lacks so users have a real perp and the pool does not eat Alice.”

Taken 2026-09-13 after S0–S9, funding overlay (`08`), and liquidation liveness (`09` / PR #11).

---

## 0. How to use this list

| Tag | Meaning |
|---|---|
| **Load-bearing** | Protocol is not a perp (or D3 is fake) until this exists. Plan next. |
| **Runtime** | Function exists; nothing calls it on a clock. Same class as the liq/funding holes. |
| **Numeric** | Logic exists on stub units; live Phoenix numbers are not wired. |
| **Specified** | Freeze or `04` already says it; code does not. |
| **Product** | First-win thinness. Not solvency. |
| **Deferred** | Standing order: do not build until a stage is Open. |
| **Not ours** | Phoenix (or a later product) owns it. Do not invent a Cinder copy. |

IDs are stable (`G-*`). Close a row only by pointing at an agreed spec + merged code.

---

## 1. Load-bearing (plan these)

| ID | Gap | Why it matters | Today |
|---|---|---|---|
| **G-PNL** | **Trading PnL never hits user cash.** `ack_phoenix_fill` updates lots, stub IM, `entry_quote_lots`, fees. Close/liq does **not** `realized = closed × (exit − entry)` into `free` then `reserved`. | User who wins and flattens gets IM back, not profit. User who loses and is liquidated can have the hole land on Phoenix collateral or vanish off the private book (silent socialize). Funding *does* move health; trade PnL does not. Same class as the first funding-draft I2 bug. | Not specified as an ix overlay. `entry_quote_lots` is the basis when we plan it. |
| **G-RUN** | **No adapter process.** `hedge_pending`, `scan_liquidations`, `crank_funding`, `expire_inflight` are library methods. | 08/09 are unit-test true and production-false without a loop that ingests mark + trader-state, hedges, scans, cranks funding, heartbeats, expires oids, tops buffer, writes `ReserveRoot`. Liq polling and funding payouts were two clocks; this is the missing clock-driver. | `cinder-adapter` is a lib crate. No `main`, no WS supervisor. |
| **G-MM** | **Cinder IM/MM still stub notional** (`1 USDC/lot × 1.25 / 10`). Scanner and `crank_funding` call `stub_cinder_mm`. | D3’s 1.25× line is not Phoenix’s line on a live SOL perp. P-F8 / P-L1 already say stub is illegal on a fork. | Rise `createMarginCalculator` named in `04` / `08`; not the helper. |

**G-PNL** is the next spec slice if we keep going in protocol-correctness order. **G-RUN** can share a process with that slice or follow it. **G-MM** can land with G-PNL (same mark/calculator).

---

## 2. Half-done clocks (functions exist)

These were the two you noticed. Spec + code for the *handler*; liveness is G-RUN.

| ID | Gap | Notes |
|---|---|---|
| **G-FUND** | Funding crank is not scheduled. Phoenix interval is hourly; collateral fold when venue collateral actually moves. | `docs/08`, adapter `crank_funding`. No hourly (or “interval elapsed”) caller. Live 24h settle smoke still not done. |
| **G-LIQ** | Liq scan is not scheduled. | `docs/09`, `scan_liquidations`, I1-safe ix. No mark WS + 50 ms loop. Fork `CINDER_LIQ` smoke not done. |
| **G-OID** | In-flight / oid TTL expire is not scheduled. | `expire_inflight` exists. Same process as G-RUN. |
| **G-TS** | Trader-state staleness is a **hedge gate**, not a loop. | `TRADER_STATE_STALE_MS` → `HALT_ENTRIES` on `hedge_pending`. Quiet book + dead Rise WS does not trip it unless the scan/runtime watches trader-state too (`09` already says keep ingesting). |

---

## 3. Specified, not built (ops / failure)

| ID | Gap | Where it is written | Notes |
|---|---|---|---|
| **G-BUF** | 20% pool buffer (floor 50 USDC) top-up before a hedge that would breach. | D3, `04` “post if needed” | `hedge_pending` does not `post_collateral`. Buffer constants live on `Config`. |
| **G-SLIP** | `max_slippage_bps` on `place_order` is only range-checked. Never compared to fill VWAP on ack. | Freeze `place_order` args | No-op protection. |
| **G-FEE** | Phoenix taker 3.5 bp “assigned to that user on ack.” | `04` | `ack` *can* debit `fee_usdc`; adapter hedge path must pass the real fee. Confirm on fork, do not assume. `CINDER_FEE_BPS = 0` is policy. |
| **G-SOC** | Venue already liquidated Cinder → freeze, flatten users at venue avg, socialize shortfall from those users’ free then reserved. | Freeze halt machine | **Failure of D3**, not v0. Halt flag only. Do not build before G-PNL + G-LIQ runtime exist. |
| **G-DEBT** | After a *successful* Cinder MM liq, if equity is still negative (needs G-PNL). No insurance fund. | Implied by I2 | Unspecified leftover: take Alice to 0 then halt / named haircut. Silent I2 break is the wrong default. |
| **G-COMMIT** | Delegated fee payer + `magic_fee_vault` before the 10th ER commit. | Freeze Commit | Easy to forget until PER sessions are long-lived. Commit `ReserveRoot` rarely; that is the MagicBlock bill. |
| **G-I1LIVE** | I1 live = `Book + pending_place + pending_liq == Phoenix` after venue fill, before ack. | Freeze invariants (09 patch) | Adapter should assert this on the liq hedge path; memory tests cover a slice. |

---

## 4. Product (first win is thin on purpose)

Not solvency. Do not confuse with G-PNL.

| ID | Gap | Notes |
|---|---|---|
| **G-LIM** | No resting limits, cancel, or amend. Market/IOC only. | Phoenix 64+64 unused. Maker 0.5 bp unused. |
| **G-TRIG** | No TP/SL / trigger / TWAP. | `02`: browser TWAP out of scope. |
| **G-MKT** | One allowlisted asset. | Layout: 16 user positions, 32 book markets. |
| **G-API** | Surface is API-first (`01` Q4-A). No HTTP/WS user API. | Programs + adapter lib only. |
| **G-READ** | No user-facing mark equity / liq price / funding owed view. | QFS can read the ledger; health is adapter-side. |
| **G-ESC** | `escape_withdraw` returns `unsupported`. `ReserveRoot` writes. | Named gap, not silent CEX (`01` Q7-A). Working escape is **Deferred**. |
| **G-DEP** | Deposit is not atomic L1 USDC + ER `credit_deposit`. | Known. Adapter credits after L1. |

---

## 5. Deferred (standing orders — do not start)

Do not implement until the stage is Open and previous stages Closed (`AGENTS.md`).

| ID | Item | Notes |
|---|---|---|
| **G-WIN** | Windowed residual (window > 0) | Explicitly not a tracker stage. Layout can absorb it. |
| **G-CFEE** | Cinder fees (`CINDER_FEE_BPS` > 0) | Field exists; keep 0. |
| **G-SESS** | Session keys | Q12. Do not smuggle as crank authority for `liquidate_user`. |
| **G-INV** | Invite graph | D8 / Q5. |
| **G-ESPL** | Stealth eSPL | Out. |
| **G-ESC2** | Working `escape_withdraw` (merkle/claim from `ReserveRoot`) | After root is writing and trusted. |
| **G-ISO** | Isolated Phoenix children / `isolatedOnly` markets | Kills Q1-A. Skip. |
| **G-TEE** | Persist user TEE/QFS tokens on the server | Never. Adapter holds one operator token. |
| **G-MAGIC** | Phoenix order as a Magic Action | Never. Commit would revert if Phoenix rejects. |

---

## 6. Not Cinder’s job

Do not open a planning doc that “adds” these on ER.

| Item | Owner |
|---|---|
| CLOB matching, index, mark construction | Phoenix |
| Funding **rate** / accumulator | Phoenix (we attribute) |
| IM/MM/liquidation of trader PDA `(0,0)` | Phoenix (we must beat this on the **user** book) |
| Insurance fund, backstop, ADL | Phoenix (hits the **pool** if G-LIQ/G-PNL fail) |
| Isolated vs extra `pda_index` | Unavailable / privacy-destroying |
| Permissionless liquidators | Cannot see private books (honest operator = HyperLink enclave) |

---

## 7. Suggested planning order (when we come back)

Not a tracker. A reminder of dependency:

1. **G-PNL** — realize close/liq into cash; I2-safe; leftover = G-DEBT policy. Same ack discipline as place/liq.
2. **G-MM** — Rise calculator is the IM/MM helper (drop stub on fork). Shares mark ingest with 1.
3. **G-RUN** — one operator process: mark + trader-state WS, hedge, scan, funding, oid TTL, buffer (G-BUF), heartbeat, reserve root. Makes G-FUND / G-LIQ / G-OID / G-TS live.
4. **G-SLIP** / **G-FEE** — small, can ride on 1 or 3.
5. **G-SOC** — only after 1+3, and treat as disaster recovery, not happy path.
6. Product (G-LIM, G-MKT, G-API, G-ESC2) — after the overlay is a perp.

---

## 8. What we are *not* missing (do not reopen)

- One Phoenix cross PDA `(0,0)`. Residual hedge, not a dark pool.
- Window = 0 first win.
- PER + QFS attribution privacy.
- I1 (Book vs Phoenix after ack) and I2 (cash, with unsettled terms).
- User IM overlay + 10× cap **as policy** (numeric wiring is G-MM).
- Funding *attribution* design (`08`) and liq *liveness* design (`09`) — handlers exist.
- `ReserveRoot` as the public claim snapshot.

If spec and this list disagree later, the agreed freeze wins; amend this file.
