# Architecture decisions

Numbered decisions preserve why a boundary exists. Read the
[current system architecture](../architecture.md) first; older phase descriptions
retain their historical scope and are not a deployment-status report.
The [financial contract](../implementation/BASELINE.md) controls approved economics;
new material changes require an explicit decision, not an incidental refactor.

| Topic | Decisions |
| --- | --- |
| Workspace and exact accounting | [0001 workspace](0001-workspace.md), [0002 primitives](0002-financial-primitives.md), [0003 ledger](0003-unified-ledger.md), [0004 funding/reconciliation](0004-funding-reconciliation.md) |
| Acceptance, durability and scheduling | [0005 journal](0005-durable-journal.md), [0006 encryption/freshness](0006-encrypted-durability.md), [0025 concurrent reads](0025-concurrent-private-reads.md), [0026 history packs](0026-retained-history-packs.md) |
| Orders, funds and risk | [0007 lifecycle](0007-order-lifecycle.md), [0008 funds/payouts](0008-funds-payouts.md), [0009 joined risk](0009-joined-risk.md) |
| Protection and forced events | [0010 claims](0010-protection-claims.md), [0011 liquidation/exceptions](0011-liquidation-exceptions.md), [0012 ADL/restoration](0012-adl-restoration.md) |
| Pacifica and native evidence | [0013 observations](0013-pacifica-observations.md), [0014 execution](0014-pacifica-execution.md), [0023 live observations](0023-live-observation-ports.md), [0027 transfer capture](0027-native-transfer-capture.md), [0028 measured capture](0028-measured-native-capture.md), [0029 demo confirmation](0029-demo-deposit-confirmation.md) |
| Solana custody and recovery | [0015 vault](0015-solana-vault.md), [0016 funding coordinator](0016-funding-coordinator.md), [0017 claims](0017-recovery-claims.md), [0024 confidential chain ports](0024-confidential-chain-ports.md), [0030 venue escrows](0030-venue-directed-escrow.md), [0031 onboarding](0031-backend-onboarding.md) |
| Private API and runtime | [0018 API](0018-private-api.md), [0019 attested service](0019-attested-service.md), [0020 Nitro](0020-nitro-runtime.md), [0021 confidential web](0021-confidential-web-api.md), [private read contract](private-read-contract.md) |
| Joined verification | [0022 offline acceptance](0022-offline-acceptance.md), [tracked acceptance](../implementation/UNIT-ACCEPTANCE.md) |
| BULK source interpretation | [0032 codecs](0032-bulk-codec-foundation.md), [0033 financial observations](0033-bulk-financial-observations.md) |
| Execution-unit state and API | [0034 boundary](0034-execution-unit-boundary.md), [0035 journal](0035-unit-journal-foundation.md), [0036 signatures](0036-unit-api-contract.md), [0037 history](0037-unit-api-history.md), [0038 admission](0038-unit-financial-admission.md), [0039 service](0039-unit-service.md) |
| Joined native deployment | [0040 execution](0040-unit-native-execution.md), [0041 custody/recovery](0041-unit-custody-recovery.md), [0042 assembly](0042-unit-deployment.md), [0043 workers](0043-unit-workers.md), [0044 quarantined paper](0044-paper-bootstrap.md) |

An older version is not an implicit migration target or fallback. Follow the
selected manifest/history contract and current source; preserve original
qualification receipts when code changes. [Status](../status.md) owns the current
evidence limits, not this index.
