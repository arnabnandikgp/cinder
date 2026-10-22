# Historical plans and qualification evidence

This archive preserves implementation knowledge without making old session logs
the project's entrypoint. Start with [current documentation](../README.md) and
[status](../status.md). Dated approvals, expired budgets and historical next steps
are not current operating authority.

## Full implementation snapshots

- [Phase scope and acceptance narrative](implementation/PLAN-20261023.md): all
  original implementation sequencing and scope clarifications.
- [Complete phase handoffs and session logs](implementation/TRACKER-20261023.md):
  original commands, counts, review/merge evidence, failures and deviations.
- [Research/promotion register](implementation/EVIDENCE-20261023.md): source
  hashes, finance/TEE/venue studies, prototype mappings and qualification limits.
- [Previous architecture narrative](architecture-20261023.md): historical
  phase-by-phase explanation, superseded as the current overview.

Current [criteria](../implementation/PLAN.md) preserve all 46 phase IDs,
dependencies and acceptance checkboxes. The compact
[tracker](../implementation/TRACKER.md) references archived handoffs explicitly;
current phase records override their historical predecessors when work resumes.
Changed scope, criteria, status or work references require such a current record;
an unchanged `in progress` label cannot conceal new criteria or progress metadata.

## Exact-source receipts

| Receipt | Scope |
| --- | --- |
| [Initial Nitro](qualification/nitro-hardware-2026-10-02.md) and [follow-up](qualification/nitro-hardware-followup-2026-10-02.md) | Original measured application, key release, transports, restore and failure boundaries |
| [Retained-pack hardware](qualification/P23-PACKS-HARDWARE.md) | Exact history-growth/read-only application evidence and cleanup |
| [Pacifica native semantics](qualification/P23-NATIVE-SEMANTICS-RECEIPT.md) | Scoped original deposits/withdrawals/settings and explicit native evidence gaps |
| [BULK native paper](qualification/bulk-paper-20261020.md) | Direct native orders/fills/cancel/revocation; not an assembled SDK lifecycle |
| [Two-unit hardware](qualification/unit-hardware-20261020.md) | Read/admin/restart/fencing on that exact image |
| [Fresh paper hardware](qualification/unit-paper-hardware-20261021.md) | Bootstrap failure, corrected SDK opening/fill, refused close and separate safe teardown |

## Qualification and planning context

- [P23 financial gates](qualification/P23-FINANCIAL-GATES.md),
  [native evidence contract](qualification/P23-NATIVE-EVIDENCE.md),
  [bounded native proposal](qualification/P23-NATIVE-QUALIFICATION.md).
- [Concurrency remediation](qualification/P23-REMEDIATION.md),
  [growth diagnosis](qualification/P23-GROWTH-DIAGNOSIS.md),
  [next-run proposal](qualification/P23-NEXT-RUN-PROPOSAL.md).
- [Original API scope](qualification/API_SCOPE.md),
  [second-venue sequence](qualification/SECOND-VENUE.md),
  [two-unit qualification proposal](qualification/UNIT-QUALIFICATION-PROPOSAL.md).
- [Approved paper-test boundary](qualification/P40-PAPER-GATE.md) and
  [consumed paper-run proposals](qualification/P40-PAPER-RUN-PROPOSAL.md).

These are retained rationale/evidence, not instructions to repeat an experiment.
No private `work/` research, keys, signed requests, raw customer records or local
untracked drafts were deleted or added to Git. Duplicated session narrative was
removed from active entrypoints after preserving the full snapshots; dated
receipts were relocated with their contents intact apart from link rebasing.
The pre-cleanup tree is also preserved at source commit `0ae5218`.
