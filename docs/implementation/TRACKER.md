# Implementation tracker

| Phase | Progress | Invariant / deliverable | PR / merge |
| --- | --- | --- | --- |
| [P00 — Foundation](PLAN.md#p00) | closed | Every phase has approved scope, testable completion criteria and a reproducible next-agent handoff. | [#23](https://github.com/arnabnandikgp/cinder/pull/23), merged `587a8ca` |
| [P01 — Workspace and harness](PLAN.md#p01) | closed | A pinned, offline-buildable workspace keeps the pure kernel separate from I/O and supplies deterministic fault-test ports. | [#24](https://github.com/arnabnandikgp/cinder/pull/24), merged `5aea02e` |
| [P02 — Financial types and identities](PLAN.md#p02) | closed | Exact units and canonical identities prevent precision loss, overflow and cross-domain replay. | [#25](https://github.com/arnabnandikgp/cinder/pull/25), merged `17a29e2` |
| [P03 — Unified ledger](PLAN.md#p03) | closed | One attributed ledger reconciles user/house claims and external exposure without hiding deficits or double-counting assets. | [#26](https://github.com/arnabnandikgp/cinder/pull/26), merged `3bd8c12` |
| [P04 — Funding, fees and reconciliation](PLAN.md#p04) | in progress | Funding and fees post once to the correct owner; unexplained differences remain visible and restrict dependent actions. | [#27](https://github.com/arnabnandikgp/cinder/pull/27), based on merged P03 |
| [P05 — Durable journal](PLAN.md#p05) | in progress | Postings, holds and consumed-event keys commit atomically and rebuild identically after a crash. | [#29](https://github.com/arnabnandikgp/cinder/pull/29), stacked above #27 |
| [P06 — Encrypted durability](PLAN.md#p06) | in progress | Private state survives qualified failures without plaintext leakage, silent rollback or revived stale writers. | [#30](https://github.com/arnabnandikgp/cinder/pull/30), stacked above #29 |
| [P07 — Order intents and reservations](PLAN.md#p07) | in progress | Durable reservations precede dispatch; acknowledgements and timeouts cannot fabricate fills or release unknown commitments. | [#31](https://github.com/arnabnandikgp/cinder/pull/31), stacked above #30 |
| [P08 — Funds and payouts](PLAN.md#p08) | in progress | Partial money movements reconcile by location; only final payment discharges a user claim, exactly once. | [#32](https://github.com/arnabnandikgp/cinder/pull/32), stacked above #31 |
| [P09 — Joined risk admission](PLAN.md#p09) | in progress | New actions pass user, pool, capital and location-liquidity checks across bounded pending outcomes. | [#33](https://github.com/arnabnandikgp/cinder/pull/33), stacked above #32 |
| [P10 — Protection claims](PLAN.md#p10) | in progress | House protection absorbs eligible losses once; repeated defaults and recoveries preserve ownership and unpaid claims. | [#34](https://github.com/arnabnandikgp/cinder/pull/34), stacked above #33 |
| [P11 — Liquidation and exceptions](PLAN.md#p11) | in progress | Bounded liquidation and late-close handling record residual risk without silently reversing customers or erasing losses. | [#35](https://github.com/arnabnandikgp/cinder/pull/35), stacked above #34 |
| [P12 — ADL and restoration](PLAN.md#p12) | in progress | Qualified ADL/restoration obeys RF1 basis economics and RF2 proportional quotas within funded execution limits. | [#36](https://github.com/arnabnandikgp/cinder/pull/36), stacked above #35 |
| [P13 — Pacifica observations](PLAN.md#p13) | in progress | Native observations normalize losslessly with explicit provenance, completeness limits and capability qualification. | Local implementation above #36 |
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

Worktree: `cinder-tee`. Trunk: `product/tee-v1` at merged P03 `3bd8c12`.
Current branch: `tee/p13-pacifica-observations`, created with `gh stack add` above
P12 #36 (`3bdef58`). User authorized continuous
stacked P06–P14 implementation while away, stopping before P15; no merges or new
external tests/deployments are authorized. Preserve unrelated `stays/` and ignored
`work/`. Keep phase tests/PRs/handoffs current; stop only for substantive decisions,
new permissions or a genuine unresolved blocker. P07 supplies durable order
lifecycle with trusted authentication/source ports, not live signing or trading.

Repository integration follow-up: the TEE trunk had no classic branch protection
or applicable rulesets at P00. CodeRabbit skipped P00's non-default-base review;
it later showed processing on P01, but no submitted review was present at merge.
Recheck current settings before claiming a review gate. ADR 0001 proposes scoped
TEE protections/review configuration and legacy-preview separation, with approval
for repository/deployment-setting changes. The existing Vercel check
reports deployment failure, but its build logs were not inspected; do not infer a
specific build error from that status alone. These integrations were left unchanged.

P03 is merged with user authorization. P04 is open as PR #27 on the TEE trunk;
P05 is open as PR #29 above #27 in GitHub stack #28, with its handoff below.
P04's full local checks passed again with the reviewed arithmetic fix. Review
and merge the remaining layers only with authorization. Native execution and
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

Review follow-up, 2026-09-30: user authorized addressing relevant CodeRabbit
findings and merging #26 before P06. The sole actionable finding
[4139343064](https://github.com/arnabnandikgp/cinder/pull/26#discussion_r4139343064)
is valid: artificial reversal sublegs rejected an exact whole fill. Reproduced
`Error::Inexact` with a failing 3/2 regression before changing production code.
Fix allocates the exact whole-fill signed value toward zero, retaining residue
in opening basis. Added mirrored/full-close, pooled bridge/replay and four-ratio
property regressions; clarified the integer equations in BASELINE/ADR 0003.
The generic docstring-percentage warning is not a missing-public-contract defect:
public Rust items already enforce `missing_docs = deny`; private/test helper
docstrings will not be bulk-added solely to satisfy that heuristic. No wider
Solana-program audit, venue deployment, protocol-policy change or telemetry action.
Review-skill guidance was used for reproduction, checked arithmetic and scope.
The fix passed the pinned macOS full offline runner: 42 Rust tests and two
compile-fail doctests in debug/release, 21 script tests, formatting and strict
Clippy. Three new regressions cover the fix; the direct reversal was explicitly
run red before the change. The full runner also passed clean staged export tree
`6c7dbf0a0bc1680c6dca7f609c2c9fbcd78115b3`, without ignored research or unrelated
files; only this documentation verification note follows that snapshot.
Closure: fixed head `e77ad2b63108848c30f1b7629c4accd4a3c22a68` passed hosted
Plan/handoff and Offline Rust workspace checks; CodeRabbit confirmed the fix and
resolved its thread. The legacy Vercel preview remained failed, not a TEE test
failure. No branch protection/ruleset or deployment settings were changed.
Merged #26 with `gh stack merge 26 --squash --yes`; only this layer was merged.

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
Implementation commit: `2dfa0e89cc9da6d348f240bf099e484a09363f94`; only this tracker
closeout follows it. Hosted CI and independent review are pending; inspect the
final PR head before merge, rather than carrying local results into a CI claim.
Unexpected: the first strict Clippy run flagged a test's modulo parity check; fixed
to the pinned toolchain idiom. Native cash and private realized cash intentionally
differ when external netting realizes PnL, so the structural check uses cash minus
basis rather than demanding identical cash/basis sums. Normalized causal evidence
remains a trusted caller obligation until P13, not inferred from passing aggregate checks.
Subsequent direction: user authorized P04 as a dependent stack layer while reviewing
P03. Its base remains exactly `e165eca78f34d696303084d2134110c686953186`; no P03 branch
rewrite or merge occurred.
Next: keep P04/P05 rebased and verified on the reviewed merged result. P06 remains
the next implementation phase, separate from this review fix.
PR: https://github.com/arnabnandikgp/cinder/pull/26
Merge: 3bd8c1287873db2d535f94b6e2e8ebb2a298458a.

## P04 — Funding, fees and reconciliation

Work: 2026-09-30 — added frozen funding inventory cuts, explicit missing rate/native
inputs, per-boundary recognition/settlement, signed rounding and named source
corrections on the same ledger. Actual execution fees/rebates follow stored owner
routes; inclusive native PnL normalizes once against native basis. Separate broker
fees move customer-to-house cash only. Unknown/unexplained economics remain in
suspense, not house profit. Ingestion retains normalized duplicates/rejections,
injected times and dispositions; named issues retain age. Complete component
checks and qualified marks gate dependent views without blocking actual losses.
Added a pure normalization port and offline fixture, ADR 0004, source fingerprints,
architecture updates and technical acceptance checks. No live adapter or fee policy.
Verification: pinned macOS arm64 Rust 1.97.1 / Node 24.21.0 `node scripts/check.mjs`
passed 21 script tests, 60 Rust tests and 2 compile-fail doctests in debug/release,
formatting, strict Clippy and build. P04 adds 19 economic/evidence tests and 2
normalization-port tests. The funding property includes 500 signed rounding cases;
replay preserves all observation dispositions and retained conflicts. Source
qualification is still a trusted input, not proved by those tests. The focused
`node scripts/check.mjs --properties` passed 9 tests. The full runner also passed
from clean staged export tree `0a2293f0981d8620bc637feab3a9407339635062`, excluding
`work/`, `stays/` and Git metadata. Implementation commit:
`a5f1755995110563ed0981cb41a3f36f84c0273b`; only this documentation handoff follows it.
No container, live venue, wallet, chain, AWS, machine proof or independent audit
was used/claimed. Hosted CI and PR review remain separate pending evidence;
inspect the final PR head before any authorized merge.
Unexpected: self-review tightened snapshot resolution to require named effects
to explain every originally known component exactly; a later matching snapshot
or unrelated transaction cannot clear it. Audit effects retain before/after books
so an extreme signed value need not form an overflowing delta merely for ingestion.
A compile-time module-visibility error and Clippy's manual-contains warning were
fixed locally before the successful full run. The first clean-export doc check
flagged an inline rather than line-start `Next:` handoff; corrected the formatting.
Skill guidance informed checked
math/adversarial testing without adding a runtime dependency or live test.
Review restack, 2026-09-30: `gh stack rebase` moved this branch onto merged P03
`3bd8c12` without conflicts. Full pinned offline checks passed with the three new
P03 regressions: 63 Rust tests plus 2 compile-fail doctests in debug/release,
21 script tests, formatting, strict Clippy and build. Tested tree
`2a129549cb5e043aad88f768fa531e1f7047e997` is unchanged across the squash-merge
restack; only this tracker closeout follows it. No containers or live calls.
Next: review #27's final CI/findings and obtain merge authorization; it now bases
on `product/tee-v1`, not legacy `main`. Keep P05 #29 stacked above this layer.
P05 must persist complete normalized observations (including
duplicates/rejections, times and dispositions) and outer raw evidence atomically
with postings/consumption. Do not replay only the accepted event list, bypass the
ingestion gate, or treat this in-memory evidence as crash durability.
PR: https://github.com/arnabnandikgp/cinder/pull/27
Merge: none.

## P05 — Durable journal

Review follow-up, 2026-10-01: CodeRabbit #4147277655 is valid. Added the
missing P13 acceptance contract for qualified per-input resolution and its stable
identity/effect/replay boundaries. No P05 admin reset was added. The plan validator
passes (25 phases, 14 documents); P13 owns implementation and offline regressions.

Work: 2026-09-30 — added cinder-journal with versioned lossless P03/P04 codecs,
complete raw/normalized evidence and deterministic receipts/state replay; atomic
SQLite CAS commits include postings, event consumption, shared holds and immutable
attempts. Exposure is recorded before one-shot local delivery; unknown commit
outcomes poison the handle and restart never automatically resends. Actual adverse
facts survive refused control proposals. Added migration, concurrency, page-full,
corruption and real killed-child tests; ADR 0005 and source fingerprints explain
the P06 privacy/freshness boundary. Dependency guard narrowly permits pinned
storage/hash packages; CI hydrates them before offline checks.
Verification: pinned macOS arm64 Rust 1.97.1 / Node 24.21.0 full offline runner
passed 23 script tests, 82 Rust tests and 2 compile-fail doctests in debug/release,
formatting, default/hook-enabled strict Clippy and build. P05 adds 22 Rust tests,
including two parent tests that kill ten actual child processes; the one ignored
worker is explicitly invoked by those parents. Ten finite property tests are
included in the full run. No container, live endpoint, wallet, chain, AWS,
independent review or machine proof is claimed. The same full runner passed from
clean staged export tree `84b578e8174b3fbe078a3f1a200028ecc59e2432`, excluding
`work/`, `stays/` and Git metadata; compiler/dependency caches were reused to
conserve disk space. Only documentation handoff clarifications follow that tree.
Implementation commit: `326e21cc17cdef03afafa8ab326e1f3130d0a3a9`.
The final PR-link handoff is documentation-only. Hosted Linux CI and external
reviews are separate evidence; inspect the final PR head before any authorized
merge. Existing Vercel/repository integration settings remain unchanged.
Unexpected: SQLite requires signed database integers; added checked conversions.
Tests caught fixture type mismatches; strict Clippy required naming the test-hook
type. Fixed before full verification. No production cipher is supplied: a mandatory
protection seam and opaque storage keep P06's security work explicit. Flat-only
capacity and one attempt per hold intentionally prevent inventing P07/P09 policy.
Review restack, 2026-09-30: inherited the merged P03 whole-fill fix and new
regressions via P04. Only tracker handoff text conflicted during rebase; retained
P03's merge evidence and P05's implementation record. Advanced financial-engine
revision to 2 so earlier journal semantics cannot be silently replayed under the
corrected arithmetic. Genesis and transaction tests reject revision 1; wire and
SQLite schema versions are unchanged. No production migration or P06 work is
implied. Full pinned local offline revalidation passed on tree
`8c8a13acd86edd58dca6f2f4a5d6efdf2f6ac7ab`: 85 Rust tests plus 2 compile-fail
doctests in debug/release, 23 script tests, formatting, both strict Clippy profiles
and build. The process-kill parents still exercise the intentionally ignored
worker. Only this verification note follows the tested tree; no additional
clean-export, container or live qualification is claimed for this follow-up.
Stack review follow-up: P13 comment 4141545400 also exposed a P05 commit/reload
byte-budget mismatch. Count exact protected bytes, including genesis, before
append/exposure; update the counter only after successful append and rebuild it
on reload. A full-budget/reopen/duplicate/exposure regression passes, along with
all 23 journal tests and strict workspace Clippy. Existing frame bytes/semantics
are unchanged. This is capacity containment, not automatic evidence pruning.
Next: review #27 and #29 bottom-up; merge only with user authorization.
If a parent changes, restack/retest descendants and update their evidence. Keep
the stack shallow before starting another layer. P06 must close the valid-history rollback counterexample
with AEAD, independent authenticated freshness and writer fencing; do not expose
private data or real native actions using the test-only protector.
PR: https://github.com/arnabnandikgp/cinder/pull/29
Merge: none.

## P06 — Encrypted durability

Work: 2026-09-30 — added versioned XChaCha20-Poly1305 records, random per-proposal
nonces, stream/context binding and explicit key generation. Added distinct
content-addressed replica ports/files, two-copy readback-before-witness acceptance,
bounded full encrypted replay snapshots, current-head/epoch checks and poisoned
unknown outcomes. Keys/private byte buffers are zeroized on ordinary drop; no
universal memory-erasure claim. M2/W06 cases promoted into tracked tests/ADR 0006.
Verification: pinned macOS arm64 Rust 1.97.1 / Node 24.21.0 full offline runner
passed: 96 Rust tests plus 2 compile-fail doctests in debug/release, 23 script
tests, formatting, default/all-features strict Clippy and build. Eleven P06 tests
include four actual child kills; the two ignored workers are invoked by their
parent tests (P05 and P06). The same full runner passed clean staged export tree
`0a802fbecba10d3c003f5a52f0828cb03105bb99`, with no `work/`, `stays/` or Git
metadata; only this handoff closeout follows it. Build caches were reused.
Unexpected: caught accidental replica aliasing in self-review and added an identity
guard/regression; distinct IDs still do not prove infrastructure independence.
Test-only SQLite witness required signed integer conversions; fixed locally.
Strict Clippy requested a fixture type alias; fixed before full verification.
Build/review skill guidance informed checked bounds, secrecy and adversarial tests;
no telemetry, subagent, deployment, live venue or container action.
Implementation commit: `9cfee2b`; PR #30 is ready above #29 in native stack #28.
P04/P05 hosted TEE checks passed on their reviewed restacked heads; the unrelated
legacy Vercel preview remains failed. P06 TEE CI passed at `bd3e9db`. Its subsequent
review identified two relevant edge cases: new acceptance now repairs/checks all
historical replica copies, and witness accept errors other than known stale CAS
rejection force uncertain-outcome poisoning. Read-only one-copy recovery remains
available. Three targeted regressions cover sequential replica loss, refused
historical repair, and a committed witness response mislabeled Busy. Local full
verification of this follow-up passed: 99 Rust tests in debug/release, two
compile-fail doctests, 23 repository tests, fmt, strict default/all-feature Clippy
and build. No new live or container tests. CodeRabbit comments 4139804671 and
4139804693 are addressed; updated hosted checks remain separate evidence.
Next: proceed to P07 under the user's continuous P06–P14 authorization, checking
P06 CI/reviews at phase boundaries. Do not start P15 or merge PRs.
PR: https://github.com/arnabnandikgp/cinder/pull/30
Merge: none.

## P07 — Order intents and reservations

Work: 2026-09-30 — integrated immutable GTC/ALO/IOC intents, authenticated-port
digest/epoch binding, shared commitments, canonical place/attributed-cancel actions,
partial fills, ACK/unknown/terminal history and adverse-fill containment in the
same journal. Original/classified facts replay without changed ownership. A late
terminal contradiction re-encumbers holds without erasing a valid customer fill.
ADR 0007 defines the promoted contract and remaining P09/P13/P14/P18 ports.
Verification: before restacking, full offline runner passed: 109 Rust tests in
debug/release, two compile-fail doctests, 23 repository tests, fmt, strict Clippy
and build. The 13-test order suite includes all 24 additive delivery/terminal
permutations with real SQLite reloads. First full run correctly caught an obsolete
wire-v1 golden header, updated explicitly for wire v2. Final restacked clean export
of commit `7f47ea3` passed the same full runner: 112 Rust tests in debug/release,
two compile-fail doctests and 23 repository tests, including the three added P06
regressions. Export `/private/tmp/cinder-p07-export.OA9rLI` contained no `work/`,
`stays/` or Git metadata. Only verification/publication notes follow that source.
No live venue calls or signatures.
Unexpected: internal binding time exposed an old-receive-time rejection: financial
projection now uses monotone commit time while retaining the original input time.
Journal wire is explicitly revision 2 / engine 3; updated old-revision fixtures.
P06 review fixes were made and tested in the owning layer, then P07 was restacked.
Next: P07 is ready as #31. Continue P08 under the user's P06–P14 authorization;
inspect hosted checks/reviews at phase boundaries. Do not merge or start P15.
PR: https://github.com/arnabnandikgp/cinder/pull/31
Merge: none.

## P08 — Funds and payouts

Review follow-up, 2026-10-01: reproduced the summary-only faulted-cancellation
concern with a failing regression before fixing it. Clean local cancellation now
rejects faulted operations, terminal evidence or actual bound receipts. All 18
funds groups pass; new coverage includes separate/same-transaction receipts,
duplicate commit, reopen/next advance and legitimate clean cancellation. No loss
allocation or custody authority changed; assembled-stack verification follows.

Work: 2026-09-30 — implemented kernel partial movements with transit/unpaired
contra-balances, paid counters, explicit fee ownership and house overpayment;
journal FIFO/partial consent, shared holds and terminal/source coverage. Initial
marked-collateral prerequisite is joined for open-position withdrawals;
P09 still owns pending-order outcomes, leverage selection and full capital envelope.
Verification: focused 17-group funds suite passes, including all 24 four-leg receipt
permutations with exact replay and every-prefix bridge checks. The initial full
runner passed 127 Rust tests (15 new groups), debug/release, two compile-fail
doctests, 23 repository tests, formatting, strict Clippy and build. Final full run
with two additional certificate/consent regressions also passed: 129 Rust tests
in each profile and the same other checks. Self-review
checked claim discharge, actual adverse outcomes, source-specific coverage,
hold consumption and replay commitments; no second financial ledger was added.
At this boundary P06 `a2babf0` and P07 `8fd9651` both passed hosted TEE CI and
CodeRabbit status; P07 had no inline findings. Status is not an independent audit.
Clean export of implementation commit `4d4e1cd` at
`/private/tmp/cinder-p08-export.cCxf6m` passed the same full pinned offline runner,
without `work/`, `stays/` or Git metadata. Only handoff/publication notes follow.
Unexpected: none requiring a new product decision. Numerical collateral profiles
are explicit synthetic test parameters, not live leverage/fee approvals.
Unexpected: the partial-receipt algebra needed an explicit unpaired contra-balance
to avoid double backing when arrival precedes debit. P08 introduced a shared marked
collateral prerequisite ahead of P09; pending orders still refuse this path until
P09 supplies their bounded outcomes. Wire 3 / engine 4 reject old semantics.
Next: P08 is ready as #32. Continue P09; inspect hosted checks/reviews at phase
boundaries. No new live tests, merge or P15 authority is implied.
PR: https://github.com/arnabnandikgp/cinder/pull/32
Merge: none.

## P09 — Joined risk admission

Review follow-up (2026-09-30): addressed CodeRabbit #4142787863/#4142787878.
Accept inserts the candidate order before its atomic reserve/capacity evaluation;
a rejected candidate still rolls back both. Risk reports build an active index once,
preserve retired identities, and budget index construction plus actual active
reservation/order/selection/scenario scans. Engine 11 rejects the former engine 5.
The 20 risk groups pass, including opposing-pending admission, 350 released orders,
reopen/idempotency and active-work-budget rejection; strict all-feature Clippy passes.
No policy/capital-limit change or evidence pruning.

Work: 2026-09-30 — implemented one joined derived risk report over ledger/shared holds,
independent pending quantity intervals and adverse execution cost, capped private
leverage, native margin, capital/concentration and location/deadline scenarios.
Verification: pinned full offline runner passed 147 Rust tests in debug/release,
two compile-fail doctests, 23 repository tests, formatting, strict Clippy and build.
The 18-group risk suite includes 540 partial/price/order-ordering cases, replay,
every-prefix capital, cap downgrade, actual shock, source-liquidity and shared
payout tests. Self-review added a final-control-cut check to prevent exposure
preceding a policy downgrade in the same atomic proposal, and evaluates each
execution inside grouped scenarios. A conservative nested-work bound was tightened
after that run; final clean-export verification follows below.
Clean export of final implementation `9917561` at
`/private/tmp/cinder-p09-export.qDNdq1` passed the same full runner (147 Rust tests
per profile, two doctests and 23 repository tests), including the tightened bound.
No `work/`, `stays/` or Git metadata was present; no Linux container or live call.
P08 head `0049c26` passed hosted TEE CI and CodeRabbit status with no inline findings.
Unexpected: no new product decision. The interval bound intentionally overestimates
some combinations; finite stress paths are not a probability or universal guarantee.
Unexpected: scenario execution/evidence errors fail qualification rather than
vanishing into an empty safe report; old wire 3 / engine 4 are explicitly rejected
for wire 4 / engine 5. No actual live policy or independent solvency proof claimed.
Next: P09 is ready as #33. Continue P10 protection claims; inspect hosted checks
and reviews at phase boundaries. Do not merge or enter P15.
PR: https://github.com/arnabnandikgp/cinder/pull/33
Merge: none.

## P10 — Protection claims

Work: 2026-09-30 — added repeated loss episodes, distinct deficit/remediation
recognition, designation/commitment/absorption, actual debt recovery and immutable
collection history in the same ledger. Joined explicit coverage/allocation ports
and global/per-customer lifetime caps to the journal and P09/P08 admission.
Verification: full pinned offline runner passed 163 Rust tests in debug/release,
two compile-fail doctests, 23 repository tests, formatting, strict Clippy and build.
The 16 protection groups include V06, repeated/partial recoveries, depletion,
remediation offsets, replay, authorization binding, pending commitments and an
opposing-account extraction counterexample. Initial full runs found stale test
constants for wire/engine revisions; updated the golden assertions and added old
wire 4 / engine 5 rejection cases, then reran successfully. Final clean export of
`b6c4787` at `/private/tmp/cinder-p10-export.wFxvkC` passed the same full runner,
including all 163 Rust tests per profile, without ignored research or Git metadata.
P09 head `85efc22` passed hosted TEE CI and CodeRabbit with no inline findings.
Unexpected: removed the study's single-episode shortcut. Self-review additionally
required quiet customer commitments before absorption. G02 still gates live
coverage/priority; caps bound rather than prove away coordinated extraction.
Ambiguous recovery during a reopened debt episode retains assets and a named
containment fault; no discretionary clear-flags command. No outside fund shares.
Review follow-up: verified CodeRabbit #4140440713. Protection stress transitions
were incorrectly restricted to Economic keys although the kernel requires Request
keys. Fixed per-variant key validation with deployment binding; added a scenario
that recognizes, commits and absorbs across every prefix without mutating live
claims. Also fixed the stale architecture status row (#4140440723). Full pinned
runner passed again: 164 Rust tests per profile, two doctests, 23 repository tests,
formatting, Clippy and build. This is a scenario integration fix, not a change to
the coverage promise or actual loss-posting semantics.
Next: P10 is ready as #34. Implement P11 bounded liquidation and
late-close house exceptions. Native house capital payout remains disabled pending
qualified custody integration; a designation release is not an external payout.
PR: https://github.com/arnabnandikgp/cinder/pull/34
Merge: none.

## P11 — Liquidation and exceptions

Work: 2026-09-30 — implemented bounded liquidation/close allocation and exceptional
house unwind, using the existing order status, shared holds and authoritative ledger.
Qualified policy bounds size, price, fees, time and funded support. Actual partial,
over-limit and late fills retain their economics without silently flipping users.
Verification: clean tracked export of `3ad800f` at
`/private/tmp/cinder-p11-export.OtlviF` passed the full pinned runner: 181 Rust
tests per debug/release profile, two doctests, 23 repository tests, formatting,
strict Clippy and build. The close suite has 17 groups including 432 long/short
allocation cases, local-abandonment contradictions and a joined close scenario.
P10 reviewed head `a1309af` also passed both hosted checks and CodeRabbit.
Unexpected: split-owner execution must partition the whole exact native fill value
before rounding, rather than require both artificial child notionals to be exact.
Self-review added explicit never-exposed abandonment and final-cut authority
revalidation; neither can release an exposed unknown action. P10 claim absorption
now requires full order completion, including these house-only reserved exits.
Native crisis recovery/incident resumption and live calibration remain gated; this
controller is deliberately limited to funded, source-qualified bounded exits.
Review follow-up at the P14 boundary: reproduced both new CodeRabbit findings
locally before the production fixes. Extracted full exposure qualification and
rechecked it at the final control cut, preserving scoped cancels. Moved depth
expiry out of the general risk gate into order admission/dispatch; all other
capacity/protection checks remain. Three regression groups cover both freeze
orderings, generic/no-risk and emergency dispatch, cancel cleanup, fresh-capacity
payouts/reservations/designation reductions and continued rejection of expired
orders/overdraws. All 20 close tests and the full pinned runner pass: 184 Rust tests
per debug/release profile, two doctests, 23 repository tests, strict Clippy,
formatting and build. Engine revision 8
explicitly rejects pre-correction history rather than reinterpreting it.

Next: P11 is ready as #35; continue P12
RF1/RF2 and bounded-resource quota scheduling. Do not merge or enter P15.
PR: [#35](https://github.com/arnabnandikgp/cinder/pull/35).
Merge: none.

## P12 — ADL and restoration

Review follow-up (2026-09-30): CodeRabbit #4142163504/#4142163522/#4142163544
addressed. Wrong-route restoration receipts fault the matched order and preserve
the unchanged rejected evidence instead of panicking/reclassifying. A single void
no longer vetoes other fixed awards at prepare/bind/expose; all-void incidents
still cannot authorize speculative house-only exposure. House-unwind labels do
not void private awards. Engine 16 rejects previous interpretation. Targeted close,
orders, restoration and risk suites pass 75 groups; strict all-feature Clippy passes.
Regressions cover ordinary/close misrouting, both void timings, unchanged quotas,
all-void containment, house versus private close, and durable replay.

Work: 2026-09-30 — implemented qualified ADL observation/declaration, fixed
proportional reduction and bounded exact-EDF quota compilation; joined funded
RF1 replacement, original basis/refunds, private-intent voids, source-time bounds,
house exceptions and terminal-history release to the existing ledger/order journal.
Verification: clean export of `da74dae` at
`/private/tmp/cinder-p12-export.IXE1mL` passed the full pinned runner: 204 Rust tests
per debug/release profile, two doctests, 23 repository tests, formatting, strict
Clippy and build (17 restoration groups total). P11 head `c702021` passed both hosted checks
and CodeRabbit.
Unexpected: the research per-lot tape cannot support arbitrary atomic quantities.
The compiler now skips only affine-certified identical EDF blocks, with explicit
owner/work/storage bounds and failure rather than an alternative allocation.
Tests cover trillion-lot accepted profiles and an unsupported irregular profile.
Source time, immutable cap/depth revisions and cumulative positive-cost limits
are explicit. No live ADL source or margin-isolated subaccount claim was added.
Next: implement P13 sanitized Pacifica observations using current primary docs.
Do not enter P15. P12 is ready for review, not merged or live-qualified.
PR: [#36](https://github.com/arnabnandikgp/cinder/pull/36), above #35.
Merge: none.

## P13 — Pacifica observations

Work: 2026-09-30 — added the Pacifica crate, exact typed native schemas, immutable
qualification profile, shared REST/WS identities, diagnostic views, bounded cursor
replay and named containment. Raw provenance and economic inputs share one encrypted
journal transaction. Promoted VM-01–VM-09 and rechecked current primary documentation.
Verification: full pinned offline runner passed: 216 Rust tests per debug/release
profile (12 adapter groups), two doctests, 23 repository tests, strict Clippy,
formatting and build. Clean export `e2a9eb7` at
`/private/tmp/cinder-p13-export.HEhqQa` passed the same runner. No live calls or wallets used.
Unexpected: retained response attachments required an explicit journal wire/engine
revision (8/9 initially, engine 10 after the P11 review correction). Public
settlement/funding/completeness semantics remain insufficient;
dependent capabilities stay disabled. Initial fixture missed the required location
hold; corrected the fixture, not the production admission rule. One old engine
golden assertion also needed the explicit version bump. P12's two hosted checks
and CodeRabbit are green; no inline findings were present at this phase boundary.
Next: complete offline/clean verification and publish P13, then P14 bounded signing
and shared API-credit admission. Stop before P15; preserve all open G01/G02 gates.
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
