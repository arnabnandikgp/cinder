# Implementation tracker

| Phase | Progress | Invariant / deliverable | PR / merge |
| --- | --- | --- | --- |
| [P00 — Foundation](PLAN.md#p00) | closed | Every phase has approved scope, testable completion criteria and a reproducible next-agent handoff. | [#23](https://github.com/arnabnandikgp/cinder/pull/23), merged `587a8ca` |
| [P01 — Workspace and harness](PLAN.md#p01) | closed | A pinned, offline-buildable workspace keeps the pure kernel separate from I/O and supplies deterministic fault-test ports. | [#24](https://github.com/arnabnandikgp/cinder/pull/24), merged `5aea02e` |
| [P02 — Financial types and identities](PLAN.md#p02) | closed | Exact units and canonical identities prevent precision loss, overflow and cross-domain replay. | [#25](https://github.com/arnabnandikgp/cinder/pull/25), merged `17a29e2` |
| [P03 — Unified ledger](PLAN.md#p03) | closed | One attributed ledger reconciles user/house claims and external exposure without hiding deficits or double-counting assets. | [#26](https://github.com/arnabnandikgp/cinder/pull/26), merged `3bd8c12` |
| [P04 — Funding, fees and reconciliation](PLAN.md#p04) | closed | Funding and fees post once to the correct owner; unexplained differences remain visible and restrict dependent actions. | [#27](https://github.com/arnabnandikgp/cinder/pull/27), merged `95fe3b7` |
| [P05 — Durable journal](PLAN.md#p05) | closed | Postings, holds and consumed-event keys commit atomically and rebuild identically after a crash. | [#29](https://github.com/arnabnandikgp/cinder/pull/29), merged `95fe3b7` |
| [P06 — Encrypted durability](PLAN.md#p06) | closed | Private state survives qualified failures without plaintext leakage, silent rollback or revived stale writers. | [#30](https://github.com/arnabnandikgp/cinder/pull/30), merged `95fe3b7` |
| [P07 — Order intents and reservations](PLAN.md#p07) | closed | Durable reservations precede dispatch; acknowledgements and timeouts cannot fabricate fills or release unknown commitments. | [#31](https://github.com/arnabnandikgp/cinder/pull/31), merged `95fe3b7` |
| [P08 — Funds and payouts](PLAN.md#p08) | closed | Partial money movements reconcile by location; only final payment discharges a user claim, exactly once. | [#32](https://github.com/arnabnandikgp/cinder/pull/32), merged `95fe3b7` |
| [P09 — Joined risk admission](PLAN.md#p09) | closed | New actions pass user, pool, capital and location-liquidity checks across bounded pending outcomes. | [#33](https://github.com/arnabnandikgp/cinder/pull/33), merged `95fe3b7` |
| [P10 — Protection claims](PLAN.md#p10) | closed | House protection absorbs eligible losses once; repeated defaults and recoveries preserve ownership and unpaid claims. | [#34](https://github.com/arnabnandikgp/cinder/pull/34), merged `95fe3b7` |
| [P11 — Liquidation and exceptions](PLAN.md#p11) | closed | Bounded liquidation and late-close handling record residual risk without silently reversing customers or erasing losses. | [#35](https://github.com/arnabnandikgp/cinder/pull/35), merged `95fe3b7` |
| [P12 — ADL and restoration](PLAN.md#p12) | closed | Qualified ADL/restoration obeys RF1 basis economics and RF2 proportional quotas within funded execution limits. | [#36](https://github.com/arnabnandikgp/cinder/pull/36), merged `95fe3b7` |
| [P13 — Pacifica observations](PLAN.md#p13) | closed | Native observations normalize losslessly with explicit provenance, completeness limits and capability qualification. | [#37](https://github.com/arnabnandikgp/cinder/pull/37), merged `95fe3b7` |
| [P14 — Pacifica execution](PLAN.md#p14) | closed | Only authorized, reserved actions are signed; unknown outcomes reconcile before retry and cleanup capacity stays available. | [#38](https://github.com/arnabnandikgp/cinder/pull/38), merged `95fe3b7` |
| [P15 — Solana vault](PLAN.md#p15) | closed | Vault movements enforce asset, authority, recipient, epoch and atomic payout-counter boundaries. | [#39](https://github.com/arnabnandikgp/cinder/pull/39), merged `95fe3b7` |
| [P16 — Funding coordinator](PLAN.md#p16) | closed | Collateral completes the supported vault/venue round trip without duplicate funding, withdrawal or customer credit. | [#40](https://github.com/arnabnandikgp/cinder/pull/40), merged `95fe3b7` |
| [P17 — Recovery program](PLAN.md#p17) | closed | Authorized, funded final claims pay once; stale roots, wrong recipients and prior ordinary payouts cannot replay. | [#41](https://github.com/arnabnandikgp/cinder/pull/41), merged `1f750b0` |
| [P18 — Private API and SDK](PLAN.md#p18) | closed | Customers access only their own scoped operations and views through a secure SDK, never raw pooled-account authority. | [#42](https://github.com/arnabnandikgp/cinder/pull/42), merged `1f750b0` |
| [P19 — Attested transport and runnable service](PLAN.md#p19) | closed | A runnable relay/private-service slice carries authenticated SDK operations through verified encrypted sessions; parent sees no private plaintext. | [#44](https://github.com/arnabnandikgp/cinder/pull/44), merged `dc5f01d` |
| [P20 — Nitro application qualification](PLAN.md#p20) | closed | The assembled confidential application runs on actual Nitro with qualified key release, egress, restore and fencing; mocks are not hardware evidence. | [#45](https://github.com/arnabnandikgp/cinder/pull/45), merged `76f3295` |
| [P21A — HTTP and WebSocket API](PLAN.md#p21a) | in progress | Browser/Node HTTP and WebSocket commands and private updates preserve attested confidentiality, owner permissions and one authoritative financial journal. | `tee/p21a-web-api-contract`, contract/transport review |
| [P21B — Public documentation](PLAN.md#p21b) | open | Published guides and method pages describe implemented contracts and actual environment availability, not planned endpoints or unverified guarantees. | — |
| [P21 — Recovery integration](PLAN.md#p21) | open | Fenced and reconciled funds back final claims; users obtain claim packages without the ordinary trading API. | — |
| [P22 — Offline adversarial acceptance](PLAN.md#p22) | open | SDK/service-driven trading, funding and recovery workflows preserve accounting under reproducible external fixtures, process faults and races. | — |
| [P23 — Live integration qualification](PLAN.md#p23) | open | The same workflow assertions run on approved actual Nitro/chain/venue rails; receipts, test exposure, funds and cleanup reconcile. | — |
| [P24 — Release safety case](PLAN.md#p24) | open | Reviewed evidence, calibrated policy and explicit governance support an honest release decision, not automatic deployment. | — |

Updated 2026-10-03. Only progress values: `open`, `in progress`, `closed`. A blocked
phase stays `in progress` with the reason below. `closed` requires completed PLAN
criteria, recorded tests/review and actual merge; a green/unmerged PR is not closed.
Dependencies and detailed acceptance criteria live in [PLAN](PLAN.md).

## Current handoff

Worktree: `cinder-tee`. Trunk: `product/tee-v1` at
`76f32956e3ee5d31707ec1bf7fa81f8711666c97`. Current branch:
`tee/p21a-web-api-contract`, a fresh native stack based on that exact merge.
P20 #45 merged at 2026-10-03 03:40:39 UTC through the required asynchronous
`gh stack merge --yes --merge` path. All three hosted jobs passed at checked
head `311c0a7fa55187357f42f52965e690f73746c4da`; CodeRabbit's completed delta
review to that head reported no actionable comments. Four earlier inline threads
are resolved; the outside-diff cloud-parser finding is fixed and regression-tested.
Local trunk is fast-forwarded. Hardware receipts remain exact-source, not a new
qualification of the later review fixes. Native/production gates remain separate.

P21A: the user approved all API_SCOPE recommendations, including richer reads,
history and scoped Cinder agents. Browser/Node HTTP and WebSocket serve both a
future frontend and programmatic trading clients. Typical frontend use is HTTP
commands plus WebSocket private updates; WebSocket commands are optional, not a
requirement to place an order. Public venue market-data feeds remain separate.
The user subsequently explicitly approved bounded local Noise NK qualification.
[ADR 0021](../architecture/0021-confidential-web-api.md) is tracked and the separate
`tools/web-channel/` Rust/WASM core uses pinned Snow 0.10.0 without altering the
shipping graph or P19 Node TLS. No HTTP trading handler or stream is implemented.

Native tests and actual Node/browser round trips pass with explicit synthetic
responder-key trust. Independent browser Nitro verification is NOT implemented;
the opaque library's key-erasure limitation is recorded before shipping. The full
repository runner also passes (debug/release workspace, 29 repository tests and
17 SDK/process tests). Next: publish the bounded first-slice native stacked PR
with its new Linux/browser CI, then qualify hardened dependencies, browser verifier and exact attested binding
before HTTP/WS integration. Continue API → Mintlify → recovery. Preserve ignored
work and unrelated `docs/user-journey.md`/`stays/`; no AWS, live venue, wallet,
container or subagent was used for this merge/qualification.

### Historical P20 pre-merge handoff (superseded)

Worktree: `cinder-tee`. Trunk: `product/tee-v1` at
`dc5f01ddadb6730886f939693b2b6c0c7d642caf`.
Current branch: `tee/p20-nitro-runtime`. Native stack #46 originally placed
[P20 #45](https://github.com/arnabnandikgp/cinder/pull/45) above
[P19 #44](https://github.com/arnabnandikgp/cinder/pull/44); P19 is now merged and
P20 is rebased directly on the updated TEE trunk. Historical code/evidence
checkpoint `b6e0f4c` became `adf42ea`; that rebase changed documentation only.
The runtime review fixes in `82073c221895f505e4d029f2b4e99c2a6e13031e` and
`c94adc94c3fa72b545c0aa5ce1448668309e239c` change source and are offline-tested,
not newly Nitro-qualified. All three hosted jobs passed at preceding published
head `479ef98155a3da2ccdbcb03973f96a5462f23656`. Four original CodeRabbit threads
are resolved; the completed review at `b2c2e8b` also identified one valid
outside-diff cloud-header finding, now fixed in `c94adc9`. Its regression failed
before the fix and passes afterward; complete Rust workspace debug/release tests,
strict all-feature Clippy and format pass. Plan validation and all 13 validator
tests pass using available Node 26.8.2. The final publication needs its own hosted
CI check before merge. The user authorized P20's merge and approved the complete
API_SCOPE.md recommendations, including bounded richer reads and scoped agents,
on 2026-10-03. Browser transport security selection still requires ADR review.
Hardware results remain tied to their actual source. Pre-hardware assembly is
`b67f3947b6eda241bd56912003b22ee90107857f`; real AWS compatibility fixes are
`37487ca`; the qualified export is `5557d3350f371d616965006a90ea03b55420400f`.
The bounded hardware matrix completed on 2026-10-02. Do not mark P20 closed
before review/merge or relabel the later review changes hardware-tested.
P20's pre-hardware assembly includes the consumed public manifest, actual NSM
clock/recipient, role-separated KMS release, encrypted S3 replicas, strong-read
DynamoDB witness, default-feature enclave executable and bounded private API/
supervisor; see [ADR 0020](../architecture/0020-nitro-runtime.md) and the
[hardware runbook](../operations/nitro-qualification.md). No seeded balance,
fixture dispatch or alternate ledger is promoted. Native trading, funding and
reads are deliberately disabled until qualified G01/G02/P23 ports and policy.
Local assembly is not a completed hardware criterion. Dated results below apply
to their exact source, not a later change. P19's authorized merge, exact checked
head and review disposition are recorded in its owning phase below. P20 remains
open pending final-fix CI and the authorized merge; no production activation is
authorized.

AWS: initial disposable test allowance **$5**, replacement profile `cinder_new`,
region `us-east-1`, no mainnet/customer funds. Fresh STS on 2026-10-02 verifies
the intended IAM user; read-only EC2 inspection confirms the candidate
`c6g.large` supports Nitro. The root-identity and Free-plan launch blockers are
historical. After the user's explicit Paid-plan upgrade, the actual
disposable Nitro session now has positive application/KMS/SDK and encrypted
restore/fault receipts; see [hardware evidence](../operations/nitro-hardware-2026-10-02.md).
The former Free-plan/no-resource/$0 blocker is historical. Three real AWS format
compatibility fixes are implemented and verified on macOS and Linux; that first
session's partial matrix is completed by the follow-up below. Review/merge remains
open. The evidence page records verified teardown
and conservative spend below $0.50 (not a finalized bill). No native trading/funding/read capability
was enabled. Do not delete source/research or use broad container pruning.

P17 #41 and P18 #42 merged atomically through `gh stack merge --yes --merge`
on 2026-10-01 after explicit user authorization. Both GitHub receipts report the
historical merge `1f750b059f39e9f409ae10fe2f4e070c2e0aa428`. Checked source heads were P17
`8868749f8805a49945bcaa2dcfe4e30044f4d6b2` and P18
`70641e322f8555eb3bfe671ca2fed5e460b762fb`. Plan/handoff, offline Rust and
Anchor/SBF/Surfpool jobs were successful at both exact heads; CodeRabbit statuses
were successful and both actionable threads had author replies and were resolved.
Local trunk fast-forwarded to that commit; its tree is identical to the tested P18
tip, which is also an ancestor. P17/P18 are now `closed`; no future P19 CI is implied.

The TEE trunk has no branch protection or branch rules configured. Passing checks
and resolved reviews were verified manually; no settings were changed and legacy
main is untouched. Production policy, source qualification, keys and deployment
remain G01–G05 gates. No AWS, live venue/RPC, funding, configured wallet, container
or agent was used for this merge.

Historical P18 trunk baseline, not verification of the P19 tip: 53/53 client/SBF
checks and 296 Rust tests plus two
compile-fail doctests per debug/release profile, 24 repository tests, strict
Clippy/format/build and 9/9 private SDK tests. Dated results and actual merge
receipts remain in the owning phases below.

Next: inspect latest-head CI/review for P20 #45's locally verified fixes and
approved roadmap, and verify the relevant thread replies/resolutions. No automatic
merge. Sequencing subsequently approved 2026-10-03: after P20's reviewed merge,
proceed with P21A HTTP/WebSocket implementation, P21B public docs, then P21 recovery
and P22/P23 joined offline/live acceptance.
Public docs follow implemented APIs, not speculative endpoint promises. Browser
transport needs its reviewed ADR before implementation; onboarding/terminal and
public market-data feeds remain separate scope.

Historical exact-source hardware closeout, 2026-10-02:
see [follow-up evidence](../operations/nitro-hardware-followup-2026-10-02.md).
The complete security batch now has an actual independently verified attested
PASS, including recipient/upstream TLS negatives and journal recovery. A real
current-snapshot restore bug was reproduced locally and fixed by reusing validated
immutable objects; failed accepted histories were preserved. Current export
`5557d3350f371d616965006a90ea03b55420400f` passes macOS workspace, targeted
macOS/Linux checks and all 17 SDK tests. Wrong-image rejection has same-artifact
positive brackets with unchanged KMS approval. The real separate-register CAS
race, final shipping application SDK/restart regression and debug refusal pass.
Natural 90-second loaded-lease expiry passes with retained head and enabled keys.
Actual witness STS expiry at 16:56:38 UTC and same-expired-capsule restart refusal
pass with a 40-minute boot lease, positive SDK bracket, unchanged accepted head,
five enabled keys and a live parent relay. No policy change or witness renewal
manufactured the result. The bounded hardware matrix is complete.
Retain exact-source receipts for the P20 security matrix
listed in [hardware evidence](../operations/nitro-hardware-2026-10-02.md). New
tests are compiled only in a separate test image, not the shipping enclave;
their actual hardware results and limits are recorded separately. The
scoped prerequisite policies and user-selected Paid upgrade resolved the prior
IAM/account-plan blockers. Technical P20 criteria are satisfied in the bounded
disabled-native profile; review/merge remains pending. No automatic merge or
production-key approval. P19 is closed after verified merge. P21 joins recovery
to the journal; P21A adds customer HTTP/WebSocket ingress; P21B documents it;
P22/P23 reuse workflow assertions offline/live. Onboarding/terminal remain
separate scope. Preserve ignored `work/`, untracked `docs/user-journey.md` and
unrelated `stays/`. Local fixtures alone do not establish
hardware qualification or authorize live keys, tests or deployment.
Second-session teardown is verified: termination requested at 17:04:07 UTC,
test EBS/buckets/table/runtime roles/SSH/security group absent, five fresh KMS
keys PendingDeletion and existing managed policies preserved. Exactly 45
generated local secret files were deleted; operator source and public/ciphertext
evidence remain. Only the task build container was removed. No additional
hardware categories remain from that approved batch. Later runtime changes have
their own source/test evidence; P23 qualifies the updated joined application.

### CodeRabbit coverage snapshot (2026-09-30)

Latest triage, 2026-10-01 (supersedes the historical rows below): P04/P07 now have
completed full reviews with no actionable finding. P05 #4147277655 is addressed
by its explicit P13 acceptance contract and P13's bounded qualified resolution
port. P08's later summary finding is reproduced/fixed with cancellation retaining
faulted holds. P13's later summary finding is reproduced/fixed with transactional
diagnostic inspection. P15's latest full review has no concrete defect; optional
runtime hardening is not a new offline acceptance requirement. P16 #4148900279
is reproduced/fixed without weakening ordinary/restoration risk fences. The 14
older resolved threads were checked for bot confirmation; remaining unchanged
layers have no new relevant actionable finding. New hosted reviews after the
published restack are not implied by this snapshot.

| Phase / PR | Existing review disposition |
| --- | --- |
| P04 / #27 | Review failed; no actionable finding submitted. Not evidence of a clean review. |
| P05 / #29 | Rate-limited; no actionable finding submitted. Shared P13 byte-budget fix added here. |
| P06 / #30 | Two earlier findings already resolved; no new unresolved finding. |
| P07 / #31 | Rate-limited; no actionable finding submitted. Not evidence of a clean review. |
| P08 / #32 | Bot explicitly reported no actionable comments. |
| P09 / #33 | Two valid pending-order/work-budget findings fixed and regression-tested. |
| P10 / #34 | Two earlier findings already resolved; no new unresolved finding. |
| P11 / #35 | Two earlier findings already resolved; propagated guards preserved. |
| P12 / #36 | Three valid receipt/void/house-owner findings fixed and regression-tested. |
| P13 / #37 | One valid archive-duplication/replay-budget finding fixed in P13 and P05. |
| P14 / #38 | Two valid read-cooldown/malformed-reply findings fixed and regression-tested. |

Generic docstring-percentage notices were assessed as non-blocking metrics, not
missing specified behavior; no blanket comment churn was introduced. Review
rate limits/failures are reported honestly rather than interpreted as approval.

Repository integration follow-up: the TEE trunk had no classic branch protection
or applicable rulesets at P00. CodeRabbit skipped P00's non-default-base review;
it later showed processing on P01, but no submitted review was present at merge.
Recheck current settings before claiming a review gate. ADR 0001 proposes scoped
TEE protections/review configuration and legacy-preview separation, with approval
for repository/deployment-setting changes. The existing Vercel check
reports deployment failure, but its build logs were not inspected; do not infer a
specific build error from that status alone. These integrations were left unchanged.

Historical pre-merge handoff (superseded by the current handoff above):
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
Merge: 95fe3b7395ac62d8c91554e27fa9a3032d3d26b9.

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
Merge: 95fe3b7395ac62d8c91554e27fa9a3032d3d26b9.

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
Merge: 95fe3b7395ac62d8c91554e27fa9a3032d3d26b9.

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
Merge: 95fe3b7395ac62d8c91554e27fa9a3032d3d26b9.

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
Merge: 95fe3b7395ac62d8c91554e27fa9a3032d3d26b9.

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
Merge: 95fe3b7395ac62d8c91554e27fa9a3032d3d26b9.

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
Merge: 95fe3b7395ac62d8c91554e27fa9a3032d3d26b9.

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
PR: https://github.com/arnabnandikgp/cinder/pull/35.
Merge: 95fe3b7395ac62d8c91554e27fa9a3032d3d26b9.

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
PR: https://github.com/arnabnandikgp/cinder/pull/36.
Merge: 95fe3b7395ac62d8c91554e27fa9a3032d3d26b9.

## P13 — Pacifica observations

Review follow-up (2026-10-01): addressed the remaining summary finding in #37:
rejected pages now preserve accepted view/cursor/scan state on ingestion and replay,
while retaining conservative gaps. Reproduced the failure before the change.
Completed #29's P13 dependency with exact-input qualified resolution, not a reset:
original commit/ordinal/fingerprint, source/epoch/cut/time, retained evidence and
same-transaction effects bind one permanent result. Actual facts survive refused
controls; retries never repost cash or clear unrelated faults. Seven journal
resolution groups and all 14 observation groups pass locally, including restart,
conflicts, duplicate-effect/hold isolation, restored exposure gating and work bounds.
The new semantics/state commitment fence old histories with engine 19 (wire 8).
Full assembled-stack checks passed at `1c3bd09` (see current handoff).
Live authentication/completeness remain
G01/P23 obligations; synthetic evidence hashes do not prove them.

Review follow-up (2026-09-30): CodeRabbit #4141545400 addressed in two owning
layers. P05 enforces the replay byte limit before append/exposure; P13 stores one
exact response archive with compact hash/ordinal references in normalized and gap
inputs. All 13 Pacifica observation groups and strict all-feature Clippy pass.
The new 32-row/gap regression verifies exact archived bodies, reference resolution,
bounded per-input overhead and restart reconstruction. Engine 17 carries the
stacked semantic fences; this storage-deduplication change does not rewrite history.

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
Next: P13 is ready for review; continue P14 bounded signing
and shared API-credit admission. Stop before P15; preserve all open G01/G02 gates.
PR: https://github.com/arnabnandikgp/cinder/pull/37.
Merge: 95fe3b7395ac62d8c91554e27fa9a3032d3d26b9.

## P14 — Pacifica execution

Review follow-up (2026-09-30): CodeRabbit #4141057824/#4141057830 addressed.
Trusted read 429 reporting binds a persisted read reservation, records shared
cooldown, supports late replies after key rotation, and deduplicates exact retries.
Malformed post-send bodies/times persist bounded Unknown evidence with HTTP status
and backoff instead of returning a pre-send-looking Codec error. Targeted Pacifica
tests pass (2 unit, 15 execution, 13 observation groups), plus strict all-feature
Clippy. New cases include restart, changed/unknown reservation rejection, repeated
responses, short/huge retry durations, oversized 200/429 and regressing clocks.

Work: 2026-09-30 — added exact native preimage persistence before signing, a scoped
real Ed25519 signer, GTC/ALO/IOC and attributed cancel encoding, durable key epochs,
shared API-credit/read admission and cleanup reserve, plus persisted unknown/429
response handling. No arbitrary-message or money-moving signer is exposed.
Verification: full pinned runner passed 230 Rust tests per debug/release profile,
two doctests and initially 23 repository tests. After ACK parsing/dependency-identity
hardening, the final clean tracked export of `0bd24d3` at
`/private/tmp/cinder-p14-export.wlqXju` passed the full pinned runner: 230 Rust tests
per debug/release profile, two doctests, all 24 repository tests, formatting, strict
Clippy in both feature configurations and build. Includes 14 signing/execution groups.
Unexpected: native signatures omit account/domain/epoch; bind them in the durable
envelope and require independently scoped keys. Limits are one-sided and native
agent authority is wider than trading; neither limitation is concealed. Read-credit
reservations include bounded delivery time; rotation does not replenish them. P13
hosted CI/CodeRabbit are green, no inline comments at this boundary.
Follow-up: final stack review found two P11 issues; both were reproduced, fixed and
verified in their owning branch (`9f1d2f8`), then restacked into this branch. Added
three regressions and advanced distinct P11/P12/P13 engine revisions to 8/9/10.
Final post-review clean export `2d15c16` at
`/private/tmp/cinder-p14-reviewed.DLjQgv` passed the full pinned runner: 233 Rust
tests per debug/release profile, two doctests, 24 repository tests, formatting,
strict Clippy and build. No ignored research or wallets were included. This
supersedes the earlier 230-test result for the assembled stack.
Next: STOP BEFORE P15. Review the open stack bottom-up; await user authorization
before merging or starting the Solana vault. Hosted checks for the final restack
must be inspected separately; local success is not a claim that hosted CI is green.
No merges, live calls, Solana vault work or new permissions in this run.
PR: https://github.com/arnabnandikgp/cinder/pull/38.
Merge: 95fe3b7395ac62d8c91554e27fa9a3032d3d26b9.

## P15 — Solana vault

Work: one cohesive Anchor 1.2.0 vault in the isolated `programs/` workspace;
typed classic-token/PDA constraints, upgrade-authority bootstrap, public customer
attribution, immutable deposit/operator receipts, allowlisted working-collateral
release/return, ordinary payouts with shared lifetime paid/sequence counters,
signed role handoff and epoch fence. Recovery authority only freezes; it cannot
pay through the normal path or reset history. Generated IDL/types and an unsigned
`@anchor-lang/core` 1.2.0 client are tracked. ADR 0015 promotes W04/W05 invariants
without fixture keys or a second private financial ledger.
Verification: all 25 signed SBF transaction tests passed locally in offline
Surfpool via the complete vault runner. All financial-workspace checks passed:
both strict Clippy configurations, all-target build, debug/release tests, doctests
and 24 repository regressions. Debug symbols/incremental compilation were disabled
to bound disk usage, not to disable debug assertions. Clean-export verification
passed from `/private/tmp/cinder-p15-export.FVQoPZ` as well: complete vault and
financial runners passed without `work/`, `stays/` or Git metadata, using only
hydrated dependency/build caches. Strict vault Rust and TypeScript compilation
passed. The reproducible runner checks exact tool and
lock pins, generated-IDL equality and stack diagnostics, owns its localhost-only
Surfpool lifecycle, and refuses occupied ports. A separate checksum-pinned CI job
does not contact venues or RPCs during tests.
Hosted checks for implementation head `74715cd` passed: plan/handoff consistency,
offline Rust workspace and offline Anchor vault. This records that tested head,
not automatic approval of later pushes or a deployment/recovery qualification.
Unexpected: Anchor's token-init macro references its Token-2022 module even for
typed classic Token; enabling that Rust module does not enable Token-2022 assets.
Boxed account wrappers remove generated SBF stack overflows. Customer deposit IDs
have a separate owner-bound namespace to prevent operator receipt squatting.
Prefunded-PDA tests must supply the system-account rent minimum, not one lamport.
Disk exhaustion required removing only reproducible debug artifacts, not source,
research, wallets or unrelated files.
Next: review ready PR #39 above #38 in the GitHub stack and check any later-head CI.
Do not merge or begin another phase without user authorization.
P16 joins finalized custody receipts/native funding to
the existing authoritative journal; P17 adds funded final claims sharing these
paid counters, not an independent payout program.
PR: https://github.com/arnabnandikgp/cinder/pull/39.
Merge: 95fe3b7395ac62d8c91554e27fa9a3032d3d26b9.

## P16 — Funding coordinator

Review follow-up (2026-10-01): #4148900279 is relevant and reproduced locally:
incomplete native funding incorrectly blocked a qualified emergency liquidation.
Narrowed the readiness fence to Order/Generic/Restoration, preserving all existing
emergency authority, freeze, evidence and funded-close checks. Added regressions
for both control orderings, restart, ordinary/generic/restoration refusal and
freeze under an incomplete-credit cut. Engine 20 fences changed replay semantics
and incorporates P13's engine-19 exact-input resolution after restacking. Full
assembled-stack verification passed at `1c3bd09`: 275 Rust tests plus two
compile-fail doctests per profile, 24 repository tests and all 30 client/SBF tests.
No container, external RPC/venue call, wallet access, deployment or delegation
was used. Fixes are published and both actionable inline threads resolved; P08/P13
summary replies explain their fixes. Hosted checks are running. P15's latest review has
no concrete defect; its signer/deployment trust qualifications remain documented.

Work: implemented the three-location funding controller, native credit-readiness
fence, immutable original plans/UUIDs, exact signed-wire persistence, separate
broker-owner signer with shared Gateway credits, causal receipt qualification,
house-owned actual fees, payout wallet/counter binding, marginal allocation and
residual reporting. Added the bounded unsigned Anchor client contract, instruction
builder and exact-wire signature verification; see ADR 0016. No second ledger,
new dependency, native account topology or custody/recovery promise introduced.
Verification: final staged-only export `/private/tmp/cinder-p16-export.Ft5cJS`
passes 262 Rust tests plus two compile-fail doctests in each debug/release profile,
both strict Clippy configurations, all-target build, formatting and 24 repository
tests. All 30 client/offline SBF tests pass with Anchor 1.2.0, locked generated-IDL
equality and pinned Node/SBF/Surfpool. This includes 17 new coordinator/risk/crash
regression groups, four real-signature codec groups and actual custody rail
execution. The ignored
child entry is actually invoked/killed by the parent regression. All 30 client/
offline SBF tests also passed in the working tree. The export excludes `work/`,
`stays/` and Git metadata and reuses only build/package caches. Only documentation
verification/publication notes follow that tested source. No Linux container, external
RPC, live venue call, deployment, wallet/cloud access or delegated review used.
Unexpected: the old two-location ledger could not represent intermediate broker
tokens honestly. Added that location to the same kernel, risk liquidity and replay
commitment (revision 18); old semantics fail closed. Forecast now includes prepared
ingress before exposure to avoid a reservation/dispatch double-allocation gap.
The old experiment's one-off deposit-first bootstrap permission is not promoted:
missing/lending-active accounts fail closed without a new approved manifest.
Source-qualified native completion and chain finality remain trusted observation
ports; the synthetic tests are not evidence of deployed production recognizers.
Focused self-review tightened one-use wire capability consumption and final
financial/authority checks before delivery, rejected noncanonical signed bytes,
and retained a regression for every older journal revision including 17. This is
not an independent security audit. The first whole-workspace run failed only the
old revision-17 golden assertion; it was updated and both final profiles pass.
Implementation commit: `c10354c`; only verification/PR-link handoff follows the
clean-export tested source. Native GitHub stack #28 contains the new P16 layer;
older PRs were unchanged. No merge was performed.
Next: review ready PR #40 above #39 and its hosted checks/review. P17 adds final
recovery claims/activation with preserved paid counters; stop before it until
requested and do not merge the stack automatically.
PR: https://github.com/arnabnandikgp/cinder/pull/40.
Merge: 95fe3b7395ac62d8c91554e27fa9a3032d3d26b9.

## P17 — Recovery program

Work: 2026-10-01 — implemented immutable final recovery statements and owner-signed
full claims in the same custody program. Governance publishes after freeze; a
separate recovery operator activates exactly that funded root. Canonical permanent
epoch/owner receipts prevent root resets and duplicate-owner payouts. Salted
program/domain/pool/asset/cutoff/counter-bound ordered membership, zero unresolved
qualification assertions, checked shared lifetime counters, all-remaining backing
containment, actual CPI rollback and terminal count/total consistency are enforced.
Added pure exact-unit packaging, generated IDL/errors/types and ADR 0017. Existing
config/customer layouts and authoritative financial journal semantics are unchanged.
Recorded actual P04–P16 atomic merge receipts and closed those reviewed phases.

Verification: pinned macOS offline workspace runner passes 275 Rust tests plus two
compile-fail doctests in each debug/release profile, 24 repository tests, formatting,
both strict Clippy configurations and all-target build. Pinned vault runner passes
**51/51** client/SBF checks, locked SBF/IDL and TypeScript checks. V08 executes real
ordinary payout 20, an explicitly synthetic external settled loss 35, and recovery
claims 45/200 with lifetime payments 65/200. Maximum 65,536-leaf membership executes
with 16 siblings: **1,124 bytes** without the optional compute instruction and
**34,433 CU** for the actual signed test transaction. Tests cover malformed paths,
wrong identities/roles/counters, unresolved reservations, frozen custody/destination,
impairment/repair, late arrivals, root/receipt replay, odd trees, over/understated
totals, malicious duplicate owners and integer overflow. Tests require no ignored
research or configured wallet, and use no external RPC/venue call, container,
deployment or agent.
This verification ran in the working tree; it is not mislabeled a fresh clean-export
run, independent audit or complete solvency proof. `git diff --check` passes.

Unexpected: first vault run passed 49/50; the alternate-recipient test tried to
create an already-existing ATA. Fixed the test to create a distinct token account,
then added funded-but-frozen activation containment; final full rerun passes 51/51.
The initial documentation validator rejected older Markdown/trailing-text PR receipt
formats once phases became closed; normalized four URLs, then the complete runner
passed. Added only the exact 3.1.0 Solana SHA-256 hasher already present transitively;
no transitive version or kernel dependency changed. Source qualification and private
claim delivery remain P21, not invented on-chain facts. One-time full recovery and
no new fee follow the bounded research contract; production terms stay G02-gated.

Next: ready P17 PR #41 targets merged `product/tee-v1`; review its hosted
checks/findings before any merge. P18 adds private owner/agent API/SDK grants and
operation semantics above P17, with confidential transport ports pending P19.
Do not deploy, call live venues or activate a real recovery estate from this handoff.
2026-10-01 publication refresh: head `3a26573`, ready/open, all three hosted protocol
jobs successful. CodeRabbit comments await triage; its green status is not review
closure. Preserve these dated results when review fixes create a new head.
PR: https://github.com/arnabnandikgp/cinder/pull/41.
Merge: 1f750b059f39e9f409ae10fe2f4e070c2e0aa428.

Review follow-up: 2026-10-01 — addressed CodeRabbit comment
[4149944472](https://github.com/arnabnandikgp/cinder/pull/41#discussion_r4149944472).
An overstated recovery total must prevent terminal closure, not payment of the
last valid included claim. Removed that payment gate; closure now requires both
all declared leaves consumed and exactly zero remainder. Full-estate backing,
owner signatures, lifetime counters, permanent receipts and ordinary-path fencing
are unchanged. ADR 0017 records the retained ACTIVE remainder and separately
reviewed remediation requirement; there is no root-reset escape.

Verification: reproduced the original SBF failure locally with the new regressions
(**50 passed / 3 failed**, each at the original `RecoveryTotal` gate). Rebuilt the
corrected program with the pinned Anchor 1.2.0/Node 24 toolchain; the complete
offline vault runner passes **53/53** checks, including the one-owner case and
both two-owner claim orders, exact token payouts, replay rejection, existing
backing/overflow/CPI rollback tests, strict Rust/TypeScript and locked SBF/IDL
checks. Clean staged-source documentation checks pass (25 phases, 27 documents,
12 validator tests). No configured wallet, remote RPC, deployment, container,
live venue, AWS or agent was used. Hosted checks for this new revision are pending;
the original head's green checks are not evidence for the fix. P17 remains
`in progress` until merged; carry this fix into P18 with the native stack workflow.

Merge follow-up: 2026-10-01 — P17 #41 is now closed. Its checked final head
`8868749` passed all three hosted protocol jobs; the recovery-total review thread
was replied to and resolved. Atomic merge with P18 produced the receipt above.
Earlier pending/ready notes are historical, not the current status. P21 still owns
actual ledger finalization and independent claim delivery; merge is not deployment.

## P18 — Private API and SDK

Work: 2026-10-01 — added the venue-independent `cinder-api` boundary and thin
`clients/private` SDK. Strict Ed25519/channel/domain/epoch authorization precedes
account-scoped replay/query; governed owner/recipient bindings and grants are
persisted in the same protected journal. Added per-market/per-order/lifetime grant
bounds, all-grant revocation with exposure fencing, server-only P09 admission
port, attributed cancels, fixed owner payouts, exact position/cash/funding/hold
views and explicit ACK/partial/unknown/actual-payment semantics. Promoted W06 in
ADR 0018, pinned only existing crypto/tool dependencies and added offline SDK CI.
Verification: complete pinned offline macOS runner passes from a clean staged
export with no research/credentials/unrelated files: **294 Rust tests and two
compile-fail doctests in each debug/release profile**, 24 repository tests, both
strict Clippy configurations, formatting, all-target build, strict TypeScript and
**9/9 SDK tests**. New P18 coverage is 17 encrypted-journal authorization/lifecycle
groups and two interop groups (eight exact signed-method vectors and shared reply).
`git diff --check` passes. No container or new vault/SBF run was necessary; no
hosted result is claimed before publication.
Unexpected: local fixture assertions initially confused tentative preparation
with signature exposure, and cancel-status routing with the cancel attempt rather
than its original order. Corrected the fixtures without weakening the actual
authority/terminal gates. Tightened grant dispatch expiry and trade-only replay
ownership during self-review. No new economic policy or production limit adopted.
2026-10-01 publication refresh: head `5ba8915`, ready/open, all three hosted protocol
jobs successful; CodeRabbit comments await triage. Planning clarification adds
explicit runnable backend deliverables and the SDK/service integration ladder to
PLAN/architecture/contributor handoffs; no runtime is implemented by those edits.
Documentation-only verification: `node scripts/check-implementation-plan.mjs`,
the 12 validator self-tests and `git diff --check`. No Rust/SBF rerun is claimed
for this planning-only update; review code changes will receive owning-layer checks.
Review follow-up: 2026-10-01 — addressed CodeRabbit comment
[4150398996](https://github.com/arnabnandikgp/cinder/pull/42#discussion_r4150398996).
Replaced per-record scans of all earlier API records with a standard-library
`BTreeSet` keyed by account and request ID. Duplicate detection is now O(R log R)
rather than O(R²); the history is still replayed on each request, and the returned
record vector preserves journal order. No cache, dependency, alternate journal or
authorization/economic change was introduced. Added encrypted-SQLite regressions
for shared request IDs across accounts, ordered private operation replay, and
duplicate retained evidence failing closed without mutation before/after restart.

Verification: clean staged export `/private/tmp/cinder-p18-review.OTmNJw` passed the
complete pinned offline runner: **296 Rust tests and two compile-fail doctests in
each debug/release profile**, 24 repository tests, both strict Clippy configurations,
format/build and **9/9 SDK tests**. The focused API run also passes 19 lifecycle
groups and two interop groups. The first new test compile caught a moved expected
receipt; the assertion now borrows it, without a production-code workaround.
Only tracker/report publication follows that tested source; documentation and
whitespace checks run again before publication. P17's source-identical 53-check
SBF result covers the inherited recovery fix, not a second P18 SBF run. No live
venue/RPC, deployment, AWS, configured wallet, container or agent was used.

Next: verify publication above P17 `8868749`, the review reply/resolution
and new-head hosted checks. Keep P18 in progress until its actual merge. The
runnable P19 service is a new phase, not implemented by these planning or
API-review changes.
P19 owns real attested transport, a runnable service slice and client verification;
P20 hardware, P21 integrated recovery and G01–G05 remain explicit gates. No live
venue/RPC, deployment, AWS, configured wallet, container or agent was used.
PR: https://github.com/arnabnandikgp/cinder/pull/42.
Merge: 1f750b059f39e9f409ae10fe2f4e070c2e0aa428.

Merge follow-up: 2026-10-01 — P18 #42 is now closed. Its checked final head
`70641e3` passed all three hosted protocol jobs; the history-scan review thread
was replied to and resolved. Atomic merge with P17 produced the receipt above.
The local trunk tree exactly matches that tested tip. This next-branch handoff
only records the actual merge, with no runtime changes or new test claims.
Next: P19 attested transport and runnable service; implementation is not started.

## P19 — Attested transport and runnable service

Work: ADR 0019 selects enclave-terminated TLS 1.3, bounded synchronous listeners and the independently distributed public-data quote verifier. New service/relay/fixture entrypoints wire P18 to the real P06 cipher/replica journal. Node SDK gates private requests on the exact socket SPKI/exporter, fresh challenge, expected release/domain and short expiry. Default builds exclude fixture attester/key providers; production verifier never accepts fixture roots. Exact dependency/feature/build-script policy is updated; no financial policy, native signing or Solana program change.
Verification: 2026-10-01 pinned full offline checks passed on macOS ARM64, including a staged-source export excluding work/, user-journey.md and stays/. Each debug/release profile passed 303 Rust tests plus two compile-fail doctests; 25 repository tests and 17 SDK tests passed (seven use actual local service/relay/verifier processes). Default/all-feature Clippy, format, build, exact dependency policy and strict TypeScript checks passed. Relay ciphertext corruption cannot forge a request; a corrupted committed reply reconciles after process death without another attempt. No AWS, RPC, wallet, funding, container or agent used; SBF not rerun because on-chain source is unchanged. Local CA/clock/witness/venue evidence is synthetic, not NSM or independent storage qualification.
Unexpected: real transport exposed Darwin inherited nonblocking sockets and Node Buffer.slice aliasing in the P18 SDK. Explicit worker socket mode and copy-safe IDs/reply decoding fix both; tracked regression tests retain them. A client TLS protocol accessor reports configured legacy version even on rejected handshakes, so the downgrade test checks secureConnect was never reached rather than that accessor.
Review evidence: P19's source verification is 303 Rust tests plus two compile-fail
doctests per debug/release profile, 25 repository tests and 17 SDK tests, with
strict Clippy/format/build and TypeScript checks. Hosted Rust, Anchor/SBF/Surfpool
and plan jobs passed at `1aeef17`. CodeRabbit's one valid minor finding was the
unlabeled P18 baseline in the current handoff; the documentation-only correction
preserves historical evidence without attributing it to P19. No runtime or test
source changes. These P19 details belong in their owning phase rather than being
presented as verification of P20's later source.
Historical next action (superseded by the merge below): P19 #44 was ready for review through native gh stack on product/tee-v1; P20 supplies actual Nitro qualification, separately from P19's local service evidence.
PR: https://github.com/arnabnandikgp/cinder/pull/44.
Merge: dc5f01ddadb6730886f939693b2b6c0c7d642caf.

Merge follow-up — 2026-10-02: one valid minor CodeRabbit finding was fixed in
`486de5319cd4538b82012dfe65beb03e972b2491`, with a public fix reply and resolved
thread. Planning validation, its 12 tests and diff checks pass locally. All
three hosted jobs (Rust, Anchor/SBF/Surfpool and plan consistency) pass at that
exact head. CodeRabbit's original completed review remains the source review;
the latest status is rate-limited, not a fresh completed review of the doc fix.
After explicit user authorization, `gh stack merge 44 --yes --merge` merged only
P19 at 17:26:30 UTC, leaving P20 open. Local trunk fast-forwarded to the receipt
above. P19 is now `closed`; legacy main, wallets and cloud resources are untouched.
Next: review P20 #45 on the updated TEE trunk; no automatic upper-layer merge.

## P20 — Nitro application qualification

Initial work (historical): `tee/p20-nitro-runtime` above P19 head `1aeef179b6f963649954f2e1a2da8eab1b6690d6`; the current branch is now rebased on the merged trunk. Application-policy commitment derives from the actual journal/API/execution/funding objects, not a supplied hash; rejects conflicting beneficiaries, mismatched configurations and reused trading/funds key roles. Actual NSM adapter pins AWS's 0.5.2 driver, opens Linux hardware only, checks locked nonzero exact PCR0/1/2 and known SHA384 profile, requires bounded entropy, and verifies fresh connection-bound quotes with the production AWS-root verifier. Dependency/feature/lock guards retain a closed graph. No economics, history codec, Solana source or native capability activation changes. ADR 0020 records the partial implementation and remaining boundaries.
Verification: 2026-10-01 final-source pinned offline runner passes on macOS ARM64: 310 Rust tests plus two compile-fail doctests in each debug/release profile, 26 repository tests, all 17 SDK tests (including the seven real local-process tests), default/all-feature strict Clippy, format, build and TypeScript checks. Four new NSM tests and three loaded-contract tests pass; dependency guard includes the new driver feature/pin/platform refusal test. Linux driver path and actual hardware tests are still pending; local NSM protocol exchanges are synthetic, not hardware evidence. None of the four complete P20 acceptance criteria is marked satisfied. On-chain source is unchanged, so SBF is not rerun.
Unexpected: current `cinder-dev` STS identity is account root; the same ARN was freshly confirmed after the user's IAM clarification. Provisioning is blocked pending the intended non-root credentials, independently of local implementation. User approves at most $5 for the initial disposable session; no resources or billable spend. Host disk is tight (~2.6 GiB free before the final checks) and Apple containers have no cached images. Container services/images were inspected read-only; no wallet, venue, RPC, AWS write, agent or Linux container was used.
Initial next action (2026-10-01; superseded below): finish local runtime ports and obtain non-root authentication before bounded hardware tests. No P19 fixture promotion or automatic unknown retry.

Continuation (2026-10-02): the replacement IAM profile `cinder_new` is verified;
the prior root blocker above is historical. Added fresh signed NSM clock samples,
sticky backward-time/error fencing, five-second late-result rejection, distinct
clock/session purpose binding and explicit clock-driven TLS certificate dates.
Fixed the real-NSM quote-versus-earlier-clock ordering error without relaxing SDK
client freshness. Six new regression groups pass within the service suite:
14 unit tests and six integration tests, offline on macOS ARM64; strict service
Clippy passes after correcting test comparison/layout issues. The complete pinned
root runner subsequently passes: 316 Rust tests plus two compile-fail doctests per
debug/release profile, 26 repository tests and 17 SDK tests, with format, strict
default/all-feature Clippy, build, dependency guards and TypeScript checks. Linux
and hardware qualification are still pending for this checkpoint.
Pinned Node 24.21.0 was restored under a task-specific temporary directory and
checked against the official archive SHA-256. This local clock checkpoint is
committed before the next socket/dependency slice; P20 has no PR yet. No cloud
write, paid resource, live venue or wallet was used. Apple containers were started
explicitly for the upcoming Linux-specific socket qualification; export only
tracked source plus intended changes, never ignored research or credentials.

### Socket/HTTPS continuation (2026-10-02)

Implemented safe owned Linux vsock streams/listeners, exact destination/peer
checks, shared bounded TLS serving, opaque parent ingress and fixed-origin
egress executables. Venue HTTPS verifies the actual loaded CA, hostname and
qualified-clock certificate time inside the confidential runtime; signed requests
are never parsed by the parent. Add explicit socket2/Windows dependency pins and
five negative guard mutations. Narrow HTTP framing and all failures stay Unknown,
without retry or release of an escaped attempt. No native capability is activated.

Verification: the complete pinned macOS offline runner passes (27 repository
tests, 17 SDK tests, strict default/all-feature Clippy/build, Rust debug/release
and compile-fail doctests). The entire Rust suite, format/default/all-feature
strict Clippy and build also pass in network-disabled Linux ARM64 against staged
source tree `f9221a4df607d362a17f9d3ee37799371d6d4ae0`. Linux-only owned-socket
operations and NSM driver compilation are now checked; a Unix pair/missing device
is not Nitro hardware evidence. SDK processes are macOS evidence only. Exact
image/platform/library details and setup failures are in ADR 0020. Anchor/SBF
source is unchanged. No agent, venue/RPC/wallet, deployment or AWS resource used;
AWS test compute spend remains $0. Source/research and local user-journey/stays
are preserved. The generated incremental cache and task builder are recoverable
by rebuilding, not source deletion.

Historical next action (superseded by the assembly below): complete consumed-runtime manifest and financial executable, actual RNG
integration, recipient-bound KMS release, authenticated encrypted remote replicas/
independent witness and bounded controller loops. Then package/qualify real vsock,
native response compatibility, loaded-key/capability fencing and crash/restore on
Nitro only within the approved $5 test session, with pricing/permission/cleanup
checks first. Keep all four P20 criteria open and no PR ready/complete claim. P19
#44 remains open/unmerged; production authority/topology and G01–G04 remain gates.
PR: none.
Merge: none.

### Complete pre-hardware assembly (2026-10-02)

Work: actual measured manifest binds all consumed application/storage/witness/KMS/
clock/egress/gate fields. Add one-shot purpose-bound RSA KMS recipients and five
separated confidential roles, the official closed SigV4 signer, encrypted immutable
S3 replicas and strongly consistent conditional DynamoDB witness. Missing witness
never initializes history and the enclave writer cannot reset Epoch. The actual
API, gateway and funding controller share one restored journal; sticky time/epoch/
storage fencing and finite supervisor prevent stale work or restart resend.
Add policy-derived gross customer/native margin holds without fixture constants.
The default-feature enclave and parent/operator tools, trusted local preparation,
minimal rootfs recipe and detailed permissions/startup/fencing/budget/cleanup
runbook are implemented. No native read/chain authority is pretended: qualification
manifest refuses trading/funding/native-read activation; P22/P23 own those workflows.

Verification: final pinned macOS full offline runner passes 332 Rust tests plus
two compile-fail doctests per profile, 28 repository tests and all 17 private SDK
tests. Complete Linux ARM64 format, strict default/
all-feature Clippy/build and debug/release suites pass on exact staged source
`629c4a0fe4a04f0ef027c04481d9eeee1dabcff5`: 332 Rust tests plus two compile-fail
doctests per profile. Default-feature package compiles all seven enclave/parent/
operator tools; recompiling the enclave application yields byte-identical ELF.
Ordinary-host boot and the minimal-rootfs chroot both fail redacted, not as a
successful hardware test. Enclave ELF SHA-256 is
`d0679f2d0fa908369f1816413dd3333b9d2fb86862f54f89dcf3b7a533dc0399`;
local bundle/hash receipt path is in ADR 0020. Exact local
image/library pins and previous receipts remain in ADR 0020. No NSM device, AWS
resource, wallet or live venue was present in the Linux check. Anchor/SBF unchanged.

Unexpected: the official generic signer needs S3's explicit payload-hash header;
corrected before Linux checking and covered for PUT/empty GET. DynamoDB uses JSON
1.0, not KMS's JSON 1.1. The signer requires its explicit http1 feature; the closed
dependency guard now records the reviewed graph. Test setup's reused journal
directory was corrected, not a changed financial rule. Apple Containers cannot
copy artifacts from a stopped container; briefly resume it for artifact extraction,
then remove only that task container. Removed only regenerable target/debug for
disk capacity; local research/user-journey/stays are preserved.

Next: stop before the AWS hardware session. Actual NSM/kernel/OpenSSL/OsRng,
AF_VSOCK routing, KMS OAEP/CMS/denials, real AWS HTTP compatibility, remote restore/
split-writer and loaded-key/SDK fencing still need their bounded hardware receipts.
EIF/PCRs and finalized disposable resource identities are session outputs, not
invented software receipts. No complete P20 hardware criterion is closed; no
billable resources or test compute charge incurred. P19 remains open/unmerged.
Commit: `b67f3947b6eda241bd56912003b22ee90107857f`; preserved locally before
hardware work. Final documentation handoff export
`695f7110a6f13c1bc5e679de5b9fb19fc257ceef` passes link/progress checks without
ignored work, local-only user journey or unrelated stays. Runtime/build sources
match the Linux tested tree; final receipt edits are documentation-only.

### Hardware resumption / IAM prerequisite (2026-10-02)

Work: user authorizes resuming the existing $5 disposable session. Fresh STS
confirms `arn:aws:iam::588966314654:user/cinder_new`, region us-east-1. Verified
preserved Linux artifacts and c6g.large's two ARM64 CPUs/4096 MiB/enclave support.
EC2 describes the default VPC and latest Amazon-owned ARM64 AL2023 image; S3
metadata read works. No root profile or customer wallet used.

Verification: KMS ListKeys, DynamoDB ListTables, IAM own-policy inspection,
Pricing GetProducts, SSM public parameter and EC2 quota lookup return AccessDenied.
These are read denials, not proof every Create action is denied. No blind creation
probe or partial paid deployment was attempted without known cleanup authority.
The IAM prerequisite is separate from an attestation or application-code failure.

Unexpected: authenticated IAM identity has insufficient permission visibility for
the previously planned KMS/witness topology. Prepared account/session-scoped
operator and fixed runtime-boundary policies in ignored local
`work/experiments/p20-hardware/permissions/`, with administrator setup instructions.
They grant no IAM user management, policy versioning or boundary removal; new
roles require the fixed administrator-created ceiling. No arbitrary existing KMS
tagging is granted. Syntax/managed-policy size are checked, not effective AWS
authorization; the added policy does not subtract existing attached rights.

Next: user applies the two managed policies using an administrator console,
attaches only CinderP20Operator to cinder_new, then we verify effective test/cleanup
permissions, current prices and quotas before launch. No AdministratorAccess or
root CLI is requested. No AWS writes/resources/test compute spend; four hardware
criteria remain open. Current application/executable sources are unchanged.

### Hardware launch / account-plan prerequisite (2026-10-02)

Work: verified the actual v1 CinderP20Operator and CinderP20RuntimeBoundary
documents against the prepared local JSON. STS still identifies cinder_new;
EC2 quota is eight standard on-demand vCPUs. Current regional c6g.large Linux
on-demand price is $0.068/hour. Selected Amazon-owned ARM64 AL2023 kernel-6.1
AMI `ami-0ae8605ed708e3c3e`, public default subnet and restricted operator-/32 SSH.
Prepared a local resource ledger and two-hour shutdown/termination configuration;
no IAM instance profile or native/customer authority is loaded.

Verification: EC2 RunInstances dry-run reports DryRunOperation, but actual launch
returns InvalidParameterCombination: the selected instance is not eligible for
Free Tier. Current describe-instance-types returns eight Free Tier eligible types,
all with NitroEnclavesSupport=unsupported. No instance was launched; no volume,
application KMS key, bucket, witness table or runtime role was created. Deleted
the exact temporary SSH key and security group; follow-up EC2 metadata confirms
no session instance, volume, group or key pair. Disposable local SSH files are
removed. Actual hardware criteria remain open; test compute spend is $0.

Unexpected: account-plan restrictions are checked by the actual launch, not the
successful EC2 permission dry-run. This is distinct from the resolved IAM blocker.
No plan upgrade, wider IAM grant, unsupported instance substitution or synthetic
hardware receipt was attempted. The local session script remains ignored research
tooling; it is not a production/provisioning interface or reviewed release artifact.

Next: user chooses a direct Free-to-Paid account-plan upgrade in AWS Console
(remaining eligible credits continue applying under AWS's documented terms), or
provides another explicitly authorized Nitro-capable account. Do not change billing
plans automatically. Then refresh identity, plan eligibility, budget and resource
preflight and recreate fresh SSH setup for the same bounded $5 qualification.
P19 remains open/unmerged; P20 has no PR and no hardware acceptance claim.

### Actual non-debug hardware session (2026-10-02)

Work: after the user's explicit Paid-plan upgrade, ran one tagged c6g.large with
a default-feature measured application, five recipient-only KMS role keys, two
private ciphertext buckets and a region-local independent-credential witness.
Native trading/funding/read gates stayed false. Real NSM/KMS exposed three format
compatibility failures: the closed indefinite NSM map, AWS leaf's absent AKI,
and BER rather than canonical DER CMS. Implemented narrow bounded fixes and
regressions; temporary diagnostic hooks are not in the accepted executable.
One-hour witness renewal retained original keys/domain/stream/epoch/application
before first journal initialization; no accepted witness was reset.

Verification: [dated hardware evidence](../operations/nitro-hardware-2026-10-02.md)
contains exact candidate ELF/EIF/PCRs and tested/remaining distinctions. Actual
SDK views/grant/revoke, quote/policy negatives and disabled native order pass.
Crash restore, missing-first replica fallback, both-copy loss, authenticated
ciphertext corruption, witness-route loss and loaded retained-head epoch fencing
pass without forgotten history or financial movement. Full macOS workspace release
tests plus two compile-fail doctests pass; 30+8 service tests and strict Clippy pass
on Linux; 17 SDK tests/TypeScript and 28 repository tests/guards pass. The prior
debug runner is historical, not a debug rerun for these new fixes. No SBF change.

Unexpected: real AWS signed-map/certificate/CMS encodings differed from stricter
synthetic assumptions. Parser bounds, pinned trust, exact original signature and
recipient/purpose/body checks remain enforced. Initial probe setup also exposed
tunnel expiry, startup readiness and a zero-position test expectation; these are
not counted as successful failure tests or a reason to automatically retry money
movement. No controller/economics/ledger codec or production capability changed.

Cleanup: verified actual EC2 termination and EBS removal; both buckets/objects,
table, runtime roles and SSH/SG setup absent. Five keys PendingDeletion for the
seven-day minimum; preexisting managed policies retained. Removed 13 exact local
throwaway plaintext/credential/key files and the task container/tunnel. Conservative
spend estimate below $0.50, within $5; final AWS billing is not yet available.

Next: remaining isolated KMS recipient/context/role/replay cases, actual competing
writer/uncertain append, expiry, stale/orphan replay and upstream-integrity matrix
in the evidence page. Keep all four complete P20 criteria open. Preserve sanitized
receipts/ignored work and unrelated untracked files. No P20 push/PR or P19 merge
has occurred or is authorized by this hardware session.

### Hardware-probe preparation (2026-10-02)

Work: added three explicitly ignored, test-only Nitro probes: authenticated
per-role KMS context/recipient/ciphertext rejection with positive brackets;
actual independent-client DynamoDB CAS contention on a fresh empty disposable
register; and fixed-origin Pacifica paper TLS handshakes with CA, hostname and
encrypted-record negatives. The qualification executable is separate from the
shipping application, has no fixture feature and needs its own independently
approved measurement. The witness hashes are synthetic, not journal frames;
never boot the application against that mutated register. TLS sends no HTTP or
native action. Neither compilation nor skipping is a live receipt.

Verification: source export tree `4b2bf7279a51a289de7fa67c4fd3e0fad65a7910`
passed macOS release service checks: 32 unit and 8 integration tests with all
features, 25 default-feature unit tests, three hardware cases explicitly ignored,
and strict all-target/all-feature Clippy. Linux ARM64 default-feature release
unit checks passed 25 tests with three hardware cases ignored; strict
all-target/all-feature Clippy passed. Both builds used Rust 1.97.1, locked/offline
dependencies. Linux source and pinned vendor input were read-only with networking
disabled and no `work/`, wallet or AWS mounts. The public Linux test ELF SHA-256
is `6ff140088d53ae608199da77d746d45cfcf58b3ee9603c740dc3e7b1275b2949`;
no EIF/PCR or KMS approval is claimed for it yet. Node 24.21.0 plan validation
and 12 validator tests passed. Whole-workspace/SDK/SBF/hardware results above
remain dated evidence, not new reruns at this export.

Unexpected: AWS cloud and Pacifica shared an Internet root, so that root cannot
serve as the wrong-CA test. The negative now uses the distinct Nitro root and
retains the real upstream SNI while testing a different verification hostname.
An attempted operator-tooling update was rejected by the execution safety review
because it selected a new ledger while leaving old resource/tag names in place.
That patch was not applied; the existing `hw1` launcher and receipts are unchanged.
Do not launch it against the cleaned historical ledger or change IAM policy
automatically. Prepare a consistently isolated second-run scope and resolve its
narrow permission requirements before any AWS mutation.

Next: finish that safe session setup and expiry/stale-orphan/uncertain-append
recipes; then run the measured probes under the existing total $5 ceiling and
retain exact results/cleanup. No new AWS resource or spend was incurred in this
preparation. All four full P20 hardware criteria remain open; no push, PR-ready
claim, native capability activation or P19 merge.

### 2026-10-02 second hardware invocation — fixed final scope

Work: fresh generation-two disposable resource ledger/tags; separately measured
test-only executable and current default-feature shipping ELF. Actual full
security-batch PASS includes five-role KMS negatives, upstream TLS negatives,
real encrypted orphan exclusion and accepted lost-ack recovery. Wrong-image
denial has same-artifact positive brackets with unchanged KMS approval. Real
separate-register CAS race, final application SDK/restart, debug refusal and
natural 90-second loaded-lease expiry pass. Actual witness STS expired at
16:56:38 UTC with a 40-minute boot lease: SDK positive shortly before, then
API refusal/enclave exit with unchanged accepted head and five enabled KMS keys.
Restart with the same expired capsule refuses too; fresh parent relay STS does
not renew the sealed witness. No new test category is queued.

Verification: code export `5557d3350f371d616965006a90ea03b55420400f` passes
macOS locked/offline release workspace, targeted macOS/Linux journal/service
release checks and strict Clippy, and 17/17 SDK checks with TypeScript. Linux
source/vendor inputs are read-only and networking disabled. Public ELF/EIF/PCRs,
actual verdicts and limits are in the [dated follow-up](../operations/nitro-hardware-followup-2026-10-02.md).
Latest documentation validator passes 25 phases/34 documents; diff check passes.
No SBF change/rerun, native capability, real funds or production approval.

Unexpected: immutable S3 restore exposed a real bug; regression was red before
validated-copy reuse and green afterward, followed by actual hardware PASS.
Failed accepted histories were not reset. Public test-entrypoint permissions,
wrong-role AWS error expectations and invalid random agent-key input needed
operator/test corrections. Failed/transient probes are retained, not hidden or
counted as passes. Runtime test source is versioned; account-specific operator
scripts remain ignored local research. Verified exact cloud teardown and
45-file disposable-secret cleanup preserve operator/public evidence and user
wallets, AWS profiles, research and unrelated files. The task container is removed.

Next: P19 #44 is merged; P20 #45 is rebased on `dc5f01d` and remains open.
Implementation/evidence commit `b6e0f4c` is now `adf42ea`; code-export inputs are
unchanged and only tracker/publication text differs from the original P20 tip.
The rebase conflicts were documentation-only: preserve the P18 baseline label
and keep P19-specific verification in its owning phase. Current plan validation,
its 12 tests and diff checks pass; hosted CI/review must be checked at the new
published tip. No further hardware categories without discussion. P20 stays
`in progress` until review/merge; no native activation or production approval.
PR: https://github.com/arnabnandikgp/cinder/pull/45.
Merge: none.

### Review follow-up — 2026-10-03

Work: CodeRabbit #4168125099/#4168125110/#4168125139 are valid. Runtime fix
`82073c221895f505e4d029f2b4e99c2a6e13031e` permits repeated unconsumed HTTP fields
while counting every field and rejecting duplicate consumed metadata; repairs a
missing funding-route bind from verified history after interrupted genesis; and
resolves fresh IPv4 routing per connection with one TCP attempt and no address
fallback. The AKI nit uses a documented named constant: the pinned safe OpenSSL
wrapper omits that constant and `from_raw` requires forbidden unsafe code. No new
dependency or certificate waiver is introduced. #4168125151 updates the ADR and
current handoff so historical pending launch/qualification checkpoints are not
presented as current. PLAN/TRACKER now include approved P21A/P21B insertion after
recovery, with dependencies, PR-sized criteria and implementation-before-docs.

Verification: header and interrupted-boot regressions were red before fixes.
Full pinned macOS `node scripts/check.mjs` passes format, strict default/all-feature
Clippy, all-target build, debug/release workspace tests, TypeScript and 17/17 SDK
tests. The new DNS test checks fresh IPv4 selection, resolver failures and no
retry; the AKI fixture pins the actual strict-path error/depth. Plan validation
passes 27 phases/34 documents and all 29 repository tests, including the new
letter-suffixed phase dependency/handoff regression. One local Clippy run found
test-module placement; moving it to the end fixed it without suppressing lint.
Pre-fix hosted plan/Rust/SBF jobs passed at `d07038df`; latest publication needs
its own CI/review. No local SBF rerun: program/client sources are unchanged.

Unexpected: no new economic or custody decision. Review additions are offline
tested; October 2 hardware receipts remain exact-source evidence, not qualification
of the new runtime source/PCR. No AWS, live venue, wallet, container or agent was
used. The focused local review artifact stays ignored under `work/reviews/`.

Publication: runtime fix `82073c2` and roadmap/evidence update `083a641` are pushed
to #45. All four actionable threads have author fix/test replies and are resolved;
the safe named-constant alternative is explained in a summary comment. At the
post-push check, `083a6413107763daf29a4605e1a3587e492c45a8` has successful hosted
plan validation; Rust and SBF jobs are in progress, CodeRabbit is pending. No new
review/CI pass or merge is presumed. Final prose-only plan validation and all
13 validator tests also pass with available Node 26.8.2 after the temporary pinned
Node binary was removed; the earlier full run used pinned Node 24.21.0.

Sequencing follow-up: the user subsequently approved HTTP/WebSocket → Mintlify
docs → P21 recovery. PLAN/TRACKER are reordered without renumbering completed
phases or deleting any recovery criterion. API_SCOPE.md records the current
eight-command surface, HyperLink method comparison and explicitly proposed
read/history extensions. Local plan validation passes 27 phases/35 documents,
all 13 validator tests and diff checks on available Node 26.8.2. No runtime
change or additional Rust/SBF/hardware test is implied by this planning update.

Final outside-diff review follow-up: review #5398556153 correctly identified
that `cloud.rs` still rejected harmless repeated fields. Runtime commit
`c94adc94c3fa72b545c0aa5ce1448668309e239c` validates/counts every field line,
retains only consumed metadata and rejects its case-insensitive duplicates.
Regression covers repeated Set-Cookie/Vary, duplicate Content-Length and exactly
64 versus 65 field lines; it failed before the fix. Cloud tests (4/4), full
offline Rust debug/release workspace suites, strict all-feature Clippy, format,
plan validation (27 phases/35 documents), all 13 validator tests and diff checks
pass. No SDK/program source change, AWS session, live venue call or container.
Existing hardware receipts are unchanged; the new parser is not hardware-tested.
Merge is now explicitly authorized, subject to final published-head checks.

Merge receipt: #45 merged via `gh stack merge --yes --merge` at 2026-10-03
03:40:39 UTC. Checked source head
`311c0a7fa55187357f42f52965e690f73746c4da` passed hosted plan/handoff, offline
Rust and Anchor/SBF/Surfpool jobs. CodeRabbit run
`5a1c982f-3ec8-4b5a-b8b5-78681f23c85e` reviewed the delta from `b2c2e8b` to that
head and reported no actionable comments. The generic docstring percentage warning
does not identify a remaining defect; no blanket comment churn. Ordinary `gh pr
merge` refused this stacked PR before making a change; the required native stack
merge then succeeded. Local product/tee-v1 fast-forwarded to the receipt below.

Next: P20 is closed. P21A starts from the merged TEE trunk with the approved
scope; finalize its separately reviewed browser transport, implement the API,
publish P21B, then complete P21 recovery. Native trading/funding/reads and
production policy remain G01–G04/P23 gates. Later review fixes remain offline-
tested, not a rerun of the historical hardware matrix.
PR: https://github.com/arnabnandikgp/cinder/pull/45.
Merge: 76f32956e3ee5d31707ec1bf7fa81f8711666c97.

## P21A — HTTP and WebSocket API

| PR-sized slice (ADR 0021) | Progress | Completion boundary |
| --- | --- | --- |
| 1 — Contract and isolated core qualification | in progress | Reviewed contract plus offline native/browser vector and hostile-wire evidence, then merge; not attestation or a shipping API. |
| 2 — Hardened dependency, browser verifier and binding profile | open | Independent AWS/COSE/X.509 verification and exact attested channel vectors match the strict Node policy; no parent verification substitute. |
| 3 — HTTP and shared SDK/service | open | Encrypted browser/Node commands reach the same authoritative API/journal, including lost-response/restart reconciliation. |
| 4 — Authorized reads and WebSocket | open | Own-account projections/paging and bounded revocable committed streams; no native pooled-query/global-sequence leakage. |
| 5 — Joined adversarial acceptance | open | Both transports satisfy all four parent-phase criteria; then P21A can close after merge. |

Work: API → public docs → recovery ordering and full API_SCOPE approved 2026-10-03; P21 is not an API prerequisite. Started tee/p21a-web-api-contract on P20 merge 76f32956e3ee5d31707ec1bf7fa81f8711666c97. After the frontend/bot explanation, the user explicitly approved bounded Noise NK qualification. ADR 0021 carries the contract, permission/provenance matrix, bounds and split. Isolated tools/web-channel supplies one Rust core, thin WASM binding, synthetic native responder, published known answer, adversarial tests and disposable browser harness; no shipping dependency or existing Node TLS change.
Verification: primary browser/Noise/Snow/RFC/AWS sources checked. Native debug/release tests: 10 pass each. Node 24.21.0 and actual Chrome 154.0.8037.93: seven groups each pass, including standard vector, encrypted native round trip, early/replay/tamper/reflection/size refusal and RNG failure. Carrier marker absent in both directions. Complete isolated runner passes: four dependency-guard tests, native/WASM strict Clippy and formatting. Full pinned repository runner also passes default/all-feature Clippy, workspace build/debug/release tests, 29 repository tests and all 17 SDK/process tests. Foundation: 27 phases/36 documents; focused 13 plan-validator tests and diff check pass. On-chain source unchanged, so no local SBF rerun. Hosted CI/review receipt to follow; no HTTP/WS financial workflow or fresh browser Nitro attestation claimed. All four PLAN P21A acceptance criteria remain open.
Unexpected: Snow lacks qualified zeroizing Drop for opaque key/chaining state and has no formal audit; this candidate cannot silently become production crypto. Removed unused Snow std feature (otherwise enables ring/Blake2), pinning only selected suite/entropy and 43 registry packages. Existing risk_report uses journal cut time and needs qualified read-time validation. READ remains own-account scoped. Tool setup used task-local downloads, not global replacement; no AWS, live venue, wallet, container or agent.
Next: publish the bounded contract/core qualification PR using native gh stack; then qualify hardened implementation, independent browser AWS verifier and exact attested context/binding. Implement HTTP/shared SDK next, followed by read projections/WS and joined adversarial acceptance. No parent-readable fallback, fake recovery, triggers/modify/batching, public feed, onboarding or terminal expansion.
PR: none.
Merge: none.

## P21B — Public documentation

Work: scope approved 2026-10-03; implementation not started. Public docs follow the implemented HTTP/WebSocket layer; planned endpoints will not be published as available integrations.
Verification: not run; acceptance in PLAN P21B. No Mintlify project, hosted site or deployment is claimed.
Unexpected: local implementation and actual deployment availability are distinct; public examples must identify the supported environment and qualified release.
Next: after P21A, build the separate docs-site/ tree from checked handlers/schemas/examples and product/security/recovery decisions, then record actual publication if authorized/performed.
PR: none.
Merge: none.

## P21 — Recovery integration

Work: not started; scheduled after HTTP/WebSocket implementation and public docs by the user's 2026-10-03 decision. No recovery criterion is removed.
Verification: not run; acceptance in PLAN P21.
Unexpected: none yet.
Next: recover the implemented service's accepted journal, fence/reconcile/unwind/return funds, finalize backed claims and independently deliver/submit kits with the ordinary API unavailable. Keep explicit hardware/witness doubles and actual local SBF evidence distinct. Update public recovery docs only when this integrated path is tested.
PR: none.
Merge: none.

## P22 — Offline adversarial acceptance

Work: not started.
Verification: not run; acceptance in PLAN P22.
Unexpected: none yet.
Next: after P21/P21A, drive complete deposit/trade/partial-cancel/funding/close/return/payout and recovery paths through actual HTTP/WebSocket SDK/service/controllers with offline external fixtures. Assert claims, exposure, locations, reservations and payment at each boundary; kill/restart service processes and test private-stream gap recovery with reproducible seeds/counterexamples. No live endpoints in ordinary CI.
PR: none.
Merge: none.

## P23 — Live integration qualification

Work: not started.
Verification: not run; acceptance in PLAN P23.
Unexpected: none yet.
Next: prepare the bounded G05 manifest after P20/P22; run the same workflow assertions through actual SDK/relay/Nitro/program/venue rails, verify current capabilities and receipts, and reconcile all cleanup. No mock or historical replay counts as live qualification.
PR: none.
Merge: none.

## P24 — Release safety case

Work: not started.
Verification: not run; acceptance in PLAN P24.
Unexpected: none yet.
Next: assemble evidence/independent reviews and bring G01–G04 production policy and assurance decisions to the user before release authorization.
PR: none.
Merge: none.
