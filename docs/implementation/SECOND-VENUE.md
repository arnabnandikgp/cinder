# Second-venue implementation direction

Date: 2026-10-15. User-approved direction: finish P25 reviews, reassess BULK under
current custody, then take the smallest justified path toward a second venue.
This is a **proposed implementation breakdown**, not approval of numerical risk
limits, a new financial contract, live activity or production activation.
[BULK assessment](../venues/bulk.md), [baseline](BASELINE.md), [tracker](TRACKER.md).

## What changes, and what does not

Keep the financial kernel, accepted journal, confidential SDK/API, custody program,
recovery counters and RF1/RF2 economics. Reuse tests as requirements/vectors, not
as an excuse to copy the entire experimental implementation. No frontend work.

Today `kernel::ledger::Config` explicitly represents one quote pool/native account.
`service::runtime::Configuration` composes one Pacifica profile/gateway/funding
controller; P25 discovery exposes one immutable route. The code is not already
multi-venue merely because identifiers include a venue. Do not rename Pacifica
types into generic names before a second implementation establishes the actual
common boundary.

The minimum ownership model to specify before code is an **execution unit**:

```text
deployment's single accepted history
├── wallet ownership/authentication and explicitly scoped capabilities
├── unit A: venue/environment/account/quote/custody route/policy
│   └── attributed customer + house books, assets, holds, native evidence
└── unit B: venue/environment/account/quote/custody route/policy
    └── attributed customer + house books, assets, holds, native evidence
```

One history does not mean one undifferentiated cash bucket. Per-unit economic
books must be projections/components of that same accepted state, not independent
balance databases reconciled afterward. Define the atomic unit-scoped dispatcher
before extending the current single-pool journal. Public/native evidence and
customer grants must not select arbitrary routes.

Required invariants:

- Every action, source/event ID, attempt, reservation, original funding operation,
  epoch, payout/recovery claim and native credential has an explicit unit binding.
- The same cash/collateral/house capital is not simultaneously available at both
  venues. Dashboard totals grant neither cross-margin nor spending authority.
- Same market symbols, native order IDs or sequence numbers on two venues cannot
  collide, change owner or discharge each other's operation.
- A future inter-unit transfer reserves source, records transit and confirms the
  destination; it is not an atomic pair of native calls or two balance setters.
  Initial second-venue scope can omit this feature entirely.
- Recovery/payout counters retain their original domain/config/customer meaning.
  The existing one-route-per-config Anchor design needs a distinct config per
  unit/cohort, not route relabeling or a fresh payout counter for an existing claim.
- Private grants, reads and subscriptions include or derive the unit scope. No
  accidental widening of existing READ/TRADING rights or mixed aggregate equity.
- Stale/faulted admission rejects new risk, not already-real fills/adverse events.
  Shared signing/rate/storage resources cannot starve cancellation and recovery.

## Smallest useful sequence

These are acceptance slices, **not six new mandatory research phases**. Combine
adjacent work when it forms one independently verifiable PR. Allocate new phase
IDs in PLAN/TRACKER as a slice is actually selected; only P26 is opened now.

| Slice | Deliverable | Exit gate |
| --- | --- | --- |
| Candidate assessment (P26) | Current source pins, financial/custody capability map, evidence limits and this minimum boundary | No invented native guarantees; concrete next qualification proposal; docs checks/review/merge |
| Exact adapter foundation | A narrow BULK decoder/encoder boundary with sanitized canonical vectors and explicit disabled capabilities; no network worker yet | Integers/grids, wide IDs, source/direction/fill/funding/risk attribution, original bytes and error/unknown cases tested; existing checks remain green |
| Environment qualification | Current public reads followed, if useful, by disposable paper signer/order/subaccount experiments | Separately approved run, exact provenance and limits; paper versus actual bridge results separated |
| Joined two-unit runtime | Minimal dispatch plus per-unit books/holds, configured routes and SDK views; reuse each adapter's tested contracts | Same IDs/wallet across units, wrong-unit signatures/credits/fills, crash/replay, concurrent holds, capital and recovery isolation; one authoritative accepted history |
| Financial round trip | Original chain deposit, attributable venue credit, trade/funding/close and finalized return/payout | Supported test bridge and approved economic run. No new confirmation contract or force-credit to make the test pass |
| Hardware qualification | Changed measured application with qualified native/chain scope | Separately approved Nitro run; no inherited old-image evidence |

Offline qualification and unit design can proceed without resolving every
production question. Do not finish a bridge controller before knowing whether the
bridge is testable. If BULK is paper-only, publish that result and retain the
adapter foundation; do not claim a funded end-to-end path or force reopening
Pacifica's deferred investigation.

## Required red-first / adversarial cases

Adapter: large slot/sequence, exact decimal/excess-precision refusal, duplicate
and conflicting trade IDs, REST/WS direction, maker fees/rebates, partials,
reductions/reversal legs, funding boundary/settlement, unknown ADL cause,
changed metadata, incomplete/retention-invalidated history and mixed local/agg
account fields. Preserve actual foreign adverse events under local admission freeze.

Composition: one user's deposits assigned to A do not increase B's margin;
duplicated native IDs remain separate; a wrong-unit source cannot settle an
operation; grant A cannot sign B; partial/unknown funding remains reserved;
unit A distress cannot silently spend B customer claims or protection capital;
restart restores both units and all original attempts; final claim production
covers accepted participants without resetting a unit's ordinary paid counters.

The existing P22 manifest supplies shared lifecycle/fault cases. Promote new
vectors/test registration only when the owning code is added. Ordinary CI stays
offline and credential-free. Reusing existing general tools is preferred over
another venue-specific test framework.

## Next scoped investigation

Before native invocation, prepare a bounded read-only qualification record:
official testnet only; no wallet or signature; fixed public market/account/fee/
history queries; hard request/time/response-size ceilings; retain raw headers and
bytes privately; record unknown coverage and schema conflicts. No API mutation,
faucet, existing customer account, RPC, deposit, trade or cloud session.

Then choose whether a fresh paper account test is worthwhile. It separately needs
permission for its signed faucet/registration/order/cancel/transfer actions. A
Solana funding test additionally needs an explicitly supported non-mainnet bridge,
token/program/cluster pin, bounded funds/fees and original-operation lifecycle.
Never select mainnet because devnet bridge information is missing.

Open design review points before joined-runtime implementation: precise schema/
upgrade strategy for per-unit journal state; deployment-level owner versus unit
account IDs; explicit grant/read unit binding; per-unit capital designation and
recovery-cut/activation coordination. These refine scope/ownership, not RF1/RF2 or
the approved economic contract. Material changes go to the user before code.
