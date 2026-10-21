# Pacifica / BULK experimental capability comparison

Evidence cut: 2026-10-21. This compares implementation and observed evidence,
not launch readiness. A native paper runner, synthetic joined acceptance and a
measured deployment establish different properties; none substitutes for the
others. [Acceptance](../implementation/UNIT-ACCEPTANCE.md),
[qualification scope](../operations/unit-qualification.md).

| Capability | Pacifica | BULK | Boundary |
| --- | --- | --- | --- |
| Shared owner, unit discovery, per-unit grants/operations | Implemented and tested | Same common implementation | One accepted history and global revocation epoch; unit-local money, holds and attribution |
| Confidential Node/browser HTTP and WebSocket | Joined production SDK/Noise acceptance passes locally | Same carrier with explicit unit binding | Synthetic trust/ports in local acceptance are not AWS attestation |
| Native order/cancel signing and bounded workers | Adapter and joined worker implemented | Binary signing, nonce discipline and joined worker implemented | Default fresh shipping books are not financially ready; wiring cannot activate trading |
| Native paper order/cancel/open-close evidence | Historical scoped testnet executions | Direct native lifecycle; October 21 actual encrypted SDK opening/fill attribution, but Cinder close refused | Separately approved local paper teardown finished flat/revoked; it does not qualify the assembled SDK close/cash bridge |
| Ordinary fills/fees and replay | Scoped normalization/attribution tests; historical discrepancy retained | Observed slot/sequence identity fallback, raw re-decoding and adverse regressions | No complete causal frontier or universal execution/funding/ADL qualification |
| Native account/history reads | Bounded retained observation controller | Bounded account/trades controller | Pagination, not-found and an ACK cannot invent financial facts |
| Two-unit measured assembly | Version 8, separate private configuration and agent role | Same image, distinct account/agent/source/unit | Fresh managed accounts are empty; no seeded balances, reserve capital or readiness |
| Changed-image Nitro | Node/Chrome private access, original-receipt restart and independent fencing pass on October 20's exact image | October 21 corrected image passes bootstrap, SDK opening/fill, partial restart and live fencing; active-account precision/funding blocks its close | [October 20](../operations/unit-hardware-20261020.md) / [October 21 partial results](../operations/unit-paper-hardware-20261021.md); full corrected-image financial/admin acceptance remains open |
| Escrow/sweep/return/payout/recovery | Program, SDK and journal integration tested offline | Common unit composition tested; no qualified native rail | Unswept escrow refund is distinct from withdrawal of collateral held at a venue |
| Live funding/credit/returns | Missing-credit investigation remains explicitly deferred | Supported funding/return interface remains unqualified | Neither is a launch-qualified round trip; do not substitute a mock credit |
| Venue-backed recovery | Offline orchestration and shared payment protections | Same common machinery with native dependency unresolved | Operator-assisted reconciliation and actual backing are prerequisites, not heartbeat-only exit |
| Fee-tier savings, calibrated risk/capital and production assurance | Separate gates | Separate gates; native fee-tier endpoint returned 404 | No guaranteed lowest fee, native margin equivalence, self-custody or blanket loss protection |

## What parity means here

BULK now has the same common experimental unit API, private carrier, accepted
history, worker and adapter-composition surfaces as Pacifica. Venue-specific
capabilities still need their own evidence; a common interface does not imply
identical native behavior. BULK's [paper receipt](bulk-paper-20261020.md) and
Pacifica's [qualification map](pacifica.md) retain the differing native limits.

October 20's AWS run is limited to read/admin access, restart, fencing and
bounded account queries. October 21's first paper run refused zero-slot empty
histories before allocation. Its separately approved corrected-image run passed
the actual encrypted SDK opening/fill path, but active-account precision/funding
containment refused Cinder's close. Final flat/revoked state came from a separately
approved local paper teardown after fencing, not from a passing Cinder close.
Full financial acceptance remains open. Do not seed synthetic credit/risk cuts,
describe direct teardown as an SDK trade, or infer a reconciled cash bridge.

## Next integration gate

Before a third venue, review this comparison and the remaining assembled
financial-lifecycle dependency. Any next proposal must name the actual capital,
credit, native account/agent authority, original-operation reconciliation and
finite cleanup mechanism needed for its specific test. Production/customer-fund
activation and the deferred Pacifica credit investigation are not implicit next
steps or inherited permissions.
