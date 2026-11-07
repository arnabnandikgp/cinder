# ADR 0009 — Joined bounded admission and peak-path capital

2026-09-30. P09 derives risk from the existing ledger, order/funds lifecycle and
shared holds. It adds no balances or independent risk ledger. All parameters in
tests are synthetic; calibration, scenario coverage and release remain gates.

## Atomic admission and policy

`Control::Risk` installs trusted immutable-version configuration against the exact
financial version or records a customer-authenticated private leverage selection.
The journal CAS also binds all lifecycle/hold state. Selection changes margin,
not positions, basis, equity, native leverage or maintenance. Scaled 10,000 units
represent 1x; 25,000 represents 2.5x. The maximum is market-specific. A later cap
downgrade uses `min(selected, cap)` without erasing the old preference or forcibly
liquidating an otherwise maintenance-healthy position.

Risk market caps replace the P08 private-margin prerequisite when a risk policy
is installed; P08's qualified price/evidence cut and native initial-margin rules
remain inputs. Native maintenance must be strictly below native initial margin;
private maintenance must be strictly below initial margin at maximum leverage.
Configuration is not supplied by a customer endpoint. P18 authenticates the
selection digest, request, policy, epoch and expiry. Policy/scenario coverage also
has an expiry. Construction of a Rust struct is not authentication/calibration.

Ordinary reservations, order acceptance, funds preparation and exposure recompute
the joined gate. A final check on the **whole control result** prevents exposure
followed by a stricter policy later in the same transaction from bypassing the
final cut. Failed controls roll back together; preceding actual financial facts
remain posted. Configuration alone may deliberately record restrictions. Scoped
cancels and proven commitment release remain available during containment.

## Pending outcomes without a combinatorial simulation

For each owner/market, let current signed lots be `q`, remaining independent buys
`B`, remaining independent sells `S`, common qualified mark `p`, and configured
quote conversion `v(q,p)`. Every partial outcome lies in `[q-S, q+B]`:

```
worst absolute quantity = max(abs(q-S), abs(q+B))
private IM = ceil(abs(current notional) * 10_000 / min(selection, cap))
private MM = ceil(abs(current notional) * maintenance_bps / 10_000)
```

Each remaining order adds a conservative nonnegative execution-cost bound:
buy quantity × `max(maximum price - mark, 0)` or sell quantity ×
`max(mark - minimum price, 0)`, plus its entire authorized fee ceiling. Positive
favorable execution gains never finance another order. Rational magnitudes and
requirements round upward; actual fills retain the kernel's exact semantics.
Both inventory endpoints must remain representable. No per-atomic-lot iteration.

Let `M_out` be margin at the worst endpoint plus all adverse cost, `M_now` current
margin, `H_order` explicit order reservations and `H_other` non-order holds:

```
required = max(M_out, M_now + H_order)
free customer collateral = conservative equity - H_other - required
```

This enforces the computed outcome envelope even if a caller supplies a tiny hold,
without adding that same order margin twice. Payout commitments are never netted
against it. Positive unsettled funding is excluded from usable equity. Native
margin uses the pooled net interval, whereas concentration checks sum customers'
gross intervals independently. A private reduce-only close can increase pooled
native exposure, so its label gives no exemption.

These are conservative bounds, not an exact optimizer: some extrema cannot occur
together; favorable fills are ignored. During hypothetical closeout replay,
remaining commitments are retained, not optimistically released by invented
terminal certificates. Throughput/capital efficiency is not claimed qualified.

## Capital, liquidity and every prefix

Starting free capital is bounded by both house equity less house initial margin
and positive-claim backing surplus. Nominal transit and positive native unsettled
funding are deducted from eligible backing. Existing house holds and additional
customer deficit exposure from adverse pending costs are deducted once. Negative
customer equity is not a collectible asset. Reserve designation/coverage-specific
commitments and absorption are P10 extensions of these same resources.

Each nonempty versioned scenario contains complete hypothetical marks, optional
normalized executions and location/deadline liquidity observations. Executions
replay through **the same kernel transitions on a noncommitted clone**; no parallel
accounting model contributes assets. A missing route, duplicate/conflicting event,
unexplained PnL or invalid precision fails qualification. Scenarios cannot inject
deposit receipts, promised fundraising, earned broker fees or hoped-for rebates.
House executions in a stress fixture are hypothetical costs/exposures, never
authority for speculative live house trading. Later phases extend the allowed
stress vocabulary with their actual claim/liquidation/ADL transitions.

The evaluator examines the starting cut, each price shock **before execution**,
and every execution inside a grouped step. Recovery later in the same step cannot
hide an earlier breach. With current eligible free capital `F0`, minimum over all
tested prefixes `Fmin`, and explicitly configured residual buffer `B`:

```
peak erosion = max(0, F0 - Fmin)
target = B + peak erosion
```

Losses are not capped at the available fund. The report retains negative free
capital and positive-claim shortfalls. It distinguishes insolvency, immediate
illiquidity, private initial/maintenance, native initial/maintenance, capital
target and concentration breaches. An actual outside shock still enters the
ledger. The current state must satisfy ordinary admission; stress paths must
preserve backing, capital buffer, native maintenance, concentration and deadline
liquidity. Stressed private maintenance breaches remain explicit, not hidden
automatic liquidation. P11 supplies the concrete bounded exit policy.

Vault and native liquidity are separate. Native usable cash is capped by native
cash and pending-inclusive free collateral. For each location/deadline, apply its
explicit availability fraction, aggregate existing commitments and additional due
obligations, then compare once. Each step lists both locations exactly once;
duplicate independent uses of the same available cash reject. In-transit assets,
fundraising and another location's surplus are not instant payment resources.

Finite path success proves neither scenario completeness nor an exhaustion
probability. Up to 32 paths, 64 steps/path and 64 executions/step are supported,
subject to canonical record and aggregate work bounds. Unknown/oversized coverage
fails closed. One quote pool/native risk unit is implemented; no sibling pool
customer sweeps or unqualified subaccount isolation. P10–P12 and release review
must join claim, liquidation and RF1/RF2 behaviors before live use.

## Verification and provenance

Wire revision **4**, engine **5**; prior semantics are explicitly rejected, not
silently migrated. Risk policy and authenticated selections join canonical
transactions and replay commitments. Derived reports are reproducible, not stored
equity or authorizations reusable against another journal head.

The 18-group tracked risk suite covers 540 partial/price/order-ordering outcomes,
gross/net separation, leverage/payout overlap, cap downgrades, every-prefix capital
depletion and recovery, repeated costs, location shortages, impossible/future
resources, stale policy/marks, final-cut exposure ordering, actual adverse facts,
exact replay and codec round trips. Tests are bounded, not a machine proof.

| Promoted local reference | SHA-256 |
| --- | --- |
| `work/specs/financial-model/capital-envelope.cjs` | `d8f133cd9ec5ccfacb9d057cabd5d4faa2de0f03a8e92cfcd3a80cdabaa34deb` |
| `work/specs/financial-model/capital-envelope-checks.cjs` | `c7be5a1d3398d4b49314cebfb6dbf7f22410c16535f18631d25adcd41f6ff159` |
| `work/specs/financial-model/risk-policy.cjs` | `c7059f1821a73948f4cba66e2b49ca7053ede50ff94df02cd52e9f41e03a0500` |
| `work/specs/financial-model/risk-policy-checks.cjs` | `f1dfb2bb9e25a0a3519ca501be4b4c0273ed6647d45aa11b1a9be08bb88092b9` |

Historical profitable-user ADL ranking in the risk study is not promoted; approved
RF1/RF2 govern P12. No ignored research, venue call, wallet or deployed risk service
is needed to run these tests.
