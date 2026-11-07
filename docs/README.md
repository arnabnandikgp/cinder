# Documentation guide

Start with [architecture](architecture.md) for how Cinder works, or
[status](status.md) for what is implemented, qualified and still blocked.
This directory documents the backend; public product/API documentation is
maintained separately in the Mintlify repository.

## Reading paths

| Goal | Read |
| --- | --- |
| Understand the complete system | [Architecture](architecture.md) → [financial contract](implementation/BASELINE.md) |
| Build or test a change | [Contributor guide](../AGENTS.md) → [development](development.md) → relevant component/decision |
| Integrate a client | [Private SDK](../clients/private/README.md) → [private reads and subscriptions](architecture/private-read-contract.md) |
| Understand custody or recovery | [Custody decision](architecture/0030-venue-directed-escrow.md) → [program/client setup](../programs/README.md) → [unit recovery](architecture/0041-unit-custody-recovery.md) |
| Compare venue behavior | [Pacifica](venues/pacifica.md), [BULK](venues/bulk.md), [experimental comparison](venues/experimental-parity.md) |
| Prepare an explicitly authorized qualification | [Operating references](operations/README.md); a runbook is not live permission |
| Resume implementation | [Current handoff](implementation/TRACKER.md) → relevant [acceptance criteria](implementation/PLAN.md) |
| Verify a claim or investigate a regression | [Evidence index](implementation/EVIDENCE.md) → exact dated receipt or named test |
| Explain a previous design choice | [Decision index](architecture/README.md) → [historical archive](archive/README.md) if needed |

## Debugging entrypoints

- **Wrong cash, basis, funding or risk result:** start with `crates/kernel` and
  the financial contract. Compare the original event/units, not display equity.
- **Duplicate effect or lost state:** inspect journal operation/economic IDs,
  accepted head, replay, holds and source evidence before any resend.
- **Order never dispatches:** distinguish API recording, financial admission,
  native preparation, exposure and execution. Check unit policy, authority,
  qualified observations and remaining budgets; do not bypass readiness.
- **Native data is rejected:** inspect the venue codec/profile and original
  lexemes. A snapshot is not a ledger setter; unknown coverage is not zero.
- **Reads fail after a write/restart:** inspect current witness/epoch, accepted
  publication, history packs and transport separately. Cached views are not authority.
- **Payout/recovery is blocked:** inspect final-payment evidence, shared counters,
  cutover/fencing and actually available backing. A heartbeat/root cannot pay alone.

## Documentation ownership

| Document | Owns | Keep out |
| --- | --- | --- |
| [Status](status.md) | Capability boundary, active/deferred gaps, evidence pointers | Full command transcripts |
| [Architecture](architecture.md) | Current components, flows and trust boundaries | Per-PR progress tables |
| [Financial contract](implementation/BASELINE.md) and [decisions](architecture/README.md) | Stable requirements, equations and approved rationale | Unapproved policy defaults |
| [Development](development.md) | Reproducible commands and test/CI map | Live credentials or deployment permission |
| [Tracker](implementation/TRACKER.md) | Milestone summary and one current next-agent handoff | Repeated daily session logs |
| [Evidence](implementation/EVIDENCE.md) | Test/receipt/provenance navigation | Duplicate interpretations of historical runs |
| [Archive](archive/README.md) | Dated plans, complete handoffs and exact-source receipts | Current operating authority |

Keep linked paths and explicit anchors valid when moving a document. The offline
contract suite checks links, phase criteria, archived handoff completeness and
test discovery. Archive fallback applies only to unchanged phases: modified
scope, criteria, status or progress/work references require a current phase
handoff. Current records override archived predecessors; missing evidence or a
merge SHA cannot be supplied by the archive by assumption.

Original ignored `work/`, local docs drafts and unrelated files are preserved.
Only sanitized material needed for a reproducible checkout belongs here. Historical
receipts retain their original source and limits even after code is merged or fixed.
