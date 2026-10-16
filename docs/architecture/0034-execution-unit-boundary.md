# ADR 0034 — Proposed two-execution-unit state boundary

Date: 2026-10-16. **Approved direction; not implemented Runtime/API behavior.**
P28 implements observations. P29's [new-history journal foundation](0035-unit-journal-foundation.md)
implements the bounded first state/replay slice, not native activation or the
complete composition described below.
[Direction](../implementation/SECOND-VENUE.md), [baseline](../implementation/BASELINE.md),
[custody](0030-venue-directed-escrow.md), [observations](0033-bulk-financial-observations.md).

## Minimum composition

```text
DeploymentState — one accepted journal/head, writer and freshness witness
├── registered wallets/recipients and customer authority epochs
├── operation index: (customer, request ID) → immutable unit + intent digest
└── configured units
    ├── A: Pacifica account, quote, route, keys, policies and books
    └── B: BULK account, quote, route, keys, policies and books
```

Each unit initially has one pooled native account, not one account per customer.
An enrolled customer may own entitlements in both. Per-unit books are components
of the same accepted state, not independently persisted ledgers reconciled later.
Dashboard sums grant no cross-margin. Routing, inter-unit transfers and automatic
rebalancing are deferred in this initial composition.

## State ownership

Current journal::model::State has one Config/Ledger plus its holds, attempts,
orders, funds, escrow, risk, evidence and recovery. Proposed new-schema split:

| Owner | State |
| --- | --- |
| Deployment | Original domain; one transaction sequence/head; enrolled wallets/recipients; global customer revocation epochs; immutable unit registry; global request routing index |
| Unit descriptor | Opaque UnitId; venue/environment/native account/source scopes; quote/precision; kernel Config; custody config/PDA domains; native key-role instances and policy commitments |
| Unit state | Existing kernel Ledger and its holds, attempts/orders, funds/escrow, native cuts/readiness, raw evidence, risk/protection/close/restoration and recovery permission |
| Accepted transaction | Explicit schema, selected UnitId, expected shared head, authenticated origin and deterministic effects/evidence/holds, committed atomically |

UnitId is a configured nonzero opaque identity, not a user-supplied symbol, URL or
account alias. Descriptor commitment binds sources, physical account, atom/market
precision, policies, route and key instances. Changing the physical account or
claim domain is a migration/new unit, not relabeling existing obligations.

Initially use distinct native accounts, funding wallets and existing-program
custody configs per unit. This avoids shared fungible broker inventory before an
allocation controller exists. A future shared gas payer needs explicit fee
reservations and cannot spend another unit's customer quote. The current Solana
program's config/domain separation suffices; unit separation alone does not
justify additional programs.

## One atomic journal and bounded workers

Each mutation selects one unit. Authenticate immutable routing, read current
accepted state, compute effects, then atomically commit evidence, consumed keys,
holds and postings under the single head. Native I/O stays outside the writer
critical section with existing before-exposure/after-observation rechecks.
Do not introduce one independent encrypted journal/witness per venue.

Keep kernel request keys unit-local if possible. Before exposure, a deployment
index pins `(AccountId, RequestId)` to `(UnitId, economic digest)`. Reusing the
request at another unit conflicts rather than executing twice. Session/auth
refresh can reauthenticate the same operation, never move it. Attempts, cancels,
payouts and lookups derive their original unit from this index; caller hints
cannot override durable ownership.

Native input routes through configured source → unit, not JSON symbols/accounts.
Same symbols or IDs cannot consume another unit's event, discharge its operation
or resolve its raw input. Unit-local unresolved evidence restricts dependent
actions, not unrelated financial inventory. Bound workers/rate budgets per unit
and preserve cancellation/recovery capacity. Financial distress can restrict A
without using B's backing. Global writer/freshness uncertainty still fences all
mutations: this design shares operational availability, not customer losses.

## Versioned API and permissions

A new application signature/schema explicitly binds UnitId into authenticated
and stable economic bytes. Unsigned headers or a new JSON route hint cannot
safely select a unit for old signatures. The existing confidential transport can
carry the new payload; release configuration binds the schema/unit registry.

Enrollment stays deployment-level. Grants bind one unit and market. READ permits
that unit's account views, never accidentally all venues. Preserve global
revoke-all epochs across units, without implicitly cancelling resting orders.
Narrower revocation or aggregate agent permissions are separate future scope.

Old v1 requests/grants retain only their original immutable legacy unit. Maintain
explicit single-unit compatibility mode or refuse v1 in new multi-unit mode;
never default ambiguous old operations into a new venue. Private discovery lists
configured choices; balances, subscriptions and operation views remain scoped.
No native pooled authority or frontend integration is implied.

## Assets, house capital and recovery

For each unit enforce existing identities independently in exact units, e.g.
`N_u = sum_i e_iu + h_u`. A display sum is not usable collateral; differently
denominated units require a separately labeled valuation view. A Pacifica deposit
does not increase BULK margin. The same house asset cannot back two free-capital
designations simultaneously.

Initially house capital has unit-specific actual custody/location backing.
Later redistribution needs source reservation, transit accounting, qualified
arrival and once-only release, not two setters or atomic native calls. No shared
guarantee fund, cross-unit netting or hidden customer loss sharing is introduced.
RF1/RF2 apply within the original risk unit. Future native subaccount cohorts
become distinct units only after their actual isolation is qualified.

Custody retains escrow → fixed funding wallet → venue and a unit-specific PDA
payout vault on return, with the same registered customer recipient. Preserve
the original `(vault config, customer, asset, epoch)` normal/recovery paid-counter
meaning. Moving an old claim to a fresh config cannot reset its payment history.
Legitimate new-unit claims are separately funded, not copies of old entitlements.

Initial operator-assisted recovery fences the shared deployment, reconciles each
unit's final participant inventory, settles/returns assets and builds separately
backed claims. The common accepted head ties cuts together, but A's return does
not fund or finalize B. No heartbeat-only activation, stale root, unreturned
collateral payout or aggregate solvency proof. Continued trading during another
unit's recovery is not promised by this first implementation.

## Rollout and required future tests

Implement new state/transaction/API versions only after this contract is reviewed.
First target new-history offline composition; legacy single-unit history remains
readable and tested under its codec. Do not import an old snapshot as a genesis
that clears consumed keys, holds, paid counters, recovery claims or attempts.
Active-state migration needs its own freeze/reconcile/ownership-preserving plan.

Required joined-runtime acceptance before activation:

- Same wallet/symbol/native ID/private request across A/B: routing, conflicts and
  ownership survive restart/replay without cross-unit settlement.
- Grant A cannot read/trade/cancel B; unit/session/policy/version tampering fails
  before exposure. Revoke-all remains global, resting orders remain explicit.
- Concurrent admission/payout cannot spend customer, house or location assets
  twice. A's credit/mark/funding cut/terminal page cannot release B's holds.
- Local liquidation/ADL, exhausted protection, late fills and held raw inputs keep
  other-unit books unchanged. Storage uncertainty fences the shared writer.
- Crash/response loss preserves one head, unit registry, participant inventory,
  routing index and original attempts; no unknown-outcome resend is invented.
- Unit/config/PDA/epoch/recipient mismatch cannot bypass original payout/recovery
  counters. Every accepted participant/unit claim has non-duplicated backing.

These are proposed future tests, not P28 passes. Native source/key/fee/bridge
qualification and a separately approved changed-image run remain gates. Missing
test rails cannot trigger fallback to mainnet, another venue or an unrestricted
key. P28 leaves shipping Runtime, journal schema, API v1 and Solana ABI unchanged.
