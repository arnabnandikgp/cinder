# Implementation tracker

| Phase | Progress | Invariant / deliverable | PR / merge |
| --- | --- | --- | --- |
| [P00 — Foundation](PLAN.md#p00) | closed | Every phase has approved scope, testable completion criteria and a reproducible next-agent handoff. | [#23](https://github.com/arnabnandikgp/cinder/pull/23), merged `587a8ca` |
| [P01 — Workspace and harness](PLAN.md#p01) | closed | A pinned, offline-buildable workspace keeps the pure kernel separate from I/O and supplies deterministic fault-test ports. | [#24](https://github.com/arnabnandikgp/cinder/pull/24), merged `5aea02e` |
| [P02 — Financial types and identities](PLAN.md#p02) | closed | Exact units and canonical identities prevent precision loss, overflow and cross-domain replay. | [#25](https://github.com/arnabnandikgp/cinder/pull/25), merged `17a29e2` |
| [P03 — Unified ledger](PLAN.md#p03) | in progress | One attributed ledger reconciles user/house claims and external exposure without hiding deficits or double-counting assets. | Local verification |
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

Worktree: `cinder-tee`. Trunk: `product/tee-v1` at merged P02 `17a29e2`.
Current branch: `tee/p03-unified-ledger`. Preserve unrelated `stays/` and local
ignored `work/`. P03 implements the pure in-memory ledger/position foundation and
the requested [full architecture](../architecture.md). No durable store, program,
service, live native adapter or Nitro runtime has been implemented.

Repository integration follow-up: the TEE trunk had no classic branch protection
or applicable rulesets at P00. CodeRabbit skipped P00's non-default-base review;
it later showed processing on P01, but no submitted review was present at merge.
Recheck current settings before claiming a review gate. ADR 0001 proposes scoped
TEE protections/review configuration and legacy-preview separation, with approval
for repository/deployment-setting changes. The existing Vercel check
reports deployment failure, but its build logs were not inspected; do not infer a
specific build error from that status alone. These integrations were left unchanged.

P02 is merged with user authorization; P03 passes local verification and awaits PR publication.
Funding/fees and the durable journal follow. Native execution and customer-funds deployment are
not implied by this foundation. P19/P21/P22 support
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
Subsequent head `cb986b1c270142f7a3c03deeeb0051fc156c1ecf` passed documentation CI;
final tracker-column head `e3eca304123ded0fcaca7e8471818798f8b34152` passed 12/12
local script tests and documentation CI before merge. Local Node 26 is not
asserted to be the identical environment. Historical M1 81/81 and M2 23/23
were not rerun or claimed as production evidence. Validation checks structure,
not the truth of a claimed merge, audit or financial proof.

Unexpected: clean TEE branch had only README/.gitignore and no tracked research.
Added a sanitized baseline and initial cases so P01 is not dependent on ignored
files. Native stack extension is installed; initialized a local stack with explicit
TEE trunk. Published only the TEE trunk and P00 branch; legacy main is unchanged.
GitHub reported no TEE trunk protection/rulesets. CodeRabbit success means
**review skipped**, not approval. Vercel failure remains the integration follow-up
above, not a reason to import a website into this worktree.

Next: complete P01 on the merged foundation. User approved P00 and explicitly
authorized merge; no submitted CodeRabbit review was present at closeout. The
existing Vercel failure and hosted settings were not changed or represented as green.

PR: https://github.com/arnabnandikgp/cinder/pull/23
Merge: 587a8ca801ccc656c41ee352213b32f6d8b6abf8.

## P01 — Workspace and harness

Work: 2026-09-30 — implemented the three-crate Rust workspace, exact local lockfile,
Rust/Node pins, deterministic clock/journal/venue doubles, dependency guard,
common offline runner, Linux CI and ADR 0001/development instructions. The kernel
contains only a transition boundary, not financial behavior. The required
repository integration changes are proposed in ADR 0001; hosted settings remain
unchanged pending authorization.
Verification: 2026-09-30 — `node scripts/check.mjs` passed from the worktree and
a clean staged-file export on macOS arm64, using Rust 1.97.1 / Node 24.21.0.
The same export passed on Linux 6.18.35 aarch64 using Apple container 1.4.1,
with network disabled (only loopback), source/tools read-only, and temporary
container-local build output. Each full run passed 19 script tests, 7 Rust tests
in both debug/release, formatting, Clippy and build. Toy properties cover all
65,536 u8 state/event pairs and 243 five-step fault schedules. The tested export
tree was `affa243f453afbf877b513ddce72006f920855ac`; only documentation closeout
follows that snapshot. The export excluded `work/`, `stays/` and Git metadata.
Linux base: `rust:1.97.1-slim-bookworm`, OCI index
`sha256:2775a09d208ff0d7c1f50490c45b62db929e87ba1dcbc3f2132ac71a704bcdd3`.
Node Linux archive SHA-256:
`6ad1325edbdb5649c379b75a237147a666c95d4f9ae8d340fef2d1575d289ad2`.
Clippy/rustfmt were prehydrated from verified official component archives; no
network access was needed during the successful run. Linux x86-64 CI is separate
evidence; no production financial, SBF or Nitro qualification is claimed.
Commit `2bd82b93ed1b166f059ee8a399a3c9b67d1945e2` passed hosted Ubuntu x86-64
[workspace CI](https://github.com/arnabnandikgp/cinder/actions/runs/36633338076)
and [documentation CI](https://github.com/arnabnandikgp/cinder/actions/runs/36633337972).
Documentation closeout passed all 19 local script tests and the plan validator.
At that check, CodeRabbit was pending and the existing Vercel preview failed;
neither is represented as approval/success. Recheck the final head before merge.
Unexpected: the first compile caught missing crate-level docs in a test, fixed
before the successful pass. Apple container had no Linux kernel/image installed;
prepared the recommended local kernel and official Rust image. Downloaded
checksum-verified Node binaries into a task directory, leaving global Node intact.
The slim Rust image lacked rustfmt/Clippy; its builder could not resolve the
download host. A subsequent custom image unpack hit host disk exhaustion and
could not bootstrap even without a source mount. Removed only this session's
disposable builder/cache and failed custom image; successfully ran against the
original official image with cached components instead. These were local tooling
failures, not test passes; the removed artifacts are reproducible, not user data.
Follow-up: user approved local macOS checks plus hosted Linux CI for routine PRs,
with two evidence-based fix/push attempts before container reproduction. This is
recorded in AGENTS/development docs. Final head `87c28c4812e182d750c2ac092899d49fd6c96e38`
passed both Cinder CI jobs before the explicitly authorized squash merge. No
submitted review or inline findings were present; CodeRabbit was not an approval.
The legacy Vercel failure and hosted integration settings were left unchanged.
Next: implement P02; hosted integration changes remain separately authorized.
PR: https://github.com/arnabnandikgp/cinder/pull/24
Merge: 5aea02ec13cd5bd8834a30ee2aa260efe3d4369d.

## P02 — Financial types and identities

Work: 2026-09-30 — implemented unit-tagged quote/basis/lot/tick types, lossless
decimal/grid parsing, full-range checked signed rounding and basis splitting.
Added distinct network/deployment/account/request/attempt/economic IDs, scoped
canonical frames, versioned payload framing and pure duplicate/conflict comparison.
Promoted V02 and independent BigInt vectors; ADR 0002 records wire layout, source
hashes and bounds. Kernel remains dependency-free/no_std with bounded alloc use;
guard now checks submodules, and the focused property runner includes kernel tests.
Verification: pinned macOS Rust 1.97.1 / Node 24.21.0 `node scripts/check.mjs`
passed 21 script tests, 25 Rust tests and 2 compile-fail doctests in debug/release,
formatting, Clippy and build. New cases cover 69,632 small arithmetic triples in
four rounding modes, 64 full-width BigInt vectors, 10,000 byte mutations, exact
canonical golden layouts and truncation boundaries. Focused self-review checked signed
minimum magnitudes, remainder ownership, framing bounds and scope completeness.
The full runner also passed from clean staged export tree
`900caec255f4dffaf2cb238d609372c01ed298f5`, without `work/`, `stays/` or Git metadata;
only this documentation closeout follows that snapshot. The focused property
command passed 6 tests. Implementation commit:
`c4aa534c2dceba6c313d350d2d3a2a6b4fb27d31`. Only tracker closeout follows it.
Hosted CI and independent PR review remain pending; inspect the final PR head
before merge. The first documentation CI passed; existing Vercel preview failed.
No container, live venue, wallet, chain, AWS, formal proof or complete ledger
qualification claimed.
Unexpected: full-width differential testing caught inconsistent error precedence
when Exact division was both fractional and out of range; exactness now rejects
first in every case. No incorrect numeric result was accepted. Checked i128
product-first division would unnecessarily reject representable partial basis;
used quotient/remainder decomposition with an explicit range argument instead.
Closeout: final head `84d63932b2375487851bce35b507a508b4e4bcfa` passed both Cinder
CI jobs. CodeRabbit reported success but had no submitted review or inline findings;
this is not formal approval. Legacy Vercel preview remained failed and was not
changed. User explicitly authorized the squash merge; local TEE trunk was fast-forwarded.
Next: implement P03 on merged P02; V02's test-only driver is replaced by production accounting there.
PR: https://github.com/arnabnandikgp/cinder/pull/25
Merge: 17a29e2878bbeeea272aeb61a2051de12268754e.

## P03 — Unified ledger

Work: 2026-09-30 — added production signed-basis fill transitions, explicit rational
lot/tick conversion and one scoped customer/house/suspense/native ledger. Physical
vault, native signed cash and full-transfer receivables reconcile without duplicating
assets. Stored execution routes bind fill ownership; exact normalized duplicates
are no-ops and changed payloads conflict. Read-only diagnostics expose deficits
even for net-flat external positions. Added the requested detailed architecture,
ADR 0003 hand derivations/limitations and tracked research promotion.
Verification: pinned macOS arm64 Rust 1.97.1 / Node 24.21.0 `node scripts/check.mjs`
passed 21 script tests, 39 Rust tests and 2 compile-fail doctests in debug/release,
formatting, strict Clippy and build. This includes 14 new ledger tests, 25,020
position/fill combinations at two marks, and 12 seeded histories of 128 fills at
three marks against an independent cash-flow oracle, followed by exact replay.
V02 now uses production accounting rather than a test-only transition. Focused
self-review covered ownership routing, native/private basis mismatch, physical
versus signed cash, second-leg failure atomicity, replay conflicts, source scopes,
suspense and i64 minimum/checked-overflow behavior. No container/live venue/wallet/
chain/AWS run; no machine proof or independent audit. The full runner also passed
from clean staged export tree `3b4e21e374c02abf72327e13ac98d3915352fc6d`, excluding
`work/`, `stays/` and Git metadata; only documentation closeout follows that snapshot.
`node scripts/check.mjs --properties` passed all 8 discovered property tests.
Unexpected: the first strict Clippy run flagged a test's modulo parity check; fixed
to the pinned toolchain idiom. Native cash and private realized cash intentionally
differ when external netting realizes PnL, so the structural check uses cash minus
basis rather than demanding identical cash/basis sums. Normalized causal evidence
remains a trusted caller obligation until P13, not inferred from passing aggregate checks.
Next: complete pinned offline verification, focused review and publish only P03
against `product/tee-v1`; do not merge it or start P04 without subsequent authorization.
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
