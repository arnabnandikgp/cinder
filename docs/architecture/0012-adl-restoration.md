# ADR 0012 — Qualified ADL cuts and bounded RF1/RF2 restoration

2026-09-30. This implements the approved original-basis economics and fixed
proportional allocation over the existing single native risk unit. It is not a
promise to prevent every healthy position reduction, a live ADL recognizer, or
evidence that native subaccounts provide independent margin estates.

## Facts, attribution and dispatch are different steps

`Observe` books the actual native forced execution, fee and reported-PnL residual
to the venue and named suspense. A stale/unqualified cut remains recorded. The
observation captures the prior private books. A separate request-keyed declaration
requires that exact cut, no mixed house inventory at observation, no unresolved
attribution/funding/source issue, and unchanged private books. The journal also
requires earlier orders to have completed. It never chooses customers by looking
at a later, more convenient portfolio.

Declaration apportions the native reduction across the frozen same-side private
quantities, clears only that incident's suspense, and records each customer's
removed signed basis, realized result and fee. No caller may supply custom weights,
historical allocation manifests or a replacement tape. Unsupported cuts remain
contained for reconciliation; there is no fallback haircut policy.

The first controller supports one bounded IOC attempt per incident. Its signed
authorization-port binding includes incident, attempt, quantity, financial version,
immutable price/depth/fee and lifetime-cap revisions, epoch and expiry. Actual
authentication belongs to P18; a digest is not a customer-supplied bearer token.
Missing policies disable preparation. Preparation competes for the existing house
resources, and persists the route, hold and exact abstract message before exposure.
This phase sends no network request or signature.

## RF2 schedule and bounded-resource qualification

For original same-side quantities `w_i` and native reduction `D`, largest remainder
apportionment produces `D_i`; exact ties use canonical account bytes. The fixed
restoration schedule treats each owner's kth lot as a unit job:

```
release(i,k)  = floor((k-1)*D/D_i) + 1
deadline(i,k) = ceil(k*D/D_i)
```

At each slot choose the available earliest deadline, then canonical identity.
This is the approved EDF schedule, not highest-average allocation or a newly
apportioned result after each partial fill. Its prefix counts satisfy
`floor(r*D_i/D) <= a_i(r) <= ceil(r*D_i/D)`; the composed residual error against
original weights is below two lots. Account splitting can still change ties.

The compiler retains repeated blocks rather than one item per atomic lot. For a
block of length `L` awarding `c_i` jobs, repeating it shifts each normalized job's
release/deadline numerator by `h * (c_i*D - L*D_i)`. Exact integer inequalities
certify, for every offset and competitor, that the winner remains available with
the same normalized deadline and every competitor is either unavailable or cannot
win. Only that certified prefix of block repetitions is skipped. Complete small
gcd periods are compiled directly; other profiles use 64-job blocks.

Current explicit envelope: at most 64 owners, total lots <= `i64::MAX`, 2,000,000
compiler scan units, 1,024 segments and 65,536 stored jobs. Prefix lookup visits
stored jobs, not the requested number of lots. Exceeding any bound returns no plan;
it never changes allocation rules or launches an unbounded fallback. Some enormous
irregular profiles are deliberately unsupported even though trillion-lot equal,
near-equal and skewed profiles compress. Automatic declaration/restoration for an
unsupported profile stays disabled; actual native facts remain visible in suspense.
This is a bounded-resource implementation, not a universal constant-time scheduler.

The affine certificate is a derivation checked by differential tests, not a
machine-checked theorem or independent audit. The promoted tests compare every
prefix for 3,905 small vectors, 250 seeded uneven vectors, the 10,000-lot skew,
starvation/Alabama examples, integer precision and account-splitting counterexamples.

## RF1 postings and rounding

Only actual eligible replacement lots advance the frozen schedule. For an owner's
original amount `D_i`, each signed basis, realized result and fee piece uses
`trunc(x*(n+k)/D_i) - trunc(x*n/D_i)`, retaining all cumulative residues. Restored
private quantity regains its original basis. Private cash reverses that quantity's
ADL-realized result and refunds its original ADL fee. There is no synthetic funding
payment during the missing-position interval; the normal funding cut sees only
actually held quantities.

House owns the signed replacement-price difference, replacement fees and fee
refund. Favorable price differences belong to house too. The actual whole native
fill's exact value is partitioned with explicit residue, so artificial owner legs
need not each be independently representable at native tick/lot conversion. The
cash-minus-basis bridge and quantity sum hold after each execution. Native PnL
report differences retain the existing named suspense issue.

Preparation reserves worst bounded adverse replacement gap, the full original fee
refund bound, replacement fees, configured extra support and two atoms per owner
for split-rounding bounds. P09 also checks independent private remaining-quantity
intervals, native pending exposure, private/native margin, liquidity and capital at
every configured stress prefix. Favorable refunds do not finance admission. These
intervals can be conservative relative to the correlated actual quota schedule.

An explicit administrator-installed lifetime close/replacement cost cap counts
cumulative positive house costs plus active support holds. New identities, favorable
fills, capital top-ups and policy revision do not reset prior spend. Zero disables
new support. Holdings stay conservatively encumbered at their original budget until
qualified completion; this can count incurred cost plus the full remaining hold
against a cap. It is intentional over-reservation, not an asset or second loss.
Live numerical calibration and coverage/priority remain G02, not fixture defaults.

## Changed intent, late facts and completion

A customer can explicitly decline future restoration under their authenticated
epoch, including when fully reduced. Accepted private reduce-only intent also
voids remaining awards; ordinary actual private fills/close fills void conflicting
restoration. Voided slots are not redistributed. Their actual native executions,
oversized remainders, wrong-side/out-of-bounds fills or contradicted terminal facts
become contained house inventory. P11's funded existing-house unwind recognizes
these incidents. This is provisional exception accounting, not erasure of an
independently owed remediation claim.

Source execution time must be within the exposed attempt's deadline and not before
its durable exposure or after receipt. The adapter must qualify a common time
domain; receive time alone is not execution time. An in-time fill received after
expiry remains eligible. Ordinary fill bodies lacking this qualification cannot
silently acquire RF1 eligibility. Current capital shocks or later policy changes
do not revoke economics owed on a previously exposed, valid replacement.

ACK/cancel ACK/timeout never releases support. The P07 exact execution-set, quantity
and source-cut terminal witness governs release, including terminal-before-fill.
Contradictory late facts re-encumber a released hold. Scoped cancellation remains
available during known containment. A never-exposed action may be abandoned; this
is not fabricated venue finality. There is no automatic retry or incident-resume
permission, and there is no future funding on unrestored quantities.

Wire 7 / engine 9 explicitly fence prior history; no silent migration. Raw observe
and source receipts are admitted inputs, but direct allocation/execution-eligibility
controls cannot enter via raw adapter observations. State/event replay includes all
row progress, voids, costs, policy revisions, held resources and exposure time.
