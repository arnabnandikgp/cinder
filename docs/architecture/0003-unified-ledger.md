# ADR 0003: one position and location ledger

2026-09-30. P03 implementation decision within the approved
[baseline](../implementation/BASELINE.md); no new economic policy.
[System architecture](../architecture.md) describes the later integrated runtime.

## Boundary and representation

Implement `position` and `ledger` modules in the existing dependency-free kernel.
One `Ledger` owns one configured quote pool/native account, multiple linear markets,
private customer books, house, suspense, physical vault cash, native signed cash,
full-transfer receivables, execution bindings and accepted normalized provenance.
No reported-total-equity setter or generic balance adjustment exists. All fields
behind projections are private; no plugin receives mutable balance-sheet access.

This is the initial account topology already approved, not a choice to create one
native account per user or silently mutualize future cohorts. Additional pools,
assets and cross-venue rights require their owning phase's explicit extension.
No third-party library, separate journal database or service is introduced here.

`Market` binds a rational quote-atom-per-lot-tick conversion to a `MarketUnit`.
Products and conversions are checked; inexact conversion rejects. Qualified native
precision and fee rounding remain P04/P13, not arbitrary defaults inferred here.
Signed basis splits toward zero, retaining the entire remainder in the open basis.
A full close consumes all basis; opening notional is not debited from cash.

## Transitions and attribution

| Change | Effect and identity | Rejection / boundary |
| --- | --- | --- |
| BindExecution | Attempt key fixes private owner, market and side before fills | No reservation/admission/signing permission; caller must authenticate intent; P07 supplies full lifecycle |
| Receipt | Qualified economic receipt adds one physical/native asset amount and one owner cash credit | Exact configured source location/quote; nonpositive amounts or unknown owner reject; unresolved ownership uses suspense |
| Fill | Economic execution updates both native net book and the bound private/house/suspense book | Customer cannot be supplied independently of the recorded attempt; wrong route, unit, side or source rejects |
| TransferDebit | Source decreases once, one named receivable increases | Same-location/invalid amount rejects; physical tokens cannot become negative |
| TransferArrival | Exact debit reference consumes its receivable into destination | Unknown/already-arrived/wrong-source arrival rejects; no second customer credit |

Bindings and inputs are **not cryptographic authorization**. A malicious adapter
could name a different already-valid attempt; P13 must qualify native causal
evidence before constructing the event. The P03 wrong-owner guarantee is narrower:
the supplied record cannot override a stored route's private owner, and replay
cannot change allocation under the same economic key. Likewise receipt attribution
and asset authenticity are trusted normalized input obligations, not proven by
equal totals. Production calls must not expose these internal constructors as a
public API.

The key includes deployment, source account/semantic namespace, native economic
ID and participant leg. Same key/equal semantic event is an unchanged-state no-op;
different payload is a conflict. Every successful distinct proposal appends its
accepted provenance and increments the version once. Rejected raw evidence must
be retained by P05/P13 ingestion, not silently discarded by a caller. Qualified
unattributed facts can post to suspense; the unresolved count does not disappear
just because offsetting events produce zero equity.

Apply clones the state, computes every effect, checks bridges, and only returns a
successful proposal after all checks. Arithmetic/source failure leaves the caller's
input intact. This is not durable atomic consumption, compare-and-swap, or replay
after process loss. The in-memory vectors/linear search and full clone are an
explicit refinement baseline, not a scalable production journal. P05 supplies the
bounded persisted representation and atomic commit; no live service uses this now.

## Hand derivation of bridge preservation

For each market require `q_V = Σ q_owner`, with owners including customers, house
and suspense. Also require this price-independent intercept equality:

```text
vault + transit + c_V - Σ_m b_Vm = Σ_owner(c_owner - Σ_m b_owner,m)
```

Combined, these imply external and private total marked equity agree at every
common exactly representable mark. They do not imply collateral sufficiency or
correct authenticated allocation. Proof obligations by transition:

- Initial state: all quantities and intercepts are zero.
- Receipt: external cash and the single owner's cash both increase by `d`.
- Transfer debit/arrival: equal positive and negative location/receivable changes;
  no private entitlement changes.
- Binding: no financial change.
- Fill: for opening, `Δc=0`, `Δb=xv`, so `Δ(c-b)=-xv`. For a reduction/reversal,
  `Δc=t z v-beta`, `Δb=-beta+(x+t z)v`; subtraction again gives `-xv`.
  Both native and the one attributed owner receive the same `x,v`, even if their
  old quantities/bases differ. Exposure rises by `x` on each side.
- Equal replay: identity transition. Conflict/rejection: input state unchanged.

All steps assume representable exact notionals; checked errors do not mutate the
ledger. Conservative rejection on an intermediate sum overflow is permitted,
never wrap/clamp. A source fact that cannot be represented needs retained raw
evidence and containment, not fabrication of a rounded financial event.

Native basis and cash are deliberately allowed to differ from summed private
basis and cash. Alice buying at 100 and Bob selling at 120 leave native exposure
flat with 20 realized cash, while their private open positions retain different
entry bases. A model that simply moves native cash alongside private realized
PnL would break this identity or double-count value.

Positive-claim coverage is a separate diagnostic:
`N - Σ max(e_i,0) - max(s,0) = h - Σ max(-e_i,0) + min(s,0)`.
No negative customer or suspense balance becomes a collectible asset. Unresolved
suspense prevents treating a nonnegative result as qualified assurance. The kernel
does not refuse a real adverse fill merely because the diagnostic is negative.

## Verification and limits

Tracked `primitives.rs` V02 vectors now call production `Position::fill`, replacing
the P02 test-only transition. `ledger.rs` tests V01 full transfer/return, V03 net-flat
default, winner/pool-loser and loser/pool-winner closes, unequal native/private
basis, house/suspense ownership, duplicate/conflict replay, wrong attribution,
domain/source/version errors, complete multi-market valuation and integer bounds.

Deterministic properties cover 25,020 position/fill combinations at two marks and
12 seeded histories of 128 fills, each checked at three marks against an independent
cash-flow/exposure oracle. Histories replay from empty state with exact equality.
These are finite tests plus the hand derivation above, **not machine verification**.
The DeFi skill's referenced QEDGen guide is unavailable in this installation; no
fabricated `.qedspec`, generated proof, independent audit or live qualification is
claimed. Machine proof/implementation-refinement obligations remain explicit for
the later assurance work; tests do not discharge them.

P04 owns funding/fee accruals, qualified marks/source cuts and mismatch resolution.
P05 owns durable raw/normalized journal/atomicity; P07 authorization/reservations;
P08 owns partial, out-of-order, fee-bearing and impaired transfers and payouts.
Full in-order fee-free transfer tests here do not replace those lifecycle cases.
No token exchangeability, native margin model, withdrawability, risk limit, source
freshness or actual Pacifica execution is inferred from this module.

## Local provenance promoted without runtime dependencies

Requirements come from tracked BASELINE/CONFORMANCE and research families W01–W02.
The P03 review consulted these ignored first-party study sources (SHA-256):

| Source under local `work/` | SHA-256 |
| --- | --- |
| `specs/financial-model/IMPLEMENTATION_HANDOFF.md` | `5a36ccf84c6cf8c2a15f6e5de5ad716d74febc36e3f23074e0d0e18c99542b93` |
| `specs/financial-model/model.cjs` | `853d4955f0a3b0c13125558105b37566bce374e764e69782d215938aee0ef74f` |
| `specs/financial-model/pool-protection.cjs` | `0a8238a273985e5eece602a45fa642d2820ff703750bad329d2d733de3579f92` |

Only sanitized equations/vectors are promoted; no study runtime or third-party
contract source is copied. Fresh-checkout tests need none of these ignored files.
The full architecture additionally condenses already-approved W04–W08 contracts;
their provenance and historical limitations remain in the evidence register.
