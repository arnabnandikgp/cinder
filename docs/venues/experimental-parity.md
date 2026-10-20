# Pacifica / BULK experimental capability comparison

Evidence cut: 2026-10-20. This compares implementation and observed evidence,
not launch readiness. A native paper runner, synthetic joined acceptance and a
measured deployment establish different properties; none substitutes for the
others. [Acceptance](../implementation/UNIT-ACCEPTANCE.md),
[qualification scope](../operations/unit-qualification.md).

| Capability | Pacifica | BULK | Boundary |
| --- | --- | --- | --- |
| Shared owner, unit discovery, per-unit grants/operations | Implemented and tested | Same common implementation | One accepted history and global revocation epoch; unit-local money, holds and attribution |
| Confidential Node/browser HTTP and WebSocket | Joined production SDK/Noise acceptance passes locally | Same carrier with explicit unit binding | Synthetic trust/ports in local acceptance are not AWS attestation |
| Native order/cancel signing and bounded workers | Adapter and joined worker implemented | Binary signing, nonce discipline and joined worker implemented | Default fresh shipping books are not financially ready; wiring cannot activate trading |
| Native paper order/cancel/open-close evidence | Historical scoped testnet executions | October 20 scoped direct native run; final flat/revoked | Neither establishes a live paper lifecycle through the current assembled Cinder service |
| Ordinary fills/fees and replay | Scoped normalization/attribution tests; historical discrepancy retained | Observed slot/sequence identity fallback, raw re-decoding and adverse regressions | No complete causal frontier or universal execution/funding/ADL qualification |
| Native account/history reads | Bounded retained observation controller | Bounded account/trades controller | Pagination, not-found and an ACK cannot invent financial facts |
| Two-unit measured assembly | Version 8, separate private configuration and agent role | Same image, distinct account/agent/source/unit | Fresh managed accounts are empty; no seeded balances, reserve capital or readiness |
| Changed-image Nitro | Production Node/Chrome private access, original-receipt restart and independent fencing pass | Same actual read/admin checks pass on the same image | [Exact hardware receipt](../operations/unit-hardware-20261020.md); native fresh-account queries were unavailable/404, not credit |
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

The changed-image AWS run is limited to read/admin access, restart, fencing and
bounded account queries. It cannot close the separate assembled financial
lifecycle criterion in P40. Do not seed synthetic credit/risk cuts into a
shipping manifest or describe a direct runner as an SDK trade.

## Next integration gate

Before a third venue, review this comparison and the remaining assembled
financial-lifecycle dependency. Any next proposal must name the actual capital,
credit, native account/agent authority, original-operation reconciliation and
finite cleanup mechanism needed for its specific test. Production/customer-fund
activation and the deferred Pacifica credit investigation are not implicit next
steps or inherited permissions.
