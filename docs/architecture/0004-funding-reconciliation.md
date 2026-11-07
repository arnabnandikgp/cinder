# ADR 0004: funding, execution costs and evidence containment

2026-09-30. P04 extends the existing ledger under the approved
[baseline](../implementation/BASELINE.md), not a new financial contract or native
venue qualification. [ADR 0003](0003-unified-ledger.md) remains the position/location
foundation; [TRACKER](../implementation/TRACKER.md) records verification and PR state.

## Scope and implementation boundary

`ledger::economics` implements explicit funding cuts, recognition and settlement,
native fee/rebate attribution, fee-inclusive PnL normalization, and separate broker
charges. `ledger::evidence` retains normalized observations, named discrepancies,
their age, source snapshot checks and qualified-mark gates in the **same Ledger**.
The dependency-free `NormalizeObservation` port separates source conversion from
posting; its fixture is synthetic, not a Pacifica adapter.

No database, service, signer, new account topology, live endpoint or third-party
dependency is added. Financial policy numbers remain G02. Source authentication,
native precision and causal completeness remain G01/P13; declaring an input
qualified in these internal structs does not establish those facts.

The original fee-free `Change::Fill` is retained as the algebra seam used by P03
conformance tests. Native adapters must supply `Execution` with the qualified fee
and PnL convention; an unavailable fee cannot be represented by inventing zero.
No public API may expose these trusted internal constructors directly.

## Funding cuts and exact allocations

A funding boundary binds an economic source ID, configured market, exact local
projection version, and the private/house/suspense/native quantities at that cut.
It cannot be recaptured from positions when a delayed rate message arrives. A
stale version rejects. **Version equality is not native causal completeness**:
the adapter/controller must first qualify which executions belong at the cut.
An interval-based or otherwise nonlinear venue rule needs a separately qualified
extension; it must not be forced into this per-lot boundary formula.

Let owners `o` include each customer, house and suspense. For a qualified signed
payout per long lot `r=n/d`, denominator `d>0`, and explicit rounding rule `R`:

```text
f_o      = R(q_o * n / d)
expected = R(q_V * n / d)
rho      = expected - sum_o f_o
delta    = actual_native - expected

posted_customer_i = f_i
posted_house      = f_house + rho
posted_suspense   = f_suspense + delta
sum_o posted_o    = actual_native
```

Positive `n` pays longs; conventional venue rate signs/mark factors must first be
normalized by the adapter. Floor/ceil handle signed shorts correctly; exact mode
rejects nonintegral results. Only the **derived** net-versus-owner rounding residue
`rho` belongs to house. The unexplained actual discrepancy `delta` remains in
suspense with a `FundingDifference` condition, even if small. This does not
approve a tolerance for any observed native rounding gap.

Funding inputs are optional independently, but unknown is never zero:

| Evidence | Postings / containment |
| --- | --- |
| Boundary only | Freeze inventory, open `FundingInputs`; no invented accrual |
| Rate known, native amount unknown | Retain rate, keep condition open; no purported fully reconciled allocation |
| Native known, rate unknown | Book native amount and equal suspense accrual; private allocation remains blocked |
| Both known | Allocate against frozen inventory using the equations above; close missing-input condition, retain any actual mismatch |
| Settlement before rate/recognition | Book native cash and suspense cash; later complete inputs reattribute that same cash, not another earning |

Qualified native zero still needs the independent rate for a net-flat pool.
V04 has Alice long 10 and Bob short 10, with `n=-1,d=1`: accruals are -10/+10,
native zero. Net zero cannot establish that both users owe zero.

Known rate/native values cannot silently be overwritten by another `FundingInputs`
event. `CorrectFundingNative` has a distinct source identity and preserves the
original rate/inventory. Correcting the private rate itself is not supported by a
generic override; keep the affected evidence restricted until an explicit
qualified correction contract is implemented. This is not an operator reset API.

## Recognition, settlement and bridge preservation

Each book has settled `cash` and recognized unsettled `funding` separately. A
boundary retains only its own posted amounts; settlement cannot sweep another
boundary's accrual. With `x` that boundary's allocation:

```text
recognition: a += x
settlement:  a -= x; c += x
equity:      e = c + a + sum(q*p - b)
```

Equal repeated event/identical payload does not post again. A second settlement
under a new identity rejects. An actual settlement amount that differs from the
recognized native amount changes native/suspense cash by the discrepancy, not
the customer's qualified funding rule. The retained issue ages until a named
source correction accounts for it. Corrections apply only their delta, using cash
after settlement and accrual before it. No delayed recognition creates two earnings.

The existing price-independent bridge becomes:

```text
vault + transit + c_V + a_V - sum_m b_Vm
  = sum_o(c_o + a_o - sum_m b_om)
q_Vm = sum_o q_om
```

Recognition adds equal native and total-owner amounts by construction; settlement
preserves each `c+a`; a correction adds its delta to both native and suspense.
Funding never changes position quantities or basis. These facts, with ADR 0003's
fill identity, derive bridge preservation at common representable marks. They do
not establish recoverability of debt, authentic venue assets, or future solvency.

## Fees, PnL and ownership

An execution's signed native fee `f` is positive for a charge, negative for a
rebate. Its stored attempt selects the private owner; the same fill updates
native and private basis independently. Native realization is compared with the
**native** cost basis, not the private owner's potentially different realization.

```text
reported_gross = reported                     if convention is Gross
reported_gross = reported + f                 if convention is NetOfFee
gap            = reported_gross - modeled_native_realization

c_owner -= f
c_V     -= f; c_V += gap
c_S     += gap
```

The position transition already posted the modeled realizations. These entries
therefore deduct the actual fee once. A nonzero gap creates `NativePnl`, never
house profit. `CorrectExecutionPnl` changes native/suspense cash by revised gross
minus the original reported gross; it does not execute the fill or fee again.
Fee corrections cannot be smuggled through this PnL-only correction.

An optional absent PnL report means that the qualified source does not supply it;
derive realization from basis. A corrupt or required-but-missing native report
must fail source normalization, not be relabeled as optional absence. Native
snapshot reconciliation remains necessary either way.

`BrokerFee` is a distinct, positive, explicitly authorized customer-to-house
charge with a matching customer request key, and no external asset change. This
implements ledger semantics, not a fee schedule or permission to levy charges.
P07/P18 authenticate and authorize it; G02 must approve terms. Pass-through native
fees plus a separately authorized charge do not prove a best-fee promise.

## Evidence identity, retention and replay

`apply` remains the pure economic algebra seam: equal accepted replay is an
unchanged-state no-op; rejection leaves the supplied ledger unchanged. Services
must use the `ingest` proposal path, which also records normalized rejected and
duplicate inputs, injected monotone arrival time, disposition and named issues.
Financial fields never partially commit after a failed fill/fee/funding leg.

Arrival metadata is outside semantic event equality. Equivalent REST/WS reports
must normalize to the same canonical event and convention; the fixture proves
that an already-net report and a gross report can do so without double fees.
Preserve the original raw representations separately in P05/P13. A local cut's
expected version is fixed once in its normalized event, not regenerated on replay.

Every `ingest` observation increments the proposal version once, including a
duplicate or contained rejection. It does not increment cash twice. Accepted
events and retained observations are different read-only projections of the one
state. Replaying observations from the same initial config with their recorded
times reproduces the exact state/dispositions, including unresolved evidence.
Production must not interleave unjournaled `apply` calls with journaled ingestion.

A previously unready event can retry unchanged after its dependency arrives;
success resolves its transient rejection. A changed payload under that identity
is a retained replay conflict, including when the original was rejected. Repeating
the original accepted payload still has no economic effect and does not erase
the conflict. Cross-deployment garbage or backwards time is rejected at the
envelope boundary; the eventual outer raw archive must retain such input too.

## Snapshot reconciliation and dependent views

`NativeCheck` supplies source-qualified cash, unsettled funding and position
components, source completeness and the exact projection version. None is
unknown; flat markets must be explicit for a complete match. A check cannot write
balances. Missing/mismatching components create a named `NativeSnapshot` issue;
an unexplained cash discrepancy is not an authorization to balance the books.

A later complete matching check can resolve a specifically named older check:

- Retain the earlier projected components and accepted native before/after effects.
- Named effects must be unique, genuinely change the native book and follow that
  earlier check in the accepted history.
- Their summed deltas must explain **every originally known component** exactly,
  including quantities and basis, not just total equity. Unknown components can be
  completed without postings. An incomplete flag cannot erase a known mismatch.
- A later snapshot or unrelated transaction cannot blanket-clear discrepancies.
  Existing PnL/funding/conflict issues require their own resolution paths.

This proves an arithmetic explanation, not the native causal provenance of the
supplied effects. P13 must authenticate their original cut/history relationship.
Before/after audit projections are not additional assets. Unrepresentable
explanation sums fail closed; retaining an actual representable native state does
not require an overflowing signed delta just to ingest it.

`EvidencePolicy` injects positive maximum ages for marks, matching reconciliations
and unresolved issues in a common millisecond domain. Test values are synthetic,
not recommended production thresholds. Issues retain first-seen age across repeat
observations. New valid facts do not erase old discrepancies.

- `Reconciled`: fresh matching check, no open issue or unresolved attribution.
- `Restricted`: missing/stale reconciliation or unresolved discrepancy.
- `Frozen`: replay conflict, or unresolved issue reaching its maximum age.

These are evidence modes, not complete financial operating modes. A reconciled
pool can still be insolvent. `qualified_diagnostics` additionally requires unique,
complete, current, qualified marks from configured sources; future, expired,
corrupt, wrong-unit or missing marks fail the dependent view. P09 must compose
this gate with user, gross/net, capital, liquidity and pending-outcome admission.
Plain `diagnostics` remains an analytical calculation, never an admission API.
Neither restricted nor frozen evidence suppresses ingestion of an actual adverse
execution, funding payment or correction.

## Verification, provenance and remaining owners

Tracked `crates/kernel/tests/economics.rs` covers V04, frozen inventory across
entries/partial closes/reversals, several open boundaries, both recognition and
settlement orderings, missing inputs, signed rebates, fee-inclusive PnL, distinct
native/private basis, rounding versus actual differences, source corrections,
exact snapshot explanations, conflict/duplicate replay, aging, stale marks, invalid
cuts/units and failure atomicity at signed extrema. The independent small-integer
funding oracle covers 500 signed allocation/rounding configurations. The
`test-support` normalization fixture tests lossless strings, differing report
conventions, unknown revisions, absent fields and overprecision without network.

These are finite executable tests and hand derivations, not machine proofs,
independent audit or live venue qualification. The unavailable QEDGen guide/tool
noted in ADR 0003 was not substituted with a fabricated formal-verification claim.
The DeFi skill's checked-arithmetic and adversarial-testing guidance informed the
implementation; it introduced no new live test or architecture dependency.

Consulted first-party local source fingerprints (SHA-256), promoted only as
sanitized requirements/vectors, not runtime imports or third-party source copies:

| Source under `work/` | SHA-256 |
| --- | --- |
| `specs/financial-model.md` | `d729b31a4d01aa3b25f1236d9e0f59e2b9e01cc85cb0ada6c5bce8deeedb476f` |
| `specs/financial-model/model.cjs` | `853d4955f0a3b0c13125558105b37566bce374e764e69782d215938aee0ef74f` |
| `analysis/reconciliation-scenarios.cjs` | `73f2806853095a173a25081b8e656f3bc6b5a95c5f923e824895136ed6b43d67` |
| `venues/pacifica/financial-mapping.md` | `ec906a9c6e70f5c626dfe31b5560ab144f2260f8a2efb8bf241c97136361b341` |

The older reconciliation study contains hypothetical BULK behavior, not a native
Pacifica template. Pacifica's mapping still lacks a live funding sample, complete
fee/rebate algorithm and explanation of its observed two-micro-unit discrepancy.
No precision tolerance, native zero, rate formula or rebate promise is inferred
from it. Qualify those in P13/P23 before enabling the capability.

P05 owns bounded durable raw/normalized storage, a complete wire codec, atomic
consumption and restart/CAS tests. Current vectors, full proposal cloning and
before/after audit copies are a refinement baseline, not an unbounded production
service. P06 adds authenticated encrypted durability; P07/P18 authorization;
P09 joined admission; P13 native authentication/normalization and causal coverage.
Missing policy/source capability stays disabled rather than weakening these gates.
