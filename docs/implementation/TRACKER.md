# Implementation tracker

| Phase | Progress | Invariant / deliverable | PR / merge |
| --- | --- | --- | --- |
| [P00 — Foundation](PLAN.md#p00) | in progress | Every phase has approved scope, testable completion criteria and a reproducible next-agent handoff. | [#23](https://github.com/arnabnandikgp/cinder/pull/23), awaiting merge |
| [P01 — Workspace and harness](PLAN.md#p01) | open | A pinned, offline-buildable workspace keeps the pure kernel separate from I/O and supplies deterministic fault-test ports. | — |
| [P02 — Financial types and identities](PLAN.md#p02) | open | Exact units and canonical identities prevent precision loss, overflow and cross-domain replay. | — |
| [P03 — Unified ledger](PLAN.md#p03) | open | One attributed ledger reconciles user/house claims and external exposure without hiding deficits or double-counting assets. | — |
| [P04 — Funding, fees and reconciliation](PLAN.md#p04) | open | Funding and fees post once to the correct owner; unexplained differences remain visible and restrict dependent actions. | — |
| [P05 — Durable journal](PLAN.md#p05) | open | Postings, holds and consumed-event keys commit atomically and rebuild identically after a crash. | — |
| [P06 — Encrypted durability](PLAN.md#p06) | open | Private state survives qualified failures without plaintext leakage, silent rollback or revived stale writers. | — |
| [P07 — Order intents and reservations](PLAN.md#p07) | open | Durable reservations precede dispatch; acknowledgements and timeouts cannot fabricate fills or release unknown commitments. | — |
| [P08 — Funds and payouts](PLAN.md#p08) | open | Partial money movements reconcile by location; only final payment discharges a user claim, exactly once. | — |
| [P09 — Joined risk admission](PLAN.md#p09) | open | New actions pass user, pool, capital and location-liquidity checks across bounded pending outcomes. | — |
| [P10 — Protection claims](PLAN.md#p10) | open | House protection absorbs eligible losses once; repeated defaults and recoveries preserve ownership and unpaid claims. | — |
| [P11 — Liquidation and exceptions](PLAN.md#p11) | open | Bounded liquidation and late-close handling record residual risk without silently reversing customers or erasing losses. | — |
| [P12 — ADL and restoration](PLAN.md#p12) | open | Qualified ADL/restoration obeys RF1 basis economics and RF2 proportional quotas within funded execution limits. | — |
| [P13 — Pacifica observations](PLAN.md#p13) | open | Native observations normalize losslessly with explicit provenance, completeness limits and capability qualification. | — |
| [P14 — Pacifica execution](PLAN.md#p14) | open | Only authorized, reserved actions are signed; unknown outcomes reconcile before retry and cleanup capacity stays available. | — |
| [P15 — Solana vault](PLAN.md#p15) | open | Vault movements enforce asset, authority, recipient, epoch and atomic payout-counter boundaries. | — |
| [P16 — Funding coordinator](PLAN.md#p16) | open | Collateral completes the supported vault/venue round trip without duplicate funding, withdrawal or customer credit. | — |
| [P17 — Recovery program](PLAN.md#p17) | open | Authorized, funded final claims pay once; stale roots, wrong recipients and prior ordinary payouts cannot replay. | — |
| [P18 — Private API and SDK](PLAN.md#p18) | open | Customers access only their own scoped operations and views through a secure SDK, never raw pooled-account authority. | — |
| [P19 — Attested transport](PLAN.md#p19) | open | Clients bind approved code, fresh attestation and session keys before sending private data through an untrusted relay. | — |
| [P20 — Nitro qualification](PLAN.md#p20) | open | Actual enclave, key-release, egress and storage/fencing behavior is qualified; mocks cannot stand in for hardware evidence. | — |
| [P21 — Recovery integration](PLAN.md#p21) | open | Fenced and reconciled funds back final claims; users obtain claim packages without the ordinary trading API. | — |
| [P22 — Offline adversarial acceptance](PLAN.md#p22) | open | Joined production components preserve accounting and authority under reproducible faults, races and adverse financial paths. | — |
| [P23 — Live integration qualification](PLAN.md#p23) | open | Approved bounded live runs verify the actual implementation and fully reconcile test positions, orders and funds. | — |
| [P24 — Release safety case](PLAN.md#p24) | open | Reviewed evidence, calibrated policy and explicit governance support an honest release decision, not automatic deployment. | — |

Updated 2026-09-30. Only progress values: `open`, `in progress`, `closed`. A blocked
phase stays `in progress` with the reason below. `closed` requires completed PLAN
criteria, recorded tests/review and actual merge; a green/unmerged PR is not closed.
Dependencies and detailed acceptance criteria live in [PLAN](PLAN.md).

## Current handoff

Worktree: `cinder-tee`. Trunk: `product/tee-v1` at initial clean baseline `0376eb5`.
Current branch: `tee/p00-implementation-foundation`. Preserve unrelated `stays/`
and local ignored `work/`. No production program/service/adapter has been built.

Repository integration follow-up before routine stack merges: the new TEE trunk
has no classic branch protection or applicable rulesets; CodeRabbit skipped review
because non-default-base reviews are disabled; the existing Vercel preview failed.
Propose scoped TEE protections/review configuration and legacy-preview separation
in P01, with approval for repository/deployment-setting changes. The Vercel check
reports deployment failure, but its build logs were not inspected; do not infer a
specific build error from that status alone. These integrations were left unchanged.

Next implementation is P01, then exact types/ledger/journal. Native execution and
customer-funds deployment are not implied by this foundation. P19/P21/P22 support
offline development independently of actual P20 AWS availability; P23 joins both.

Each session appends dated work/evidence/deviations and replaces the next action
with a concrete handoff. Do not erase historical failed/skipped checks or carry
forward an obsolete green result as verification of a new commit.

## P00 — Foundation

Work: 2026-09-30 — extracted approved architecture, financial/RF terms and open
gates into BASELINE; created PLAN, this tracker, EVIDENCE, portable CONFORMANCE
cases, root contributor guide and README entry points. Added a documentation-only
validator, self-tests and CI. No original research or legacy files were changed.
Follow-up: added the requested invariant/deliverable column for every phase and
made its presence/non-empty content part of the documentation checks.

Verification: 2026-09-30, macOS arm64 / Node v26.8.2: `node
scripts/check-implementation-plan.mjs` passed (25 phases, 7 documents); `node
--test scripts/check-implementation-plan.test.mjs` passed 11/11 tests. Both also
passed in an exact staged-file export without `work/` or `stays/`.
`git diff --cached --check` passed. Financial reference rerun passed 233 groups /
11 suites offline. Commit `b8e1c1ed08e7956210c59b1dd362a2e9bc1ebf8b` passed
[Linux/Node 24 documentation CI](https://github.com/arnabnandikgp/cinder/actions/runs/36626591840).
This handoff/acceptance follow-up needs its own CI run; do not transfer the earlier
result to a later commit. Local Node 26 is not asserted to be the identical
environment. Historical M1 81/81 and M2 23/23
were not rerun or claimed as production evidence. Validation checks structure,
not the truth of a claimed merge, audit or financial proof.

Unexpected: clean TEE branch had only README/.gitignore and no tracked research.
Added a sanitized baseline and initial cases so P01 is not dependent on ignored
files. Native stack extension is installed; initialized a local stack with explicit
TEE trunk. Published only the TEE trunk and P00 branch; legacy main is unchanged.
GitHub reported no TEE trunk protection/rulesets. CodeRabbit success means
**review skipped**, not approval. Vercel failure remains the integration follow-up
above, not a reason to import a website into this worktree.

Next: review PR #23 and confirm checks on its final head; then begin P01 with the
toolchain/layout ADR and the minimal offline workspace on the reviewed/tested
foundation. Keep P00 in progress until merge; do not merge or deploy without permission.

PR: https://github.com/arnabnandikgp/cinder/pull/23
Merge: pending.

## P01 — Workspace and harness

Work: not started.
Verification: not run; acceptance in PLAN P01.
Unexpected: none yet.
Next: read P01/BASELINE, record toolchain/layout ADR, then add the minimal offline workspace and real check commands.
PR: none.
Merge: none.

## P02 — Financial types and identities

Work: not started.
Verification: not run; acceptance in PLAN P02.
Unexpected: none yet.
Next: after P01, implement canonical units/identities and port V02 precision/overflow vectors.
PR: none.
Merge: none.

## P03 — Unified ledger

Work: not started.
Verification: not run; acceptance in PLAN P03.
Unexpected: none yet.
Next: join customer/house/suspense and asset/basis projections; port V01–V03 before optimizing.
PR: none.
Merge: none.

## P04 — Funding, fees and reconciliation

Work: not started.
Verification: not run; acceptance in PLAN P04.
Unexpected: none yet.
Next: port gross/net funding and inclusive-fee cases, then add named mismatch containment.
PR: none.
Merge: none.

## P05 — Durable journal

Work: not started.
Verification: not run; acceptance in PLAN P05.
Unexpected: none yet.
Next: implement atomic consumed-key/posting/hold commits and process-kill replay tests.
PR: none.
Merge: none.

## P06 — Encrypted durability

Work: not started.
Verification: not run; acceptance in PLAN P06.
Unexpected: none yet.
Next: promote M2 corruption/rollback cases and define the authenticated head/writer-epoch interface.
PR: none.
Merge: none.

## P07 — Order intents and reservations

Work: not started.
Verification: not run; acceptance in PLAN P07.
Unexpected: none yet.
Next: add durable intent/attempt lifecycle and shared hold semantics, then cancel/unknown races.
PR: none.
Merge: none.

## P08 — Funds and payouts

Work: not started.
Verification: not run; acceptance in PLAN P08.
Unexpected: none yet.
Next: join partial movement receipts to live position state and reserve total payout debit atomically.
PR: none.
Merge: none.

## P09 — Joined risk admission

Work: not started.
Verification: not run; acceptance in PLAN P09.
Unexpected: none yet.
Next: bound pending outcomes under one state cut; implement synthetic policy profiles without enabling live risk.
PR: none.
Merge: none.

## P10 — Protection claims

Work: not started.
Verification: not run; acceptance in PLAN P10.
Unexpected: none yet.
Next: port V06, remove single-default-episode assumptions and test repeated loss/recovery ownership.
PR: none.
Merge: none.

## P11 — Liquidation and exceptions

Work: not started.
Verification: not run; acceptance in PLAN P11.
Unexpected: none yet.
Next: implement bounded liquidation/unwind ports and late-close exception ownership before native dispatch.
PR: none.
Merge: none.

## P12 — ADL and restoration

Work: not started.
Verification: not run; acceptance in PLAN P12.
Unexpected: none yet.
Next: promote RF2 compiler vectors, implement bounded-resource schedule and join actual RF1 restoration postings.
PR: none.
Merge: none.

## P13 — Pacifica observations

Work: not started.
Verification: not run; acceptance in PLAN P13.
Unexpected: none yet.
Next: promote sanitized VM-01–VM-09 mapping, recheck current docs and implement codecs/replay with fake transport.
PR: none.
Merge: none.

## P14 — Pacifica execution

Work: not started.
Verification: not run; acceptance in PLAN P14.
Unexpected: none yet.
Next: bind signed attempts to durable admission and test unknown ACK/429/cancel against a fake server.
PR: none.
Merge: none.

## P15 — Solana vault

Work: not started.
Verification: not run; acceptance in PLAN P15.
Unexpected: none yet.
Next: promote M1 authority invariants; document program topology and build local SBF vault/payout tests.
PR: none.
Merge: none.

## P16 — Funding coordinator

Work: not started.
Verification: not run; acceptance in PLAN P16.
Unexpected: none yet.
Next: port controller expose-before-send/crash scenarios onto the common journal and actual custody interfaces.
PR: none.
Merge: none.

## P17 — Recovery program

Work: not started.
Verification: not run; acceptance in PLAN P17.
Unexpected: none yet.
Next: implement final domain-bound claims and shared counters; port M2 wrong-domain/replay/atomic-failure tests.
PR: none.
Merge: none.

## P18 — Private API and SDK

Work: not started.
Verification: not run; acceptance in PLAN P18.
Unexpected: none yet.
Next: define owner/agent request grants and operation-state SDK responses with local confidential transport ports.
PR: none.
Merge: none.

## P19 — Attested transport

Work: not started.
Verification: not run; acceptance in PLAN P19.
Unexpected: none yet.
Next: review protocol/library choices and bind attestation freshness/release/session keys; test hostile relay and client release risks.
PR: none.
Merge: none.

## P20 — Nitro qualification

Work: not started.
Verification: not run; acceptance in PLAN P20.
Unexpected: none yet.
Next: build deterministic packaging locally, then request any missing G03/G05 authority before real hardware/KMS/witness qualification.
PR: none.
Merge: none.

## P21 — Recovery integration

Work: not started.
Verification: not run; acceptance in PLAN P21.
Unexpected: none yet.
Next: join custody unwind, fencing, final claims and independent kit delivery offline; use explicit hardware/witness doubles.
PR: none.
Merge: none.

## P22 — Offline adversarial acceptance

Work: not started.
Verification: not run; acceptance in PLAN P22.
Unexpected: none yet.
Next: run production components through recorded adversarial paths and persist reproducible seeds/counterexamples.
PR: none.
Merge: none.

## P23 — Live integration qualification

Work: not started.
Verification: not run; acceptance in PLAN P23.
Unexpected: none yet.
Next: prepare the bounded G05 manifest after P20/P22; qualify actual implementation on approved environments, with cleanup reconciliation.
PR: none.
Merge: none.

## P24 — Release safety case

Work: not started.
Verification: not run; acceptance in PLAN P24.
Unexpected: none yet.
Next: assemble evidence/independent reviews and bring G01–G04 production policy and assurance decisions to the user before release authorization.
PR: none.
Merge: none.
