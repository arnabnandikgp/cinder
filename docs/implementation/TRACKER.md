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
| [P21A — HTTP and WebSocket API](PLAN.md#p21a) | closed | Browser/Node HTTP and WebSocket commands and private updates preserve attested confidentiality, owner permissions and one authoritative financial journal. | #47, #48, #50 and [#51](https://github.com/arnabnandikgp/cinder/pull/51), merged `0530374` |
| [P21B — Public documentation](PLAN.md#p21b) | closed | Copy-ready MDX guides and structured method references describe implemented contracts and actual environment availability, not planned endpoints or unverified guarantees. | [#52](https://github.com/arnabnandikgp/cinder/pull/52), merged `1c2c3eb` (tracked delivery receipt only) |
| [P21 — Recovery integration](PLAN.md#p21) | closed | Fenced and reconciled funds back final claims; users obtain claim packages without the ordinary trading API. | [#52](https://github.com/arnabnandikgp/cinder/pull/52), merged `1c2c3eb` |
| [P22 — Offline adversarial acceptance](PLAN.md#p22) | closed | SDK/service-driven trading, funding and recovery workflows preserve accounting under reproducible external fixtures, process faults and races. | [#53](https://github.com/arnabnandikgp/cinder/pull/53), merged `8247d24` |
| [P23 — Live integration qualification](PLAN.md#p23) | in progress | The same workflow assertions run on approved actual Nitro/chain/venue rails; receipts, test exposure, funds and cleanup reconcile. | [#55](https://github.com/arnabnandikgp/cinder/pull/55) → [#56](https://github.com/arnabnandikgp/cinder/pull/56) → [#58](https://github.com/arnabnandikgp/cinder/pull/58) → [#59](https://github.com/arnabnandikgp/cinder/pull/59), ready/open |
| [P24 — Release safety case](PLAN.md#p24) | open | Reviewed evidence, calibrated policy and explicit governance support an honest release decision, not automatic deployment. | — |

Updated 2026-10-07. Only progress values: `open`, `in progress`, `closed`. A blocked
phase stays `in progress` with the reason below. `closed` requires completed PLAN
criteria, recorded tests/review and actual merge; a green/unmerged PR is not closed.
Dependencies and detailed acceptance criteria live in [PLAN](PLAN.md).

## Current handoff

Local continuation is `tee/p23-funding-providers`, based on #59 at
`6ecaf39cbf48d0308bab7e69b283a80d2424482c`. The existing open predecessors remain
unchanged; no new hardware, native/RPC request, wallet use or merge is authorized
by this offline implementation. The prior shipping run below is closed.

### Financial providers — 2026-10-07

Implemented first bounded provider components: deposit transfer linkage requiring
the exact original finalized chain signature, and strict native withdrawal
ACK decoding, original request → batch lookup from retained HTTP-200 replies, and
account/asset/gross/time-bound transfer observation correlation. Restart restores
the mapping without cash postings, hold release or a second send. A missing native
ACK remains unresolved; loss of a customer reply after native ACK retention is a
different, recoverable case. Shared batches are refused rather than assigned by
amount/time. Deposit observations never mint venue credit; partial/whole reported
amounts retain the transit/readiness gates. Advertised/observed fee differences
remain visible for later actual settlement. This also closes an encountered
evidence-framing defect: opaque native
reply/wire/failed-chain bytes are not interpreted as controller/gateway records.
No new ledger or record format, dependency, financial policy or activation exists.

Final focused local checks: locked/offline `cargo test -p cinder-pacifica
--all-features --test funding --test funding_evidence`, in debug and release,
passes 31 funding cases and four evidence groups. The one explicitly allowlisted
child worker is invoked by its parent
kill/restart test, not claimed as an independently run scenario. Pinned
`node scripts/check.mjs` passed all 42 contracts, strict default/all-feature
lint/build, all-feature debug/release and shipping-default tests, and 26 SDK/TLS
cases. The deposit join was added during that run; its final owning-crate
debug/release tests and strict all-feature workspace lint were rerun afterward.
No fresh hardware or hosted CI/review result is claimed for this source.
Next: publish this bounded provider slice;
then prepare authenticated native capture, setup and original-deposit credit
recognition. Finality/completeness, fresh-account bootstrap, lost-native-ACK
reconciliation and actual funding/recovery remain open provider gates, not extra
storage tests. Fresh live financial activity needs its own G05 manifest approval.

P23 is in progress. Shipping continuation is published ready/open as #59 on
`tee/p23-shipping-packs`, based on reviewed #58 (`tee/p23-packed-history`) at
`a1bc5c2bfce92402c22fc8bf53cc36b1cb9f6d5c`, stacked above #56 (`tee/p23-growth-diagnosis`) at
`60eee0c558d800d6613bd1ae72e43bddadd51e13`, above #55 at
`eac1f57c16367d97ad5f32931880c979de522227`. All derive from
merged product trunk `833955f8a4ead02a71eb5ae69a6aebd92bb85091` in `cinder-tee`.
[Live qualification](../operations/live-qualification.md) and
[ADR 0024](../architecture/0024-confidential-chain-ports.md) define the fixed
local checklist and remaining capability gates. The historical pre-review
application checkpoints and receipts remain in the P23 log below.

### Shipping continuation — 2026-10-07

Latest authorization/preflight: the user approved the focused hardware run in
[the next-run contract](P23-NEXT-RUN-PROPOSAL.md), with no native/RPC/financial
activity. All eight hosted jobs and CodeRabbit succeeded at pre-receipt #59 head
`800346b09510f1d7a7ea30c2b335416ffe3df4f9`; no review threads when checked.
Non-root AWS identity, Nitro eligibility, current AMI/price/quota/network metadata
and the final thirteen package hashes reverify. Administrator setup then passed
exact original policy/attachment and fresh-name checks. Initial pre-provisioning
created only two empty tagged roles; AWS explicitly rejected the first tagged
KMS creation because the supplied operator template omitted `kms:TagResource`.
Both roles were removed after ownership checks; independent cleanup verified
absent roles/buckets/table/test volumes, zero instances and zero recorded keys.
No EIF, runtime credential, key release, native/RPC request or financial activity
occurred. Fresh `p23-20261007-pack1` templates and receipts are local under ignored
`work/experiments/p23-live/packed-permissions/` and `run-pack1/`. The updated
operator JSON adds scoped tagging, not decryption; runtime boundary is unchanged.
The administrator corrected the existing operator default policy; independent
exact-policy/attachment/fresh-name checks pass. A fresh `run-pack1b` ledger
(`CinderRun=pack1b`) preserves the rejected, closed `run-pack1` unchanged.
The same approved namespace's names were independently absent and no prior key,
credential, witness head, ciphertext or host existed to reuse. Six new keys,
two stores, table and bounded roles were created. One `c6g.large` host was launched,
with encrypted auto-deleting root, operator-only SSH and verified four-hour
automatic shutdown. All thirteen uploaded artifacts match the source register.
The fresh version-3/six-role manifest keeps native/funding/trading gates off;
independent first-EIF hash/CRC/entrypoint/nonzero PCR checks and all six exact
key/role policy readbacks pass. Actual parent-role witness-read and plaintext
KMS-decryption denials pass. Boots 1/2 are non-debug (`Flags=NONE`). Actual Node
and Chrome HTTP/WS cuts 2/8/32/64 pass one/two watches and parallel own zero-credit
reads; recorded parallel latencies are 840–2,704 ms. Actual READ grants grow the
accepted history, with head-64 growth replies at 2,147–2,606 ms. Both clients pass
attestation pin/replay refusal, own-account READ/revocation and natural grant
expiry. A deliberately lost reply reconciles its original pre-exposure ID/digest
without resend. Exact process restart preserves head 64, account epoch and
original receipts. The epoch fence advances authority to epoch 2 without changing
the accepted head and refuses private reads. Fresh-stream old-pack loss,
corruption, independently tested exact-object repair denial and fault replay pass;
all eighteen accepted receipts survive restart and the failed original remains
not-found, without resend. Witness loss refuses reads with head 20 unchanged.
One-PUT-per-replica quota accepts only genesis (head 0), with no API initialization
or extra tail. Natural 90-second boot lease refuses after a healthy baseline,
preserving head 2 while the parent and all six keys remain live. Natural witness
credential expiry also refuses private reads with unchanged head 2, enabled keys
and valid parent credentials. All fixed application cells pass and all owned
workers are stopped. The ciphertext archive retains and independently rehashes
188 objects (1,183,396 bytes) and five strong final heads. Owned cleanup was
independently verified at 10:30:46 UTC: host terminated, test EBS/stores/witness/
roles/SSH/security group absent, six keys PendingDeletion, existing setup policies
preserved. Local relay/tunnel exited. This invocation is closed.
[The sanitized hardware receipt](P23-PACKS-HARDWARE.md)
records the exact release, five profiles, seven boots, latencies, faults and
traffic boundaries, not production capacity or P23 financial acceptance.
Final parent HTTPS counts: S3 580/379, witness 1,134, KMS 42, venue/RPC zero.
Conservative setup/runtime/archive/cleanup totals (including the rejected first
attempt and manual diagnostic reads) are KMS 242, S3 1,392 and DynamoDB 1,188,
within approved caps. No pending or uncertain resource mutation remains.
Startup DNS/address-selection metadata exists; zero native/RPC API requests
does not mean zero DNS. Private receipts/scripts remain local and ignored.
Local operator preparation passes the unmodified trusted version-3 preparer
with all six roles, bounded history and financial gates off; it rejects a
257-record policy. It is not hardware or complete cloud-fault qualification.
Do not overwrite/reuse closed policies/resources, assume that unreadable means
absent, or use the untracked user preparation example. The cited hosted CI
receipt applies only to its exact committed head, not this documentation update.
Final docs-only verification: pinned `node scripts/check.mjs --group=contracts`
passes 42/42 contracts, plan and workspace-boundary checks; `git diff --check`
passes. Local operator header/read-oracle tests pass 14/14, not extra hardware
acceptance cells. No source change or new full Rust/SBF qualification is claimed.
Next: prepare authenticated native setup/credit/withdrawal providers and their
local tests. A fresh bounded approval
is required for the actual funding round trip; this read-only run grants none.
No PR is merged and P23 remains in progress.

Historical local authorization/receipts follow:

User authorization: finish local shipping integration/qualification and **stop
before AWS/Nitro tests**. No new cloud/venue/RPC/wallet activity or merge follows.
Current implementation binds version-3 retained S3 packs, closed resource limits,
six-role/chain release and actual loaded application policy. Versions 1/2 retain
their bytes/digests and frame layout; no format fallback or automatic migration.
Original full history/AEAD/witness/CAS/repair/fresh-read guarantees remain.
Record/history/count and per-replica per-boot PUT/byte caps fail closed; whole-run
quotas and changed-image hardware qualification are separately required.
[ADR 0026](../architecture/0026-retained-history-packs.md) describes exact bounds
and limitations; this is not production capacity/retention calibration.

[Native evidence components](P23-NATIVE-EVIDENCE.md) strictly decode documented
setup/loan/transfer fields. They do not authenticate source, establish complete
cuts or create financial completion certificates. Lost-ACK UUID/batch binding,
final credit/debit, execution/funding completeness, fresh-account bootstrap and
actual recovery remain named open capabilities. Trading/funding stay forbidden.
The historical M1 heuristic/exception is not silently imported.

Existing stack review disposition: #56 wording and #58 competing-history race
are fixed at `a1bc5c2`; both threads replied/resolved, no open threads in #55/#56/#58
when inspected. All eight hosted jobs and CodeRabbit succeed at that exact #58
head. These are parent receipts, not CI for the new continuation.

Local receipts: the full pinned `node scripts/check.mjs` passes at implementation
checkpoint `b3aa47480fcb940a3837e61bc39f1653bb777005`: all 42 contracts, strict
default/all-feature lint/build, all-feature debug/release, shipping-default debug
and all 26 SDK/TLS tests. Journal's 13 unit cases include policy bounds, 8-MiB
full-size archive/replay/append and the reviewed race. All five release tests
include real trusted-preparation subprocesses for manifest versions 1/2/3.
The final narrow wire correction at `0845ca67b93134783156d0ce8ce58f0f4c8b9b47`
retains a zero-net withdrawal when its fee consumes the gross amount; its three
evidence groups pass debug/release and final full-workspace lint/build passes.
It does not connect that parser to Runtime or enable any financial capability.

The complete `node tools/web-channel/check.mjs` passes: each actual Node/Chrome
client has 14 core/verifier, 26 HTTP/WS SDK process and nine C4 Runtime groups;
all four shipping-bounded packed growth cells pass. Frozen Runtime fixture hash:
`013540d47b65eeef0cef6a734e9c79ee1f070a901a9d660e7004b267ddfbce05`.
All cells reach cuts 2/8/32/64 with 100-ms replica delay, preserve accepted head65
and original-ID/digest receipt after lost reply, restart and epoch revocation.
Ledger remains zero-credit/no-position; there are no native financial actions.

| Client / carrier | Grant reaching head64 | Largest growth grant | Peak fixture RSS across growth/replay |
| --- | --- | --- | --- |
| Node / HTTP | 1,766 ms | 1,814 ms | 12,894,208 bytes |
| Node / WebSocket | 1,866 ms | 1,971 ms | 12,042,240 bytes |
| Chrome / HTTP | 1,761 ms | 1,801 ms | 12,173,312 bytes |
| Chrome / WebSocket | 1,777 ms | 1,934 ms | 12,910,592 bytes |

The direct full-size backend test alone peaks at 52,068,352 bytes RSS (2.96s),
not compiler RSS or a full decoded financial Runtime/Nitro capacity benchmark.
Full default vault runner passes **59/59, none skipped**, in 466.228s, including
actual compiled SBF normal/recovery HTTP/WS workflows in offline Surfpool.
These retain external fixture evidence limits; they are not testnet transactions.
Anchor 1.2.0 temporary binary matches published SHA256
`986d99392a520dfc50b63caf4eda9b9d7619c78ffb53fc57b6f8677c60b6d27e`;
Node 24.21.0 / Rust 1.97.1 / bindgen 0.2.129 / Chrome 154.0.8037.98 remain pinned.
Host lint initially caught a feature guard error, fixed without suppressions;
SBF initially stopped before tests at missing tool prerequisites. No global
installation changed. Hardware-only ignored cases were not invoked.

Exact committed source export independently passes all 42 contracts and
plan/workspace checks (27 phases/47 documents), with no research or user files.
The source-only ARM package at `b3aa474` passes reproducibility and ordinary-host
boot refusal. Its copied thirteen hashes/rootfs equality verify; temporary
stopped-container copying failed, so the generated container was removed and
the package rebuilt/exported while running. No user artifact was deleted.
Final ARM source `0845ca67b93134783156d0ce8ce58f0f4c8b9b47`, tree
`0ac94f4ff6b92abf958f33bff5d63e3640e265f5`, passes default-feature release,
clean application rebuild/byte-identical comparison and ordinary-host refusal
in Apple container 1.4.1 / immutable tool image
`33107aa751a9d69fb65acb26474fe3954dd30f8580b4150b4a63daf58bac434a`.
Networking disabled, source/vendor read-only, build outputs isolated; no NSM,
research, wallets or AWS configuration mounted. Generated package retained at
`/private/tmp/cinder-p23-shipping-arm.G1JsgP/artifacts/final-bundle`.
All thirteen copied hashes and rootfs executable equality verify. Enclave SHA256
`653e1643770084ad58b1120a46d21e8c40032b04a40734b9909ee2ed75474209`;
package register SHA256
`cea81afe00e1bfaa68ccd98ad6c8da3b7541de2cac13533fe494280b63c48410`.
This supersedes the earlier package for a new run; it is not EIF/PCR or hardware
evidence. Both task-created build containers exited/removed; shared images and
builder remain untouched. Raw offline logs/package are temporary/reproducible;
code/tests/run contracts are tracked.

Publication: ready/open [#59](https://github.com/arnabnandikgp/cinder/pull/59),
`tee/p23-shipping-packs` → `tee/p23-packed-history`, initial evidence head
`773217fb357f4522ca42bd1b84599f171a1dd2e4`. Native stack #57 is
#55 → #56 → #58 → #59; all three parent branch heads are unchanged.
Hosted CI/review for #59 are newly pending, not inherited from parent/local
results. This publication-only successor records actual PR/base and passes the
contracts/workspace/whitespace guards; its own new-head checks remain separate.
No PR is merged. User-owned untracked files and unrelated stashes remain out
of the commit/export. The next action is review/fresh-manifest preparation at
the AWS boundary, not launch: no new AWS/native/RPC activity occurred. Storage
local qualification is complete; the P23 financial provider/lifecycle gates
listed above remain open and no production capacity/retention policy is approved.

[The next-run proposal](P23-NEXT-RUN-PROPOSAL.md) is the handoff at the AWS
boundary, not permission to launch or activate financial capabilities. Keep P23
in progress. Do not reuse closed C5 resources/policies or fabricate live receipts.

### Offline growth continuation — 2026-10-07

The user approved local growth diagnosis and an independent financial-gap map,
not another AWS run or financial activation. The focused follow-up adds only
test instrumentation/regressions and tracked planning evidence; shipping source,
storage format, deadlines, repair/freshness and gates are unchanged.

[Growth diagnosis](P23-GROWTH-DIAGNOSIS.md) derives and measures healthy append's
3N+2 replica GETs / two PUTs. The existing count regression now covers heads
0/8/24/32/50/64. A real Runtime/API/AEAD/Replicated test with unique READ grants
reaches head 50, then injected 100-ms replica calls take 16,281 ms for the next
grant (155 GETs, two PUTs, seven witness reads, one CAS). Original-ID lookup and
explicit encrypted replay retain its COMPLETE receipt/head 51 and all 51
transactions; financial ledger unchanged, no orders/attempts. The twelve
contention regressions pass. This is a synthetic latency counterexample, not
an actual SDK timeout, a fresh process boot, measured AWS timing or cut-64 pass.

[Financial continuation map](P23-FINANCIAL-GATES.md) connects setup, owner deposit,
native credit/payment, terminal execution cuts, exact funding and recovery gaps
to current code and refusal behavior. It does not reopen approved economics,
import M1/M2 authority, flip activation booleans or add implementation phases.

At diagnosis publication, the next user review was the byte-bounded encrypted
history pack design/prototype. The user has now approved its **local** development;
the candidate and remaining promotion gates are recorded below. No shipping
pack/checkpoint/retention replacement is approved. An earlier
GET cannot silently substitute for the later repair check. Specify format/head
binding, bounds, interrupted writes, archive/replay and copy-repair behavior
before shipping a storage change; then test locally and propose a fresh bounded
hardware requalification. C5 cut 64 and all actual financial lifecycle gates
remain open. Do not reuse closed run-2 or merge automatically.

Local owning-layer verification: final service library debug/release each
98 passes / seven explicitly ignored Nitro cases; strict feature-enabled service
Clippy, Rustfmt and diff whitespace checks pass. The pinned contracts group
passes all 42 guards plus plan/workspace checks. The strengthened reproduction
also checks a fresh API record cache after replay. No hardware test was invoked;
no production/SBF/browser/package sources changed, so their prior receipts are
not claimed as new runs. Diagnostic source
`088be95661b374770b7d6885ef0798b04dccf93d` is published ready for review as
[#56](https://github.com/arnabnandikgp/cinder/pull/56), base
`tee/p23-live-qualification` at `eac1f57c16367d97ad5f32931880c979de522227`.
Native GitHub stack #57 contains #55 → #56. Parent #55's hosted checks
are green at the unchanged parent head; #56's hosted
CI/review is new/pending, not inherited from that result. This publication receipt
changes only TRACKER. Neither PR is merged; the subsequent user-approved local
prototype does not authorize another AWS or financial run.

### Retained-pack local prototype — 2026-10-07

Scope: [ADR 0026](../architecture/0026-retained-history-packs.md), explicit local
candidate only. Canonical groups of 16 preserve every original frame, AEAD,
witness head, configuration and operation identity. New-tail replication and
later both-copy checks/repair of ALL old required packs precede CAS. Historical
copy loss during new writes is tested; failed repair leaves the old head accepted.
No checkpoint/pruning, old-GET reuse, timeout change, retry, native calls, AWS
resources or economic activation. Shipping Loaded::open still selects Replicated.

Healthy existing head 50: 14 GETs/two PUTs instead of 155/two; existing head 64:
17/two instead of 197/two. Full history bytes still grow. Largest legal pack is
approximately 16 MiB, beyond shipping S3's frame-body limit. Immutable tail
versions duplicate prefixes; full-size memory, storage quota/orphans, cloud
transport and manifest/migration qualification are explicitly NOT closed.

Local fixture selection persists a format marker and refuses switching layout
on reopen. Full archive import requires the exact independent current head and
does not CAS/reset it. Real AEAD migration/reopen tests retain original receipts;
old-epoch writers and wrong-key/layout/owner reopens refuse.
Sharing the unchanged archive codec exposed an inherited one-byte header-limit
error; its bound now derives from actual format length (118, not literal 117),
with a maximum legal 4,096-frame/64-MiB archive regression. Actual Node/Chrome
HTTP/WS candidate suite is independently wired into the browser CI runner.

Local verification passes:

- Full pinned `node scripts/check.mjs`: contracts/plan/workspace, strict default
  and all-feature lint/build, all-feature debug/release and default Rust profiles,
  SDK/relay checks. After the isolated archive-bound correction, the full journal
  debug/release suites and maximum legal archive regression were repeated.
  Nine pack unit cases and three real AEAD/file/archive integration cases pass;
  final strict owning-layer Clippy, Rustfmt, TypeScript and whitespace checks pass.
- Contracts group passes all 42 guards after the CI/docs changes.
- Existing actual Node/Chrome C4 matrix passes 24 cells/nine groups per client.
  This earlier regression artifact predates the isolated archive-bound correction;
  its Runtime/core/carrier sources were not changed by that correction.
- Final `node tools/web-channel/check.mjs --packed-only` builds locked native
  fixture/verifier and WASM prerequisites and passes all four actual Node/Chrome
  HTTP/WS cells. Frozen final fixture SHA256:
  `faf74f6a203fa0ddb333fc2db9d222f4790b5c6d738b5679e04c54e5b7eb25ff`.
  Each grows from initialized head1 to64 with 100-ms replica calls, reads at
  2/8/32/64, then two watches plus 50-ms witness delay at the deliberately lost
  reply. Original-ID/digest lookup and fresh process replay retain COMPLETE at65;
  no resend, zero customer credit/positions, current-epoch revocation passes.

| Final client / grant carrier | Grant reaching head64 | Largest grant in growth | Peak Runtime RSS across growth and replay |
| --- | --- | --- | --- |
| Node 24.21.0 / HTTP | 1,701 ms | 1,733 ms | 12,615,680 bytes |
| Node 24.21.0 / WebSocket | 1,698 ms | 1,730 ms | 12,484,608 bytes |
| Chrome 154.0.8037.98 / HTTP | 1,704 ms | 1,738 ms | 12,632,064 bytes |
| Chrome 154.0.8037.98 / WebSocket | 1,694 ms | 1,729 ms | 12,664,832 bytes |

All are small-record synthetic loopback measurements, below unchanged 15-second
reply and 128-MiB fixture RSS limits; not full-size pack/cloud/Nitro capacity
calibration. Both before/after-restart boots are measured. The added CI growth
job has its own cached dependencies and unchanged fifteen-minute job limit;
the standard browser/C4 job retains its name and coverage. Default local browser
runner executes both. Test/harness sources are tracked; local raw logs remain
temporary, not a dependency on ignored `work/`.

This does not qualify the unpassed hardware cut or financial gates. Next is
review of the candidate safety/resource/migration contract, then bounded shipping
port work and a separately proposed exact-source hardware run. No automatic merge.

Publication: implementation source `0f66ed83b901a03914c41136f97c17b3dd414523`
is ready for review as [#58](https://github.com/arnabnandikgp/cinder/pull/58),
base `tee/p23-growth-diagnosis` at `60eee0c558d800d6613bd1ae72e43bddadd51e13`.
Native stack #57 is #55 → #56 → #58; both parent heads are unchanged.
A clean index source export without ignored research/user files independently
passes all 42 contracts and plan/workspace guards. Hosted CI/review are newly
pending on the publication/bookkeeping head, not inherited from local or parent
receipts. This successor changes only this publication record. Nothing is merged;
no AWS/native activity or shipping-storage selection follows from publication.

### Latest remediation implementation — 2026-10-07

Pack review continuation: the #58 same-epoch competing-writer finding is valid.
Append now compares the freshly loaded tail with the original expected head
before any pack writes. A focused same-frame race at heads 14/15/49 fails on
the old implementation (two unwanted writes) and passes after the fix, retaining
both accepted replicas and the original full replay. The full journal unit
suite and release-mode race test pass; all 42 contracts and plan/workspace
checks pass. The #56 wording finding is also valid: its continuation now states
that existing Node/browser SDK qualification is in scope, while a product
terminal, market-data UI and self-service UX are separate follow-ups. No storage
format, deadline, financial capability or AWS authority changes in this review
correction. Fresh-head hosted checks remain separate; nothing is merged.

Review continuation after published `e50dc9d84012711d825a3a9b9821b540611696a4`:
the user authorized both CodeRabbit and Codex addressal and approved the fresh
`C5-RUN-PROPOSAL.md`. Three findings were relevant: ordinary read
Busy/Raced must return private Unavailable rather than close; finalized ineligible
deposit locators must not repeatedly spend RPC budget; HTTP reply waiting must
retain a separate five-second frame/body assembly bound. Focused regressions
reproduce all three before the fixes. Ordinary prepared candidates distinguish
periodic delivery; only periodic Busy/Raced skips. Final generation/auth/time,
connection/boot checks, fatal fences and no retries remain unchanged. The chain
port retains bounded boot-local rejection flags for immutable configured locators,
not credit or a new financial journal; pending/port/code/account errors are not
terminal-cached. SDK body and opaque upstream-frame timers are absolute and
cleared on completion/close, independently of the 15-second application wait.

Final source checkpoint `d2bf4b69968def0fd10ab8b70a056c1aea71f2f3`, tree
`ce1735a7cfd42c8d84ffb57437b16409b443a8c7`: complete pinned
`node scripts/check.mjs` passes (42 contract guards; default/all-feature lint and
build; all-feature debug/release and shipping-default regressions; service 97
unit passes plus seven explicitly ignored hardware cases in feature-enabled
profiles; 15 relay and 26 SDK tests). Actual Node 24.21.0 and Chrome
154.0.8037.98 each pass 24 joined Runtime cells/nine groups and all 26 existing
HTTP/WS carrier groups. Final-release/post-CAS races now verify encrypted
Unavailable followed by a successful read on the SAME channel, without retry.
Frozen fixture ELF SHA-256:
`95b9e07b2e0457b110f8ef84fbd027f2fbc968dee6c7791e1488320b76f97496`.
400-ms independent reads complete in 411–427 ms Node / 407–419 ms Chrome;
small local process peak RSS is 10.5–15.6 MiB, not production calibration.

The same committed source passes the default-feature ARM package/rebuild/host
refusal gate: enclave SHA-256
`e69499c1e4f8444828d681c42b10580d05f50364d1de1d17f2f59665548c57f6`;
register `2530ce282b1cad9453f538e108f6a1e07e2c3f74c66dca981838dc6fcefe9cb7`.
All 13 copied binary/library hashes and rootfs equality verify on macOS.
Tool-image/vendor/lock identities and isolation are unchanged from the package
receipt below; owned container `cinder-p23-c5-review-20261007` is absent. New
bundle: `/private/tmp/cinder-c5-review-arm64.0PsmU6/artifacts/bundle`; local
logs: `c5-review-{check,runtime,http}.log` in
`/private/tmp/cinder-c3-check.DFahHx` and `package.log` in the new build root.
Only this receipt follows the tested application source. Program/SBF sources
are unchanged; no new SBF or hardware result is claimed.

The fixes and receipt successor are published as `4849a83749e3e646b960f029ef99c33c2327da3c`
on #55; all three CodeRabbit/Codex findings are replied to and resolved. Hosted
Rust regressions/lint/build, SDK/relay, browser, contract inventory, ARM package,
Anchor/SBF/Surfpool and CodeRabbit checks all pass on that head. This is not a
hardware or merge receipt.

The user renewed the intended non-root `cinder_new` login and created the fresh
policies. Both C5 v1 documents independently match the local templates; preflight
confirms the fresh roles/buckets/table/SSH/SG names are unused. Current Amazon-owned ARM AMI,
Nitro-capable `c6g.large`, regional vCPU quota, account-owned network and price
checks pass. Source-built LOCAL profile/V2/preparer checks retain all three
gates false and six role capsules below 4 KiB. Fresh local operator tools live
in ignored `work/experiments/p23-live/c5-tools/`; the old run-1 tools/ledger are
unchanged. The public-only package verifies 13 artifact hashes; six harness
oracle tests/browser bundling pass, not NSM evidence.

Approved C5 has now launched its ONE fresh host in `p23-20261007-ro2`, at
2026-10-07 03:13:48 UTC; instance `i-0d2dd891264958c33`. Termination permission
passes an actual non-mutating dry run on this owned ID; the four-hour
terminate-on-shutdown safeguard precedes host package setup. Exact-source
image/manifest/PCR review remains mandatory before recipient-bound key release.
Trading, funding, native scheduler and financial activity remain disabled.
The hardware schedule fixes the existing cases and includes a recorded limit:
the actual service requires API/funds initialization (sequence 2), so sequence
1 cannot be an initialized live cut. Measure/report that minimal actual head
honestly; grow only through real grants/revokes, to at most sequence 64 INCLUDING
initialization, not 64 API operations plus hidden setup. No extra test cases or
deadline extension is authorized by that limitation.

Completed hardware checkpoints: independently downloaded application EIF
`d7f2d3d855111cf9201d9f7c3b5c397ea6857b1177ab7fe1f95479064a2bbf36`
passes CRC/version/entrypoint and exact PCR/context pinning; six-purpose NSM
release and parent plaintext-decrypt/witness denial pass. Actual Node/Chrome
minimal initialized head 2 and exact head 8/32 pass one/two-watch concurrent
HTTP/WS reads, plus same-socket grant, READ-agent revocation and natural expiry.
Full-history growth hits an unavailable reply at head 50; original-ID lookup
confirms its grant committed as head 51. No resend or deadline increase. The
64-record cut is unpassed, not a calibrated maximum-capacity claim. A separate
local harness session-expiry refusal is preserved as pre-sign/no-wire, not an
application pass/failure; new independent commands use fresh attestation.

Actual encrypted restart preserves all 49 operation IDs/digests, epoch 3 and
head/hash 51. Replica loss closes the retained revocation reply; strong head and
post-restart original-ID reconciliation prove it was not accepted and all prior
state remains unchanged. Witness-relay loss refuses private reads. A healthy
restored boot then conditionally advances witness epoch 1→2 without changing its
head/hash, and the old worker refuses private reads. The separate 90-second
application lease expires naturally: healthy baseline at 03:50:50.777 UTC,
refusal/absent enclave at 03:52:33.593, with parent supervisor still alive and
witness credentials valid until 04:47:43. No parent kill, clock change or
credential renewal causes that result. The separate fresh 900-second witness STS
case passes: healthy baseline at 03:57:10.511 UTC, real expiry at 04:08:46,
post-expiry private connection/read refusal and absent enclave at 04:09:11 with
parent supervisor still alive and no budget stop. The 40-minute application
lease/one-hour parent credentials, no clock manipulation and no renewal keep
this distinct from lease/parent termination. This establishes the real STS
expiry/freshness refusal, not a captured AWS ExpiredToken response. Six total
boots; final runtime metadata counts are S3 2,936/1,500, DDB 778, KMS 36 and
native/RPC 0/0. The owned service is stopped. All 116 ciphertext objects (93,238
bytes) and three strong heads are archived with checked sizes, SHA-256 and private
local permissions; original head remains epoch 2/sequence 51, lease/expiry heads
epoch 1/sequence 2. Inventory SHA-256:
`15a2738ef0a64f69d7e174d5c8b13eb3d1c7fd6504483d635da832c50a8f8e69`.
Independent teardown verifies at 04:19:55.868 UTC: exact host terminated
(confirmed 04:15:24.799, about 62 minutes after launch), tagged root EBS absent,
both buckets/table/roles/SG/SSH absent, six KMS keys PendingDeletion with seven-day
window, administrator policies and historical resources preserved. No uncertain
resource mutations remain; owned local tunnel/relay are absent. Final operator
calls: STS 35, IAM 48, EC2 27, DDB 35, S3 254, KMS 111. Combined runtime/operator
counts: KMS 147, S3 4,690, DDB 813, below approved caps. Billing is delayed; the
approved $5 cap is not an observed final invoice. No financial activity or
application-source/deadline change. This invocation is closed/read-only; never
reuse its namespace or cleanup ledger.

The lease-only EIF is
`e7f71a8ba42b6af6c57fe2dd396cb1a63076ffc9295e9cb97e05a36215c7b77a`;
STS-expiry EIF is
`b81887bad82bb7d00155c6d54b513c554af37bfd87d592dd5f53e01978bd4a67`.
Both are separately downloaded/inspected and exactly pinned to six KMS purposes
before release; their public input manifests differ from the application profile,
not their reviewed executable source. Exact PCR0/1/2, manifests, ciphertext
capsules, SDK pins and private original-operation receipts remain locally under
ignored `work/experiments/p23-live/run-2/`. Operator scripts are preserved in
`c5-tools/`, not shipped or treated as application/hardware proof.

The sanitized receipt is published as `eac1f57`; local growth diagnosis is now
recorded in the continuation above. A new bounded remediation/hardware proposal
needs review; do not force a pass through longer deadlines, resend the accepted
original or reuse this closed invocation. C5's
64-record gate remains unpassed, P23 financial/native/recovery workflows remain
separate, and no automatic merge or financial activation is authorized.

This hardware receipt successor changes only PLAN/TRACKER/EVIDENCE and the C5
status row. The final pinned contracts group passes all 42 guards plus the plan
and workspace boundaries; `git diff --check` passes. Prior full source/ARM/client
results remain tied to `d2bf4b6`; new receipt-head CI is separate and pending.
Private evidence/operator tools and unrelated untracked files are excluded.

The user approved the broad remediation recommendations. Historical initial
implementation checkpoints below are superseded by the published reviewed
source/CI/package and actual partial hardware handoff above. Work follows the
[C1–C5 plan](P23-REMEDIATION.md): shared interpretation/pure authorization first,
then accepted publication and bounded scheduling, a fixed joined local matrix,
and separately authorized changed-source hardware qualification. C1 is committed
locally as `5b77b11cc0101430f432c236e415577d83622588`. C2 is locally committed as
`9feeee8a4a59e6abb7baec78c9600226192763ea`; C3 is locally committed as
`18f3607030b87f9e44e7312c1d08c12871197c9f`. C4's real-runtime fixture/matrix is
implemented and passes final local verification, checkpointed as
`b38849cf93761f380e195a2731698d862ea84f4d`, with exact handoff below. C5's
changed-source ARM package passes offline; the fresh hardware checkpoint and
remaining/unpassed gates are recorded above. Relevant reviews are resolved;
merge remains unauthorized, so none of these slices is
closed merely because its local tests pass.
Current durability/repair checks are retained for bounded dev/test;
a production segment/checkpoint/retention contract is not implicitly approved.

Historical pre-review publication target was the same ready/open one-layer PR #55, based on
`product/tee-v1` at `833955f8a4ead02a71eb5ae69a6aebd92bb85091`. Previous
published head was `6dff3731f365463ad24c1b5fcd2cb82f19d76c86`; its green checks
do not qualify this remediation. The new tip is the receipt-only/CI-prerequisite
successor directly after `354c5323d1ea2dfce50265bc9a2b0bf0149eae0d`, containing
this handoff; the actual pushed head and base are recorded in the PR description.
Tested application/package source remains `b38849cf93761f380e195a2731698d862ea84f4d`.
New-head hosted CI and CodeRabbit review remain pending. No AWS, financial
activity or merge was performed; publishing this verified checkpoint does not
authorize hardware, financial activation or merge.
Preserve unrelated `docs/user-journey.md` and `stays/`; the local qualification
preparation example is intentionally not part of this remediation checkpoint.

C1 implementation: one paired accepted-transaction/receipt traversal replaces
nested receipt searches in API, funds-controller and recovery interpretation.
The API reuses an immutable exact-head/contract index with account-scoped operation
and epoch-scoped grant maps. Shared pure authorization preserves signature,
freshness, time/epoch and grant-before-ID ordering. Index building/deallocation
stays outside its short pointer latch. No balances, freshness result, permission,
new wire, persistence format or dependency is cached/introduced. Runtime locks,
NSM/witness checks, controller I/O, replica repair and WS polling are unchanged.

Six new real-AEAD/SQLite regressions plus one all-family/paging/replay comparison
cover warm reuse, a fresh check per request, poison/unknown acceptance, contract/
head changes, expiry/revoke, authorization before lookup, account-scoped IDs and
malformed history. Final local source passes locked/offline API tests (35),
Pacifica tests (81, with the named killed-child worker remaining parent-invoked),
and service tests (83 unit + eight integration; seven named Nitro cases remain
gated). The full all-feature workspace also passed before the final paired funds/
recovery and paging refinement; those final affected crates were then rerun.
Strict all-target/all-feature workspace Clippy, formatting and diff checks pass.
Pinned Node 24.21.0 contracts/inventory (42), workspace boundaries, relay (14) and
private SDK (24) pass. Actual Node/Chrome HTTP/WebSocket suites each pass all 26
groups against synthetic loopback fixtures. These fixture results do not join
slow real Runtime/witness I/O or qualify the unresolved live WS gap.

#### C2 implementation and next handoff

Central `journal::read` publication shares immutable accepted State and individual
retained transaction/receipt/book-change records; bounded non-cloneable tickets
own at most four read slots. Boot attaches only after initialization and writer
freshness. All central commits publish after acceptance/postchecks; rejected
controls advance the accepted head, exact retries do not. Writer/polling paths
cannot bypass this publication. Panic/uncertain write, failed freshness/replay,
poison, dropped journal and runtime stop invalidate outstanding tickets. Reload
cannot revive an attached read boot. No wire, ledger revision, stored authority,
new dependency or persistence format changes.

Read/View/Operation take independent fresh Dynamo strong reads without acquiring
Runtime.active. The immutable cloud client shares finite credentials, not a
network mutex, cached response or CAS authority in its read-only port. Completion
rechecks finite credentials and existing elapsed/signed-time limits. Existing
API envelope/epoch/grant authorization and exact account/history projection are
shared across the writer and sealed read source. The final gate uses a fresh
signed clock outside the pointer latch, a 250-ms entire processing budget,
conservative expiry revalidation, the same generation/head and the actual sticky
runtime stop flag. NSM, signature checks, history/index work, encoding and network
I/O never run under that latch. Known exact newer local heads refuse without
poisoning; unknown/backward heads, wrong epoch or failed witness fence. No TTL,
cached freshness, read retry or longer application-reply deadline.

Eight new AEAD/SQLite publication regressions plus a latch-poison unit case cover
pre-CAS/post-CAS timing, accepted/rejected/duplicate records, uncertainty/panics,
wrong/missing/backward witness, final deadline/boot stop, journal drop and shared
bounded retention. The ten-family/paging/View/Operation golden comparison now
also covers independent reads before and after AEAD replay. One new API unit
case covers final grant expiry, read-only mutation exclusion and exact contract
binding. Four new direct Runtime cases cover fresh calls without writer backend
I/O, slow writes, parallel reads, revoke during I/O, request/boot expiry, bounded
final signed-clock work and panic. One cloud completion regression checks STS
safety/clock/elapsed boundaries. Synthetic timing/authority ports remain explicit;
these are not live AWS, deployed funding or queued WebSocket-delivery receipts.

Final C2 local verification — 2026-10-07: the implementation commit containing
this receipt follows `5b77b11cc0101430f432c236e415577d83622588`. Pinned Rust 1.97.1 /
Node 24.21.0 on Apple silicon pass complete locked/offline `node scripts/check.mjs`:
42 contracts/inventory tests, dependency boundaries, formatting, shipping/default
and all-feature all-target Clippy/builds, all-feature debug/release and shipping-
default Rust suites, 14 relay tests and 24 private SDK tests. API has 36 passing
tests; the all-feature service has 88 passing unit and eight integration tests,
with seven named Nitro tests still explicitly gated. The new publication suite
passes all eight cases and its separate latch-poison unit case. Named killed-child
helpers remain parent-invoked, not silently waived. Actual Node and Chrome
154.0.8037.98 each additionally pass all 26 existing HTTP/WebSocket/private-SDK
process groups against synthetic loopback fixtures. These carrier regressions do
not join slow Runtime/dependency I/O and are not C4/live qualification.

One unplanned harness constraint: SQLite's all-feature fault callback is not
Send. The publication race test keeps SQLite/writer on its owning thread and
moves only the read capability, rather than weakening that hook or adding unsafe
Send. An initially undersized disposable credential fixture was corrected to
satisfy existing validation; the final full run, not that failed run, is the
success receipt. Logs: `/private/tmp/cinder-c2-tools.NhSWw8/check-verified.log` and
`http.log`. Regression source is tracked; those logs/tool downloads are temporary,
not a new ignored experiment or required private dependency. No Linux/SBF/ARM/
AWS/venue/funded test was run for this source, and previous hardware/hosted
success is not inherited as new-source evidence.

The original C2 handoff was bounded external prepare/I/O/completion and
connection-owned scheduling; C3's implementation receipt follows. The existing
full-history append/repair remains unchanged and is still a finite/quadratic
dev-test limitation. No C3–C5/P23 closure, push, merge, financial activity or new
AWS run is claimed.

#### C3 implementation and next handoff

The shipping Runtime now prepares opaque unreleased Read/View/Operation replies.
TLS and web carriers perform their time checks, then consume the exact owning
boot/connection's current-generation/auth/signed-time release gate immediately
before encryption/write. Web connections have one scoped joined periodic worker,
one job and completion slot, one coalesced wake slot and their sole cipher/socket
owner. Already-arrived ingress is serviced before starting another poll; each lap
also drains completion, so continuous commands cannot starve authorization/expiry.
There is no detached worker, released-response queue, automatic read/mutation
retry, new dependency or increased transport deadline. Four global read tickets,
eight connections and existing directional/session/frame limits remain unchanged.

Native observation/RPC collection now drops the mutation guard during ordinary
external I/O. The writer durably reserves the exact request, then rejoins fresh
authority and its original reservation on completion. A changed logical head is
not a reason to omit a late 429 or verified original deposit: commit at the current
cut, retain original observation times, consume/credit once. Unknown exchanges
retain spent credits without resend. Backend history/durability work stays under
its single writer; manifest-forbidden trading/fund-moving branches are preserved,
not newly activated or claimed as unlocked financial-I/O qualification.

Inactive tick now checks qualified signed boot time and sticky publication fencing
without a cloud-history load. This intentionally does not proactively detect idle
witness/STS loss; every actual command/read must still perform its own fresh check.
Four Runtime cases test queued release/revoke/expiry/boot/connection, global slot
bounds, inactive tick, and concurrent grant/read during stalled native observation.
One actual Rust Noise/socket case (two close variants) tests command delivery
during a stalled synthetic poll, cipher/update ordering and joined close. Two
Pacifica phase tests retain late timestamps and exact reservation/config binding;
one chain test collects original evidence, advances the writer, credits once and
deduplicates without repoll. These are local synthetic dependencies, not AWS.

Unplanned work: replace a blocking completion wait with coalesced wake notification
after reproducing cumulative frame timeout; box the large owned deposit-evidence
variant for strict Clippy. Neither required a wire/deadline/schema change. The
previous disposable Node toolchain was absent on resume; a fresh official pinned
24.21.0 archive was checksum-verified rather than using system Node 26.8.2.

Final C3 verification — 2026-10-07: pinned Rust 1.97.1 / Node 24.21.0 on Apple
silicon pass complete locked/offline `node scripts/check.mjs`: 42 contract/inventory
tests, dependency boundaries, formatting, shipping/default and all-feature strict
Clippy/builds, all-feature debug/release and shipping-default Rust suites, 14 relay
and 24 private SDK tests. The all-feature service passes 94 unit and eight
integration tests, with seven named Nitro tests still explicitly gated. Pacifica's
read suite passes all 15 cases. Named killed-child helpers remain parent-invoked.
`node tools/web-channel/check-http.mjs` additionally passes all 26 existing groups
in each of actual Node and Chrome 154.0.8037.98; these remain synthetic cheap-handler
carrier regressions, not C4's real Runtime join. The local implementation commit
containing this receipt follows C2 `9feeee8a4a59e6abb7baec78c9600226192763ea`.
Logs: `/private/tmp/cinder-c3-check.DFahHx/check.log` and `http.log`. Temporary
logs/tool downloads are not required private test dependencies. All new regression
source is tracked. No Linux/SBF/ARM/AWS/venue/funded test or hosted CI was run for
this source, and old hardware success is not inherited.

The original C3 handoff was C4's fixed actual Node/Chrome + real Runtime/journal matrix at accepted
history 1/8/32/64 and synthetic witness delay 0/50/400 ms, one/two watches,
concurrent reads/mutation/tick, fault/continuous-write and peak-memory gates.
Do not count the existing cheap FixtureHandler process suite as that join. C5
still requires fresh ARM/EIF/client receipts and a reviewed new authorization
manifest before hardware. No old AWS caps or success receipt applies to this source.

#### C4 implementation and final local verification

`runtime::qualification` is compiled only with `local-fixture` and reached only
through explicit loopback `--runtime-web`. It assembles the actual Runtime/API,
AEAD/FileReplica journal and independently invoked synthetic witness port, not
FixtureHandler/Holds or a second financial engine. All native/financial gates
are false; no seeded customer credit, wallet/RPC/cloud access or external signer
is involved. Disposable keys/configuration and bounded aggregate controls travel
over harness stdin, never an HTTP API. Exact accepted histories 1/8/32/64 use
16-KiB non-economic evidence pads. Reopen preserves the original key/owner contract
and head; dirty initialization and changed owners fail closed.

The shared `runtime-suite.ts` has 24 cells in EACH actual Node/Chrome client:
writer/read witness delays 0/50/400 ms, one/two watches, two concurrent HTTP/WS
reads, same-socket grant and actual inactive tick. It asserts new read witnesses
without writer-backend I/O, bounded concurrent read work, exactly one mutation
acceptance, committed watch revisions and finite process peak RSS. Private
candidates, journal generations/history/indexes and carrier allocations are
included in the actual process measurement, not estimated separately and summed.
The declared 128-MiB fixture ceiling and small pages/16-KiB pads are NOT a claim
about maximum-sized production records, signed NSM cost or the Nitro application.

Eight further fixed groups (nine including the grid) cover revoke during I/O,
expiry, known post-CAS/pre-publication races, sustained unrelated commits, replay
without resend, continuous same-socket exact-retry traffic, the existing 32-update
slow-consumer queue, and witness/head/epoch/credential-port/lease/fence refusal.
Exact STS boundaries, writer poison/panic, uncertain acceptance, replica repair,
queued release and canonical all-family projection remain their existing tracked
cloud/journal/runtime/API regressions. The publication resource test now retains
four DISTINCT generations across writes, verifies shared original records and
refuses a fifth ticket without blocking the writer.

The fixed matrix is hooked into the existing offline browser CI runner, with its
job ceiling changed from ten to fifteen minutes for the added workload; no service
application/handshake/frame deadline changed. `/usr/bin/time` reports macOS/Linux
whole-process peak RSS and absence fails qualification; the final CI-only handoff
explicitly installs the Linux `time` package in its existing prerequisite step.
The test proxy observes
only public frames for notification timing. Fresh Chrome profiles, children,
private temporary files and keys are owned and cleaned by the harness.

The final frozen-artifact run passes all 24 cells and nine groups in EACH actual
Node 24.21.0 and Chrome 154.0.8037.98. The earlier unfrozen complete run also
passes, but the final receipt is the explicitly hashed single executable:
`3e6d595f3463c0c2ecee51923e6d13e59e311fee6918dcfce8f94c6ae2f0f3f1`.
This is a local-fixture executable hash, NOT a shipping ARM ELF/EIF/PCR pin.
One fixture-only parser discovery: serde's
unit-variant handling accepted extra fields despite deny_unknown_fields; using
empty struct variants makes the new zero-field controls reject them. SDK oracle
corrections use the actual `account` read family and completed-grant receipt.
No shipping API semantics were changed to satisfy these checks.

Observed limitation: a global publication that changes during a 400-ms read can
refuse it under sustained unrelated writes. After the explicitly tested quiet
window a fresh read succeeds, without weakening global generation or retrying a
mutation. Continuous-write read liveness is not promised. Full-history write cost
and finite lifetime remain unchanged; old hosted/AWS receipts do not qualify this
local source.

| Final workload observation | Node | Chrome |
| --- | --- | --- |
| Two reads together, zero witness delay | 1.7–5.3 ms | 1.6–2.3 ms |
| Two reads together, 50-ms witness delay | 55.9–59.2 ms | 56.7–63.3 ms |
| Two reads together, 400-ms witness delay | 409.0–414.0 ms | 406.9–418.7 ms |
| Same-socket grant, 400-ms writer witness delay | 2,859–3,111 ms | 2,866–3,128 ms |
| Whole-service process peak RSS across grid/faults | 10.5–15.6 MiB | 10.5–15.6 MiB |

All concurrent-read cells issue new independent witness calls with zero writer
backend GET/witness-read delta. Whole-run replica work grows with initial history:
15/155/1,751/6,567 GETs at 1/8/32/64 records; these totals INCLUDE initialization
and the write, not per-read work. The roughly 105-MB cumulative ciphertext I/O at
64 records is not resident memory or improved append asymptotics. Read concurrency
and same-socket service improve; the unchanged write path remains the slower work.
No cloud SLO, maximum-sized page/candidate budget or indefinite lifetime is inferred.

Final source verification — 2026-10-07: the local implementation commit containing
this receipt follows C3 `18f3607030b87f9e44e7312c1d08c12871197c9f`.
Complete pinned `node scripts/check.mjs` passes: all 42 contracts/inventory guards,
boundaries, formatting, default/all-feature strict lint/build, all-feature debug/
release and shipping-default Rust suites, 14 relay and 24 private SDK cases. The
all-feature service has 96 passing unit and eight integration cases, with seven
named Nitro tests explicitly gated. All eight publication tests, including the
expanded distinct-generation case, pass. The two new fixture setup/control guard
tests and focused strict all-target/all-feature service Clippy pass. Actual Node
and Chrome each also pass all 26 cheap-handler HTTP/WebSocket process groups after
the new fixture entry mode; these are separate from the joined matrix.
Metadata/format/diff checks pass after the evidence update. Logs under
`/private/tmp/cinder-c3-check.DFahHx/`: `c4-check.log`, `runtime-final.log`,
`http-final.log`, `c4-contracts-final.log`. Tracked regression source reproduces
the checks without ignored research, credentials or these temporary logs.

Next: C5's default-feature ARM/EIF/client build and fresh reviewed hardware
manifest. Apple container 1.4.1 is available; its cached pinned tool image has
been inspected and public dependencies vendored offline. No ARM package pass,
EIF/PCR, AWS, push, merge or financial activity is claimed by this C4 receipt.

#### C5 offline package and next authorization gate

2026-10-07: exact committed C4 source
`b38849cf93761f380e195a2731698d862ea84f4d`, tree
`97ac816d9491a2ffcb6bbfac95304275dca25166`, passes the shipping default-feature
ARM64 package gate in Apple container 1.4.1. Only its `git archive` source and
locked public vendor tree are mounted read-only; final artifact copy has a separate
owned output directory. No network interface except loopback, NSM, work/git,
wallet/cloud directory, inherited credential or feature-enabled fixture is present.
The seven shipping tools build; a clean second application build produces identical
enclave ELF; an ordinary non-Nitro boot refuses with exactly `cinder runtime fenced`.
Every copied binary/library hash verifies independently on macOS, and the copied
rootfs executable equals the recorded enclave binary. No new program/SBF run is
claimed: owning program sources are unchanged.

Tool-image index:
`33107aa751a9d69fb65acb26474fe3954dd30f8580b4150b4a63daf58bac434a`;
ARM64 manifest:
`d3d2f4db64dfa8e64503cc08102b68c70d9af963cc9508d0bc7927934dd814e2`.
Linux 6.18.35, Rust 1.97.1, OpenSSL `3.0.22-1~deb12u1`, pkg-config `1.8.1-1`;
deterministic package flags and remapped paths are unchanged. Cargo lock SHA-256:
`f113f7ddb0a37cc045aa0304188627a4f03567669215f0c488a9db2f41172bd3`.
New shipping enclave ELF SHA-256:
`4ff3a2f9fe2e73f42483645e8b3a02597544178ce83b52a72a5070aba52c924a`.
Package register SHA-256:
`114104d5146df25aea354c86cab98cd2c77d513d71c9d29440d0c6cde7581ccb`.
Bundle/log: `/private/tmp/cinder-c5-arm64.DLptc0/artifacts/bundle` and
`package.log`. Owned container `cinder-p23-c5-20261007` is absent after its
automatic removal; the pre-existing stopped builder/image are preserved.

Final joined clients use the existing unchanged generated WASM core, SHA-256
`3231c4a70cab472f7af5e4ac2b07f5895acb22abb88b82b13b455df30d5f3f03`,
and binding `ccd416be0ec626bd255f6d32f95e8f0eb2025854fbcd355021490839172adcdf`.
These local artifact identities and synthetic client receipts are not hardware
attestation, independent deployment pins or production release-distribution evidence.

Next is a fresh bounded **read-only** hardware proposal and scoped permissions,
then exact public manifest/rootfs/EIF/PCR review BEFORE key release. The historical
`p23-20261006-ro1` session is closed; its operator/boundary templates name that
specific namespace and do not authorize a new one. Do not reuse its resource
ledger, old heads, keys, authorizations or guessed image measurements. No AWS
calls/resources/spend occurred in this C5 preparation. Actual NSM/cloud concurrent
read/stream/revoke, restore/fencing/expiry remain unqualified for this new source;
trading/funding/native settlement and the rest of P23 remain separate gates.

Fresh untracked proposal:
`work/experiments/p23-live/C5-RUN-PROPOSAL.md`; approved by the user on 2026-10-07.
The artifacts above are historical pre-review inputs; rebuilding and current
access/manifest/image checks are still required, not waived by that approval.
It proposes one $5/four-hour/eight-boot read-serving session in a new
`p23-20261007-ro2` namespace, with zero financial/native/RPC actions. Templates
in `c5-permissions/` introduce only namespace/setup-policy-name substitutions;
parse and exact reverse-substitution comparisons pass. Operator/boundary hashes:
`dcd77901e4eedfab3c36730db1c24f68176f892c540d2a180f23ecb410ce860e` /
`78ecabb8b4184af1a8cbcecb8596d1326c005d6bdf82d5809435daf3a73c3aaa`.
An administrator must create the two fresh scoped policies and attach only
`CinderP23C5Operator` to the intended IAM user; the runtime boundary is NOT a
user attachment. Live identity/price/permission checks, fresh operator runner,
actual manifest/EIF/PCR inspection and key-release review remain prerequisites.
Neither local templates nor an approval field grant live authority. Existing
closed-run scripts/records are preserved; do not execute their fixed namespace.
Final publication handoff changes only documentation and the Linux test-tool
prerequisite; all 42 metadata guards and diff checks pass, application code and
the tested package/client artifacts are unchanged. No new hosted pass is claimed.

### Earlier architecture review — 2026-10-06

Published checkpoint remains `6dff3731f365463ad24c1b5fcd2cb82f19d76c86`,
ready/open PR #55 against `product/tee-v1` at
`833955f8a4ead02a71eb5ae69a6aebd92bb85091`. The last independent readback
confirmed all seven hosted checks and CodeRabbit success for that published head;
local diagnostic/proposal edits are not that checked source. No merge occurred.

After the local contention diagnosis, the user authorized a bounded design review,
then asked to pause the narrow fix and take a fresh whole-architecture view with
one explicitly authorized Astra review agent. That agent performed one independent
read-only review and formed findings before reading the candidate. No production
changes, interleaving model, new benchmark/test, AWS run or financial activity were
performed in this review. Unrelated local files remain untouched.

Source review confirms several composed synchronous costs: active mutex across
remote I/O; tick freshness even in read-only mode; duplicate runtime/API freshness
checks; API record reconstruction with quadratic receipt searches; complete-history
replica GET/repair per append; per-call cloud TLS; NSM time attestation/verification
under its shared mutex; and a separate same-socket poll/command head-of-line path.
Fixed clock/cheap process fixtures did not qualify the production dependency budget.
The local real-runtime measurements establish contention mechanisms, not the exact
cause of every AWS close. Current 4,096-record/64-MiB history is a finite lifetime;
the archive snapshot format is not compaction. Do not quietly remove its checks.

At this earlier review checkpoint,
[ADR 0025](../architecture/0025-concurrent-private-reads.md) was **proposed**,
not selected or implemented; the later approval and C1 work are above. It recorded
the independent review's conditions:
fresh NSM time outside a short publication latch; exact known-writer head metadata;
bounded State/index generations; unrelated-commit starvation; and the current
TLS-or-web ingress selection without unnecessary multi-listener work. No abstract
model was run. Detailed broad findings and alternatives are preserved in ignored
`work/experiments/p23-concurrency/ARCHITECTURE-REVIEW.md` for local handoff; the
sanitized candidate is tracked so a fresh checkout has the decision conditions.
Verification for these documentation edits: pinned Node 24.21.0
`scripts/check.mjs --group=contracts` passes all 42 contracts/inventory regressions
and workspace boundaries; `git diff --check` passes. No runtime regression or
hardware qualification was rerun for this read-only review.

Next: agree a cohesive runtime/storage scope, rather than continue timeout patches.
Keep one mutation owner/journal, central replay-derived indexes and immutable
publication, independent per-reply freshness/authorization, bounded external I/O
completion and one connection-owned cipher writer. Explicitly choose a bounded
dev/test retained-history envelope under today's durability versus a reviewed
segment/checkpoint/retention contract. Do not label read isolation complete broker
qualification, weaken trusted time/freshness/revocation, or start new hardware
before that choice and the fixed joined local matrix. Financial/custody decisions
and all independent native/chain/customer-funds gates remain unchanged.

### Latest hardware checkpoint — 2026-10-06

The approved read-only AWS session has ended; resource cleanup is verified.
Eight of eight boots were used on the one approved host, with zero financial
activity. Actual six-purpose release, independently checked AWS/Noise HTTP
own-only reads, accepted-operation restore, witness loss/epoch fencing and both
finite lease and natural witness-STS expiry have passing bounded receipts.
The final positive STS baseline completed at 13:13:28.876 UTC; fresh reads refused
at 13:14:19.001 UTC after actual 13:14:04 UTC credential expiry, without changing
the intervening clock, head, policy or routes.

The approved fifteen-second application-reply correction passes local slow-reply,
silent-timeout and unchanged five-second handshake regressions. All eight relay
tests and 26 shared SDK groups per actual Node/Chrome runtime pass; the final
pinned full offline runner also passes. Actual Chrome grant delivery completes
at 12,933 ms. Full live agent/subscription/revocation handling is **not qualified**:
initial subscription/read refusals remain. Serial accepted-history verification
under the shared runtime lock is a confirmed contention mechanism, not a measured
explanation of every individual close. Six accepted grants reconcile with zero
exposure and no resend; no further grant, boot, timeout widening, stale-view
workaround or durability redesign was performed.

At hardware close, the next substantive step was to measure journal/write/read/subscription
contention locally before proposing a reviewed scheduling/storage correction or
another hardware run; the latest local diagnosis and broad review are above.
The expired original stream's post-replica-fault restore,
complete native read qualification and actual devnet diagnostic request remain
named gaps. Financial setup/credit/payment/cut/funding and devnet deployment are
separate gates. Application corrections are checkpointed at
`dc8844bda90ab78b672967b96b0efaf282abd711`, on the same PR #55 branch; the user
authorized commit/push followed by focused local concurrency diagnosis. This
documentation successor records the tested source and bounded hardware outcome;
the exact published head/base and fresh CI/review state belong in the PR receipt.
No new hosted pass, merge or P23 closure is claimed. Detailed receipt below;
protected source/image/client logs and
106 archived ciphertext objects remain in ignored local experiment storage.
Independent cleanup confirms the instance terminated, test EBS/storage/witness/
roles/network resources absent, and all six KMS keys disabled in PendingDeletion
with a seven-day deletion window. Pre-existing setup policies were preserved;
local test relay/tunnel listeners are closed. Billing is delayed; the $5 cap is
not a claim about a finalized invoice.

### Earlier publication and hardware checkpoints

Publication/local diagnosis follow-up: the user-requested checkpoint is pushed
via the one-layer native stack to ready/open PR #55 at
`6dff3731f365463ad24c1b5fcd2cb82f19d76c86`, base `product/tee-v1` at
`833955f8a4ead02a71eb5ae69a6aebd92bb85091`. Its PR description records exact
source, tests, hardware limitations and verified teardown. Fresh CI/review are
separate; no merge is authorized. The following new diagnostic is local only,
not part of that published source or a production change.

Test-only `runtime_contention_tests.rs` joins real Runtime/API/AEAD/Replicated
with counted synthetic storage. Healthy append at accepted history N=1/9/25
performs 5/29/77 replica GETs (`3N+2`), two PUTs, five witness reads and one CAS.
With 400-ms synthetic witness-read latency, a read alone takes 808 ms/two witness
reads and a grant takes 2,828 ms/seven witness reads. Both cause another initial
read to hit the unchanged bounded lock refusal while periodic polls skip without
cached output. The healthy runtime resumes afterward; original grant lookup,
revocation and zero-attempt/no-money checks pass. Two focused diagnostics and all
83 service unit tests pass; seven named hardware tests stay gated. Strict offline
Clippy passes. This is handler-level causal evidence, not every AWS close or a
full SDK/watch benchmark. The measured finding is that read-on-read contention
also matters, not only write/history cost. The local experiment note records
the full method, limits and proposed alternatives; it is not a CI prerequisite.

Next: review a bounded fresh-head/epoch/authenticated read-serving design against
a fair serialized queue, then join the chosen candidate to actual SDK/watch race
tests. Keep append scaling as a separately reviewed durability problem; do not
remove full-history repair checks, relax freshness, add another ledger or expand
timeouts silently. No new AWS/financial activity or scheduling redesign occurred.

[PR #55](https://github.com/arnabnandikgp/cinder/pull/55) is ready/open against
`product/tee-v1`, following the user's readiness and review-fix requests. Reviewed
head `d26945fc7c749dc3f464f720db047ff64a8ee0a2` passed all seven hosted checks;
CodeRabbit reported three valid issues, and the connector posted two further
valid issues. Current corrections: exact manifest capsule coverage; rejected
original deposits advance polling without credit; expired unsent physical plans
close only from verified no-wire/no-effect history; individual deposit lookups
claim no complete source cut; requests have bounded lock waiting and polls leave
a full idle interval. Final correction verification and publication receipts are
recorded in the P23 log and PR description, not inferred from the older green head.
No merge is authorized. Changed source invalidates the older shipping ELF and
any hardware measurement; only unchanged program/browser source retains its
own separately identified prior test receipts.

Correction application checkpoint `5acf79bd2f931e85325841a06bd87b038cb1cf28`
passes the full pinned offline host suite and exact-source ARM64 package check.
The detailed receipt is in the P23 log below; this documentation-only successor
does not change the tested application. Verify the published head/base and new
hosted CI independently; the PR description carries that publication receipt.

Access preflight on 2026-10-06 verifies all seven hosted checks plus CodeRabbit
success at published head `b6a7d30f0a5ba30113a08cb8fa82bea33f78a66f`.
Renewed `cinder_new` STS is the expected non-root IAM user in account
`588966314654`. Regional metadata and EC2 launch DryRun pass; the old P20
operator/boundary policies cannot manage the fresh P23 witness, roles and keys.
The user completed P23 console setup. Independent live readbacks of
`CinderP23Operator` v1 and `CinderP23RuntimeBoundary` v1 exactly match the supplied
scoped JSON; operator is attached and boundary is not attached to the user.
Fresh `p23-20261006-ro1` roles/table are verified absent, and exact bucket names
are absent from the owned inventory (global name availability is not inferred).
The user approved the ignored run proposal on 2026-10-06: one four-hour/$5
read-only hardware session with zero financial activity. Fresh six-role local
preparation passes the real loaded-contract and 4-KiB KMS capsule checks; exact
public ARM64 artifacts are revalidated before upload. The operator ledger records
six keys, two private buckets, one witness table, two boundary-restricted roles
and one enclave-enabled `c6g.large` instance `i-07e2c3fb878e056c0`. Its setup
completed with Nitro CLI 1.5.0; authenticated SSH uses the AWS-console host key.
No role plaintext or signing key is uploaded. The original clean measured image
passes actual non-debug NSM/six-purpose KMS, Node/Chrome Noise HTTP own-only
reads, independent wrong-pin/replayed-quote refusal, and exact accepted-operation
restore. A replica-loss mutation refused and the independent accepted witness
head did not advance; its post-fault restore check remains incomplete because
the original witness credential expired at 12:00:45 UTC, without a head reset.
Actual WebSocket testing exposed two issues: opaque ingress closed after five
seconds of silence, and periodic subscription polls treated journal contention
as fatal. The user approved both bounded corrections. Corrected parent-only idle
reproduction passes against the original enclave. The enclave correction skips
busy signed Read polls without cached replies; witness/auth/lease/storage failures
remain fatal and ordinary commands are not retried. All 81 service unit tests,
strict Clippy and the full pinned offline workspace runner pass; seven hardware
tests remain explicitly gated. Default-feature ARM64 rebuilt enclave ELF is
`816dc8d7eddf0cb13ba9986e14a3f86cc1b81ea0732d05b4e87fa02b03b7d884`.
The source patch and independently reviewed EIF/PCR receipts are preserved in
ignored `work/experiments/p23-live/LIVE-RESULTS.md`; changed-image transport
qualification is still pending. Four boots used at that checkpoint; no extra
instance or financial activity is approved. Next: qualify the approved correction
with the planned fresh read-only profiles, finish bounded fencing/expiry checks,
then reconcile and verify teardown. No new hosted checks are inferred from the
previous published head, and this is not full P23 closure.
Continuation checkpoint: boot 5 used a fresh independent READ stream. A lost
READ-grant reply was reconciled by own authenticated lookup (one accepted grant,
epoch 1, zero exposure, no resend). Actual AWS attestation exposed a minimally
encoded 20-byte serial magnitude plus sign padding; the user approved a narrow
SDK verifier correction, preserving fixed-root/path/signature/PCR/freshness
checks. Public certificate-only regression and full Node/WASM/Chrome offline
suites pass, and the actual quote verifies. Witness-loss refusal passes on boot 5.
Slow cloud polling also reproduced a cadence failure; the approved contention
fix now leaves the full 500-ms request interval after I/O. All 81 service tests,
strict Clippy and full pinned offline checks pass for this refined source patch
`3093512a3bc77a4232f0001c03752bff50d53b60377f01267eb5e84f21fc3898`.
Shipping ARM64 ELF is
`54bfa130a2f108443651dd05d383fef7fdb933670211eacac7f33c5c38b3923a`;
READ replacement EIF is
`61e8e05d66db51ba331ca9272488f8412fc145b6e9dce75aec6f59f2c03efcc0`.
Five of eight boots used; full live streams, conditional epoch fencing, finite
lease/STS checks and teardown still pending at this checkpoint. Historical
failed images/receipts and accepted heads are preserved. No new hosted result
or full P23 closure is claimed. Local generated `target/debug` was removed under
the user's standing authorization after disk exhaustion; release artifacts,
receipts and unrelated work were retained.
Next live checkpoint: boot 6 restores the same accepted READ grant; a conditional
witness epoch 1→2 preserves sequence/hash and fences the old worker. Full Node and
Chrome streams are **not passing**, although the approved fifteen-second SDK/
opaque HTTP reply change now delivers the accepted grant and pushed revision.
Local reply regression fails before/passes after; all eight relay tests, 26 joined
SDK groups per Node/Chrome, TypeScript and dependency/boundary checks pass.
Actual grant latency reaches 12,933 ms; read/subscription refusal remains.
`Replicated::append` rereads and verifies accepted history under the shared active
mutex, so this is not a qualified low-latency/concurrent broker. No additional
deadline widening, stale-cache workaround, grant attempt or storage-design change
is authorized by this finding. Six accepted distinct grants reconcile with zero
financial exposure/no resend. Original lease/STS/teardown checks continue on the
remaining two boots; next substantive work is a separate local durability and
read/write scheduling analysis. P23 remains in progress.
Solana deployment/funding and native financial qualification remain separate.
Live acceptance remains separate.
Authentic native setup/credit/payment/cut/funding remain independently gated;
shipping validation still refuses trading/funding and API risk admission is
disabled. Local preparation is not P23 live acceptance or phase closure. Access
preflight used AWS read-only metadata/DryRun and public devnet genesis/balance
reads; no resources, account setup, deployment, signatures or funds moved. Preserve
untracked `docs/user-journey.md`, `stays/`, ignored research and docs content.

### CI follow-up merge receipt

CI follow-up [#54](https://github.com/arnabnandikgp/cinder/pull/54) is merged into
`product/tee-v1` on 2026-10-06 at
`833955f8a4ead02a71eb5ae69a6aebd92bb85091`. All seven hosted checks and CodeRabbit
status passed at exact reviewed head `9d9dd760eae57cf920ac9956d6eb903c954ad43a`;
no unresolved review threads remained. The user authorized merge. Native
`gh stack merge 54` reported no remote stack; the exact-head-guarded standard
squash merge succeeded, and the GitHub receipt was verified. Local product trunk
was fast-forwarded to the same upstream commit; legacy `main` is untouched.
Recommendations 2/3/4, caching and both prerequisite fixes are now landed.
This docs-only merge closeout remains local for the next authorized PR, not an
unreviewed direct push to trunk. At this merge receipt P23/P24 remained open;
the newly authorized P23 continuation is recorded above. P23 needs its own current bounded G05
environment/account/funding/cleanup manifest. Preserve untracked
`docs/user-journey.md`, `stays/`, ignored `work/` and docs content.

### Historical CI follow-up handoff (superseded)

Current work: the authorized CI follow-up on `tee/ci-offline-gates`, based on
merged `product/tee-v1` at `8247d24fa6d77b1e0b478ef7b8513ace80f34d4b`.
P22 #53 merged on 2026-10-06 after all four hosted checks passed at exact reviewed
head `24fe0796fbcfb2151de779922ae1932defc745f6`; both relevant CodeRabbit threads
were resolved. Local product trunk was fast-forwarded before branching.

CI audit recommendations 2/3/4 are authorized: automatic test inventory and
executed-case/intentional-skip guards; shipping-default tests and offline ARM64
release packaging; independent contracts/lint/Rust/SDK-relay reporting.
The user's subsequent request also authorizes build/dependency/tool caching in
this same PR. Focused local cache qualification passes; all builds/tests remain required.
Implementation checkpoint `297327faca999977581ed407e7acc9db3e1ad828` passes the
full local checks below; final reporting/receipt-shape edits also pass focused
guard and actual Node/Chrome carrier checks. No new economic
policy, program, product dependency pin, AWS session, live venue, RPC, wallet or customer
funds are in scope. Branch protection, artifacts/coverage and advisory scans
and live gates remain deferred. P23 is not started.
Ready/open [PR #54](https://github.com/arnabnandikgp/cinder/pull/54) targets
`product/tee-v1`, published via native `gh stack init/submit` at
`d655f39776ac777ec9617a4c5b743b187381dd55`. Its committed source-only export
passes all 41 guards and the workspace graph with no git/research/user files.
Later commits record the browser prerequisite fix and the newly authorized
caching. Hosted results for the current head remain pending and are not inherited
from P22 or local checks.
Hosted follow-up: the initial browser job failed because automatic discovery
ran the certificate serial tests before building their native verifier. Local
warm targets had masked that ordering error. The existing fixture build is now
moved before discovery execution. Full qualification passes from the fresh
committed `ad136c6` source export with no native/WASM build outputs: 11 tool and
12 relay tests, both isolated Rust profiles, and actual Node/Chrome each passing
14 core/attestation and 24 carrier groups. No test is disabled and no dependency
pin or protocol behavior changes. The ensuing docs-only successor records that
first justified CI fix. Caching source `28a78924254581d6b3aaed09f7be4e19bcaf9e6c`
passes all 41 contracts/inventory guards, workspace boundaries, workflow lint and
cold/warm network-disabled ARM64 package checks, using the runner UID with
read-only committed source/vendor mounts. Both runs explicitly rebuild the
application twice, compare identical ELF files and reject non-Nitro boot.
Hosted checks at `97c7fdcd163f1c130b06bade8f017b597196edbe` pass six jobs,
including ARM64 packaging, browser, SBF and both Rust checks. SDK/relay job
`112097236435` fails seven transport tests with `cinder-relay ENOENT`, then hits
its 15-minute timeout because partial harness startup leaves fixtures alive.
The independent client group had built only the service/fixture verifier, relying
on the old preceding all-target build for the relay and production verifier.
The user authorizes this fix: share the exact four-binary prerequisite list
between the explicit build and a fail-fast executable check before SDK startup.
A guard regression rejects missing/non-executable prerequisites, including the
production verifier, without starting the process suite. Fresh committed-source
`9c0e5c1cdd955f61afe7e94bade2ba6793eac7a4` client-group verification passes:
offline npm install, a new empty Cargo target, explicit builds of all four
prerequisites, all 12 relay tests, strict TypeScript and all 24 SDK/TLS tests.
The same source-only export passes 42 contracts/inventory guards and workspace
boundaries. No git, research, user files or prebuilt workspace outputs are present.
No timeout, cache or financial oracle is relaxed. This second evidence-based CI
fix addresses an identified platform-independent missing-build cause; another
persisting failure requires local Linux reproduction per the contributor guide.
GitHub checks for the successor remain pending after push; prior-head green
checks and local results are not presented as new-head hosted success.
Next: await hosted checks and review/merge authorization. Preserve untracked `docs/user-journey.md`,
`stays/`, ignored `work/` and docs content.

### Historical P22 review handoff (superseded)

Current work: P22 review follow-up on `tee/p22-offline-acceptance` in `cinder-tee`,
based on merged trunk `1c2c3eb836003e9aeb252e85e3aacdf5eb32ecf7`.
Ready/open [PR #53](https://github.com/arnabnandikgp/cinder/pull/53) targets
`product/tee-v1` through native `gh stack`. All four hosted Linux jobs passed at
`9a4174527757e120ee81bfaf8d073480b153a37e`; CodeRabbit then identified two relevant
minor acceptance-harness findings. Both are corrected at source checkpoint
`b7ba0c9bb28bec6b58b2a8d8c4a5c87e9514dda2` and documented in P22 below.
Full local workspace debug/release, Clippy, boundary and SDK checks pass; focused
HTTP/WebSocket SBF normal lifecycles pass after first reproducing both failures.
The final manifest/metadata regression rerun passes all 34 tests. This successor
only records that evidence and awaits its own hosted checks; it does not inherit
the prior head's CI success. Earlier full browser/57-case receipts stay historical.
Publication receipt: source/evidence pushed at `baf340f239223067d95b856e17bf28d14b67c47e`;
both CodeRabbit threads were answered and resolved, and hosted reruns were queued.
This documentation-only successor records that receipt, not a fresh CI success.
No AWS, live venue, remote RPC, configured wallet or customer funds are in scope.
Next: check the current tip's hosted results and await merge authorization. P22 remains
`in progress` until review and merge; P23 is not started. The CI audit recommends
targeted improvements but does not authorize workflow or branch-rule changes.
Preserve untracked `docs/user-journey.md`, `stays/`, ignored `work/` and docs content.

### Historical P21 merge handoff (superseded)

PR #52 merged into `product/tee-v1` on 2026-10-04 (Asia/Kolkata), at
`1c2c3eb836003e9aeb252e85e3aacdf5eb32ecf7`; local trunk fast-forwarded to that
exact commit. Reviewed head: `ff3b4c3752656ffeffdc1b88e92da0a223b5f710`.
All original runtime review fixes passed full local checks and all four hosted
jobs at `ea23a17abcf8d91525962ce6ec2c50579ed21b66`. The final successor changes
only README wording and tracker bookkeeping; its focused local checks passed.
The user explicitly authorized merge without waiting for its CI rerun; pending
checks at that head are not claimed as passed. All five review threads were
resolved, including the two minor README corrections and the timezone explanation.
Native `gh stack merge 52` reported no remote stack; standard `gh pr merge` with
the exact-head guard succeeded, and GitHub's merge receipt was independently read.

P21 is closed for its approved offline acceptance, and P21B's tracked delivery
receipt is merged. Neither means production qualification or hosted publication.
Next: await authorization for P22's offline adversarial acceptance; P23 still
owns live ports and changed-image qualification. No next implementation phase
was started. Preserve the user's untracked/ignored files. This merge closure is
recorded locally for the next authorized PR; no extra trunk commit/push is made.

### Historical final P21 publication handoff (superseded)

Merge follow-up — 2026-10-04 (Asia/Kolkata): all four hosted CI jobs and
CodeRabbit passed at `ea23a17abcf8d91525962ce6ec2c50579ed21b66`. The user
authorized merging #52. Two subsequent minor README findings are corrected:
the version check retains the PATH fallback, and offline native-fencing
evidence is explicitly fake rather than live qualification. The date warning
is a UTC/local-time difference: the review-fix commits at 2026-10-03 19:58–20:03
UTC occurred on 2026-10-04 in Asia/Kolkata, not in the future. This docs-only
successor still needs fresh hosted checks before merge; no runtime change or
additional implementation phase is included.

Current work: `tee/p21-recovery-integration` in `cinder-tee`, based on merged
`product/tee-v1` at `05303740dd00129eeb038c9963f44301405725d7`.
Native stack #49 merged #47/#48/#50/#51 after explicit user authorization and
fresh successful checks at all four exact heads; P21A is closed for offline scope.

P21B contains 37 copy-ready `.mdx` pages in `docs-content/`, for the user's
existing Mintlify repository. At the user's request the entire directory is
ignored/untracked; their own local `docs.json` is preserved. No new
site/package/hosting/CI is included.
Eight commands and ten read families use native field, request-example and
response-example components; actual shared binary route and SDK semantics remain
explicit. No public endpoint, self-service onboarding or live financial workflow
is invented. Content references the merged code and approved baseline.

Verification: 37 MDX pages compile with a temporary pinned MDX 3.1.1 compiler;
158 internal page links resolve; all 56 API TypeScript examples strictly typecheck
with pinned TypeScript 7.0.2 against the merged SDK. All 18 command/read references
have the required field/example components and the real encrypted carrier with
playground disabled. The foundation checker and all 13 focused tests pass.
No new-format Mintlify hosted rendering or publication is inferred from compilation.

Metadata follow-up: all 18 command/read references use ordinary encrypted-carrier
text instead of relative `api:` frontmatter; the incompatible playground stays
disabled. All 37 MDX pages compile and all 37 user-supplied navigation entries
resolve after the edit. `git ls-files docs-content` is empty. Content delivery is
complete; P21B remains in progress only until this tracked delivery record merges
with P21, not because a separate docs-source PR or hosted site is required.

P21 now satisfies its three **offline** acceptance criteria. Implemented: complete
settled-cut derivation; original P16 lifetime counter/backing matching; explicit
recovery cutover, bounded IOC closes/flat returns and irreversible sealing; immutable
wallet-authorized dedicated encryption-key registration; recipient-encrypted packages
with both store readbacks; separately signed opaque manifest; independent Node
retrieval/decryption/live-custody verification and unsigned claim construction.
Actual HTTP/WebSocket SDK/service tails survive process death and end in compiled
local payout/return/claim instructions with ordinary service unavailable.

Initial P21 verification on macOS arm64, Rust 1.97.1 / Node 24.21.0 (before the
review follow-up below):
`node scripts/check.mjs` passes foundation/boundaries, strict default/all-feature
Clippy, all-target build, workspace debug/release tests and all 24 SDK/TLS tests.
`node scripts/check-vault.mjs` passes locked build/IDL/strict Rust/TypeScript and
all **55** codec/SBF/recovery tests, including both accepted-tail outage rehearsals.
`node tools/web-channel/check.mjs` passes native/WASM debug/release/Clippy plus
14 verifier/core groups and 24 HTTP/WebSocket/private process groups **each** in
Node and actual Chrome 154.0.8037.93. The SBF runner uses Anchor 1.2.0, Surfpool
1.5.0 and cargo-build-sbf 3.1.10/platform-tools 1.52. Reproducible low-disk settings:
`CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0`;
the isolated WASM target uses the previously installed task-specific Rustup home.
No container, AWS, live venue, devnet, deployed key or real funds were used.

Initial joined failures were diagnosed locally: missing broker scenario coverage,
all fixture assets parked in the vault instead of native margin, and a preexisting
wallet-versus-token payout recipient mismatch. The real signed close test also
caught cleanup-credit eligibility incorrectly choosing the cancel endpoint. Fixes
preserve admission/signature/economic gates. Adding the broker source required
explicit synthetic readiness in the old transport-only fixture; all affected
SDK/TLS/browser regressions passed after that correction. Earlier failed runs
are not passing receipts. The initial 16-cut/23-funding slice receipt remains
historical; the final funding file now has 26 passing tests plus its parent-invoked
worker, and all 16 cut tests still pass.

Limits: native capability closure/completion, chain authentication and independent
store failure domains are explicit ports tested with offline fixtures, not live
proof. Preparation and claimant parsing are implemented, but package hosting,
public onboarding, key rotation/lost-key reissue, new-image hardware qualification
and production entropy/erasure are not inferred. Engine revision 21 rejects prior
revision 20 histories; migration is not implemented. No heartbeat payout, parallel
ledger, debt-as-cash, loss-sharing or venue/mainnet policy is introduced.

Published [#52](https://github.com/arnabnandikgp/cinder/pull/52), ready/open, as the
single inspected native stack layer against `product/tee-v1` at
`05303740dd00129eeb038c9963f44301405725d7`. Implementation/source checkpoint:
`dcb685f277d2c04e1256258d7cb3b10d2c0be266`. Both joined outage flows and strict
build/IDL/TypeScript checks were repeated successfully at that committed checkpoint
with `node scripts/check-vault.mjs --recovery-only`; the focus is **not** another
55-test full-suite receipt. The initial publication head
`bfe4ae64e05bb3504df9332a83149abbf7e6ecbc` passed all four hosted CI jobs;
CodeRabbit then posted two actionable findings at that head.

PR #52 review follow-up — 2026-10-04, tested source checkpoint
`8e528db870d81a7cd8011780726b8ab32795b1be`:

- Recovery keys register only on a fresh store; reopen validates the exact
  complete owner-authorized inventory against the currently witnessed journal.
  Preparation reuses the same validation. Both HTTP/WebSocket tests now prove a
  successful pre-cutover restart with retained order/payout receipts and unchanged
  head; missing keys and an owner-signed replacement key refuse without mutation.
  A disposable identical-store probe changes ONLY the witness epoch to demonstrate
  old-writer refusal independently of key registration or final recovery state.
  The original store's fence is never rolled back, and the final epoch-2 refusal
  also checks no listening output or accepted-head mutation.
- Default vault checks retain the joined recovery coverage. The vault README now
  includes exact wasm-bindgen 0.2.129, the WASM target and root/browser dependency
  hydration; hosted vault CI already provisions these prerequisites. Missing or
  incorrect bindgen fails with an actionable setup reference instead of a bare
  process-launch/version error. No dependency pin or CI coverage was weakened.

Repeated offline verification: `node scripts/check.mjs` passes foundation/graph,
strict default/all-feature Clippy, all-target build, workspace debug/release and
all 24 SDK/TLS tests. The DEFAULT `node scripts/check-vault.mjs` passes all 55
tests, both strengthened carrier flows, locked SBF/IDL and strict Rust/TypeScript.
Two explicit missing/wrong-bindgen diagnostic checks also pass. Initial local
compilation identified that witness verification needs mutable port access; the
helper was corrected before all successful reruns. No AWS, live venue, devnet,
wallet, container or unrelated docs-workspace edit was used. The earlier Chrome
qualification above is historical, not a new-browser receipt for this follow-up.
This tracker-only successor does not inherit hosted CI from the initial head;
fresh hosted checks/review remain required after the review-fix push.

Published review fixes and handoff at
`521dcceed2b3bcc063140061407af9dfb43ed1e4`; both CodeRabbit threads were replied
to and resolved. At that exact head the plan job passed and the three executable
jobs were in progress. This publication-receipt successor is documentation only;
its fresh hosted CI remains required, not inferred from those running checks.

Next: await #52's final-tip hosted CI and focused review; merge only with user
authorization; do not auto-merge or call P21/P21B closed. P22 is the next
planning boundary, expanding complete adversarial
lifecycle acceptance; P23 alone qualifies actual ports/changed Nitro image.
Preserve user-owned `docs/user-journey.md`, `stays/`, local `docs-content/docs.json`
and ignored `work/`.

### Historical final P21A publication handoff (superseded)

Current work: published [#51](https://github.com/arnabnandikgp/cinder/pull/51),
`tee/p21a-reads-websocket` in `cinder-tee`, final grouped P21A.4/5 slice above
ready/open #50 at `4841c7044dc7d1080c5ad8f45ddc14277bae4bca` through native stack #49.
First published source: `a0442d5cb3af7f3706be1b31e05b301adeacee35`; the scoped
subscription-iterator cleanup and this publication receipt follow that checkpoint.
#47/#48/#50 stay unmerged while the user reviews them.
Trunk remains `product/tee-v1` at `76f32956e3ee5d31707ec1bf7fa81f8711666c97`.

Implemented: signed read version/paging, own account/risk/position/operation/order/
fill/funding/book-history/grant/metadata projections, shared WebSocket commands,
currently authorized coalesced private snapshots, finite queues/records/deadlines,
gap/reconnect/revoke/expiry and joined offline acceptance. Immutable book-history
index derives from the same committed/replayed journal, not a second ledger.
[Exact contract](../architecture/private-read-contract.md) states unavailable
fields, source/time/retention limits and deliberately narrower forced-fill/native
metadata coverage. P19 TLS and financial/custody/margin policy are unchanged.

Verification: final pinned full workspace runner passes default/all-feature strict
Clippy, all-feature build, debug/release tests, 30 foundation/boundary regressions,
12 carrier tests and 19 SDK/Node-TLS tests. API signing/history/funding and all five
native web ingress regressions pass. Full native/WASM/verifier/browser runner passes;
final joined acceptance passes again with emitted-frame queue synchronization.
Actual Node 24.21.0 and Chrome 154.0.8037.93 each pass 24 joined HTTP/WebSocket
process groups, in addition to 14 verifier/core groups each. Native/WASM debug/
release/Clippy and 12 carrier tests passed. Fixture-only dropped-delivery control
now assembles TCP frames before dropping; packet boundaries are not frame boundaries.
Parent traffic/logs/disk private-marker checks pass. No actual venue fill/payment,
hardware qualification, live endpoint or key-erasure guarantee is claimed.

Final cleanup: returning an unused subscription iterator now closes its underlying
channel even before the generator starts. Strict SDK typechecking and all 20
SDK/Node-TLS tests pass; the public SDK bundle builds and Node/actual Chrome each
repeat all 24 joined groups with the new regression. Its fixture waits for public
delivery before abandonment, avoiding artificial read-lock contention without
relaxing the service's fail-closed busy response. Rust/crypto/financial source is
unchanged by this narrow follow-up, so the full workspace receipt above remains
at `a0442d5`, not falsely relabeled as a new full run. All four hosted jobs pass
at `a0442d5`; subsequent cleanup/receipt CI and review require their own evidence.
A successful CodeRabbit status is not a completed review disposition.

CodeRabbit review follow-up: reviewed checkpoint
`9fa8748d9feb729405473ab91717adbb51f0ec49` has all four hosted jobs passing,
but its closure assertions accepted a six-second harness timeout as termination.
The finding is valid: an indefinitely pending iterator reproduced that false
pass. The test-only fix uses actual iterator rejection or `done: true`; timeout
and another emitted page now fail. All five revocation/expiry/gap/overflow/
witness-loss scenarios use it. Four deterministic regressions cover actual
termination, unwanted pages, stalled streams and termination after the deadline.
Strict TypeScript and all 24 SDK/assertion/Node-TLS tests pass; Node 24.21.0 and
actual Chrome 154.0.8037.93 each repeat all 24 joined process groups successfully
with the stronger assertions. These supersede the earlier five closure claims;
the earlier green run alone did not establish termination. Production SDK,
Rust/crypto/financial source and dependency graph are unchanged, so no redundant
full Rust/SBF/hardware rerun is claimed. Review-fix CI requires its own head;
the docstring-coverage warning is not a reason for blanket comment changes.

Published review-fix source: `4cf0462c91f1ac424e9ecf9ebee29845500e6b58`, above
unchanged #50/base `4841c7044dc7d1080c5ad8f45ddc14277bae4bca`. The actionable
thread has a fix/evidence reply and is resolved. New-source CI is running; this
publication receipt is documentation-only and also needs its own head checks.
Neither the old green jobs nor the resolved thread closes P21A or authorizes merge.

Next: review #51 and its current-head checks, then bottom-up merge when authorized.
P21A/milestones stay in
progress until relevant reviews are addressed and all layers merge. P21B Mintlify
follows the implemented API; recovery/P22/P23/P24 remain. No AWS, configured wallet,
venue, container, subagent or merge used. Preserve `docs/user-journey.md`, `stays/`
and ignored research.

### Historical HTTP publication handoff (superseded)

Current work: ready/open [#50](https://github.com/arnabnandikgp/cinder/pull/50),
`tee/p21a-http-sdk` in `cinder-tee`, above P21A.2a #48 at
`df1e71f9110c4c4b940c689db4540839656ec473` through native stack #49.
Published source head: `8cf077298d8cc288223ba88133472d8b52187c61`.
P21A.1 #47 is at `9aefdd9d3478870ccf58b3cc1cf1ade449f13cd3`; trunk remains
`product/tee-v1` at `76f32956e3ee5d31707ec1bf7fa81f8711666c97`.
All four hosted jobs pass at that #48 source-fix head, independently checked
before this decision-doc update; this is not CI for subsequent changes.

The user selected pinned upstream Snow 0.10.0 for V1 after the alternatives
research. Continue bounded offline integration with synthetic data; no custom
handshake or crypto fork. Opaque DH/cipher/chaining-key erasure is not qualified
and must remain explicitly documented. Customer use requires an explicit
hardening/security disposition and the applicable production/release gates.
Do not claim a wrapper drop or browser teardown fixes that limitation.

The shared core is now in `crates/web-channel`; the isolated WASM tool compiles
the exact same source. The service adds NSM-only responder entropy, purpose-bound
web quotes and a bounded ingress to the existing P18 Session/Handler/journal.
The fixed-target parent Node HTTP relay handles public framing/ciphertext only.
The shared browser/Node SDK fixes AWS-root trust and supplies complete correlated
replies, finite records, deadlines/expiry and no ciphertext retry. P19 TLS remains
supported; this changes no financial, custody or margin policy.

Local full pinned workspace and native/WASM/verifier/actual-browser runners pass.
New checks include 6 core assembly/entropy regressions, 3 service expiry/fence/
clock failures, bounded relay regressions, and 8 actual HTTP/SDK process groups
each in Node 24.21.0 and Chrome 154.0.8037.93. The HTTP groups cover all eight
signed commands, agents/owner-only authority/revocation, cross-account refusal,
fresh boots, lost committed replies, restart/dedupe, finite records and witness
failure. Test data, quote root, wallet binding and storage key are disposable;
there are no actual fills/payouts or live funds. At first published source head
`8cf0772`, hosted plan/handoff passed; Rust, browser and Anchor jobs were still
running. CodeRabbit's status is not a completed review disposition. This subsequent
publication receipt is documentation-only and needs its own exact-head checks;
do not inherit the source or #48 results.

Next: review #50 and its new-head checks, then one further grouped PR for
authorized reads/WebSocket + joined offline acceptance above it in native stack #49.
All four P21A criteria remain open until complete review/merge. API → Mintlify →
recovery ordering is unchanged. Fresh changed-image/NSM/native workflow qualification
remains P23. No AWS, external venue, configured wallet, container, subagent or
merge was used for this implementation.
Preserve unrelated `docs/user-journey.md`/`stays/` and ignored research.

### Historical pre-provider-selection handoff (superseded)

Current work: `tee/p21a-browser-attestation` in `cinder-tee`, stacked above
P21A.1 #47 at `9aefdd9d3478870ccf58b3cc1cf1ade449f13cd3`; trunk remains
`product/tee-v1` at `76f32956e3ee5d31707ec1bf7fa81f8711666c97`.
The #47 cleanup fix is green on all four hosted jobs at 3c1b593. CodeRabbit's
sole minor ADR status-clarity finding is fixed on its owning base PR at 9aefdd9
and its thread is resolved; all four hosted jobs also pass at that docs-only head.
No merge is authorized here. The candidate source checkpoint is
`f73b879c9d051a52b1b7726d353ef83f89f4d36c` after native-stack rebase; all tool/workflow bytes are identical to
locally tested `48ca7c1`, with only documentation/base alignment changed.

The new candidate supplies independent browser AWS-root/X.509/COSE validation,
a deliberately narrower strict certificate profile, exact web-purpose context
and shared native/WASM binding vectors. Forty signed synthetic cases use the
existing separately compiled native verifier as a local-only certificate/TLS
oracle. Actual Node 24.21.0 and Chrome 154.0.8037.93 pass fourteen groups each,
including verified-key native NK and matching final binding with no private
carrier marker. A historical five-certificate AWS path is checked at capture
time only; its expired leaf refuses later. The default AWS-only entry point
rejects synthetic roots. The shipping Cargo graph and P19 Node TLS are unchanged.

Native debug/release tests: twelve each; dependency/process regressions: nine;
native/WASM strict Clippy, formatting and bundle pass. The root workspace guard
and 29 repository tests pass. Ready PR #48 is published above #47 through native
stack #49. All four hosted jobs pass at first publication head
`cff23613fdb26c780e7487edc320328876fc6a9b`; a successful CodeRabbit status alone is
not a completed review disposition. Any later docs-only receipt head needs its
own checks; do not reuse base-head green checks. No shipping HTTP/WS handler,
fresh browser hardware attestation, zeroization proof or full phase closure.

At subsequent docs-only head `43516d7f130b46be92eee51bb050adf2ae1fd5f5`, plan,
Rust and Anchor jobs pass but the browser job rejects a nominally valid synthetic
CRL-metadata certificate path. Local forcing of the generator's redundant-zero
serial reproduces native rejection: masking the random sign bit could leave an
unnecessary leading zero in an ASN.1 INTEGER. Fixture-only canonical positive
serial encoding fixes that defect without changing either verifier. Deterministic
leading-zero/sign-padding regressions and the full pinned runner pass (12 native
tests each profile, 9 original guard/process tests, 2 new serial tests, and 14
groups each in Node/actual Chrome). The first source fix awaits its own hosted
results after push; the earlier green receipts are not its checks.

Next: review the published verifier/context slice and its exact-head evidence.
Separately qualify reviewed secret-lifetime hardening,
server NSM web quotes/entropy and SDK lifetime/fencing before HTTP integration.
No Snow fork or production crypto choice has been made; its open erasure gap
must not be hidden by dropping a wrapper or browser worker. All P21A acceptance
criteria remain open. API → Mintlify → recovery ordering is unchanged. Preserve
unrelated `docs/user-journey.md`/`stays/` and ignored work. No AWS, live venue,
wallet, container or subagent was used.

Remaining PR target: two, not a PR per submilestone: transport hardening +
HTTP/shared SDK, then reads/WebSocket + joined offline acceptance. A material
dependency decision remains before that implementation: qualify a narrowly
scoped pinned local/upstream secret-lifetime patch (recommended candidate), or
replace the provider and repeat interoperability/browser qualification. No patch
is approved/adopted here. Continuing Node TLS alone would leave browser scope
unmet. Stop for the user's decision; do not silently maintain a crypto fork.
Changed-image Nitro/actual venue qualification remains P23.

### Historical P21A.1 publication handoff (superseded)

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
First bounded slice is [#47](https://github.com/arnabnandikgp/cinder/pull/47), ready
for review on `product/tee-v1` (base `76f32956e3ee5d31707ec1bf7fa81f8711666c97`).
Initial source checkpoint: `e4a4ec34ea3fd5fe6a0fe0bace67f10d26aa4f8a`;
publication receipt `a96b5cc5d1c696ff9089d6dfe9d85a3c6b8a7b03` was docs-only.
At a96b5cc the plan, Rust and Anchor jobs passed; the browser job passed all
native/Node/Chrome channel checks but failed in temporary-profile teardown
(`ENOTEMPTY`). An owned Chrome helper was still writing after its leader exited.
Fix `3c1b593ae3731bea5c5fa619023ef20a546006a7` stops the freshly spawned process
group, clears bounded wait timers and retries profile deletion finitely. Two local
process regressions and the complete isolated runner pass; all four hosted jobs
also passed at that exact head. CodeRabbit's completed review has one minor ADR
status-clarity finding, now corrected to distinguish qualified isolated core from
unqualified shipping channel/fresh browser hardware. This review fix is docs-only;
its new-head hosted checks remain pending, not inherited from 3c1b593.
[ADR 0021](../architecture/0021-confidential-web-api.md) is tracked and the separate
`tools/web-channel/` Rust/WASM core uses pinned Snow 0.10.0 without altering the
shipping graph or P19 Node TLS. No HTTP trading handler or stream is implemented.

Native tests and actual Node/browser round trips pass with explicit synthetic
responder-key trust. Independent browser Nitro verification is NOT implemented;
the opaque library's key-erasure limitation is recorded before shipping. The full
repository runner also passes (debug/release workspace, 29 repository tests and
17 SDK/process tests). Next: review #47 and its new Linux/browser CI, then qualify
hardened dependencies, browser verifier and exact attested binding
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

Closure, 2026-10-03: all four PLAN criteria and six submilestones are complete
for the approved offline scope. Final exact heads #47 `9aefdd9`, #48 `df1e71f`,
#50 `4841c70` and #51 `e54d429` each had all four hosted jobs successful.
Relevant review threads were addressed/resolved; #48's last source follow-up
could not receive another bot run because of its limit, disclosed before merge.
The user authorized merge. `gh stack merge 49 --yes --merge` completed; fresh
GitHub PR states and fetched local trunk confirm the same merge commit.
The stricter stream-termination regressions supersede the earlier false-pass
closure evidence. Node/actual Chrome each passed all 24 joined groups at that
review fix, with all 24 SDK/assertion/TLS tests. Earlier broader Rust/core checks
keep their own source checkpoints below. No live native action, changed-image
Nitro qualification, production release or secret-erasure guarantee is claimed.
PR: https://github.com/arnabnandikgp/cinder/pull/51.
Merge: 05303740dd00129eeb038c9963f44301405725d7.

Earlier implementation records below are historical, not current merge status.

| Milestone (ADR 0021) | Progress | Completion boundary | PR grouping |
| --- | --- | --- | --- |
| 1 — Contract and isolated core qualification | closed | Reviewed contract and offline native/browser vector and hostile-wire evidence, merged; not a hardware qualification. | #47 |
| 2a — Browser verifier and binding qualification | closed | Independent AWS/COSE/X.509 validation in a tested strict subset, exact binding vectors and native differential fixtures, reviewed/merged; not a hardware claim. | #48 |
| 2b — Secret lifetime and transport hardening | closed | Selected upstream Snow with explicit erasure limitation, NSM entropy/quote adapters and offline SDK lifetime/fencing/assembly; customer-release hardening remains separate. | [#50](https://github.com/arnabnandikgp/cinder/pull/50), with 3 |
| 3 — HTTP and shared SDK/service | closed | Encrypted browser/Node commands reach the same API/journal with lost-response/restart reconciliation; offline evidence reviewed and merged. | [#50](https://github.com/arnabnandikgp/cinder/pull/50), with 2b |
| 4 — Authorized reads and WebSocket | closed | Own-account projections/paging and bounded revocable committed streams; offline evidence reviewed and merged. | [#51](https://github.com/arnabnandikgp/cinder/pull/51), with 5 |
| 5 — Joined adversarial acceptance | closed | Both transports satisfy all four parent-phase criteria; stricter termination assertions pass; reviewed and merged, not production-qualified. | [#51](https://github.com/arnabnandikgp/cinder/pull/51), with 4 |

Work: API → public docs → recovery ordering and full API_SCOPE approved 2026-10-03; P21 is not an API prerequisite. Started tee/p21a-web-api-contract on P20 merge 76f32956e3ee5d31707ec1bf7fa81f8711666c97. After the frontend/bot explanation, the user explicitly approved bounded Noise NK qualification. ADR 0021 carries the contract, permission/provenance matrix, bounds and split. Isolated tools/web-channel supplies one Rust core, thin WASM binding, synthetic native responder, published known answer, adversarial tests and disposable browser harness; no shipping dependency or existing Node TLS change.
Verification: primary browser/Noise/Snow/RFC/AWS sources checked. Native debug/release tests: 10 pass each. Node 24.21.0 and actual Chrome 154.0.8037.93: seven groups each pass, including standard vector, encrypted native round trip, early/replay/tamper/reflection/size refusal and RNG failure. Carrier marker absent in both directions. Complete isolated runner passes: four dependency-guard tests, native/WASM strict Clippy and formatting. Two new teardown regressions pass, including a SIGTERM-resistant descendant writer after its leader exits; the complete isolated runner passes again after this fix. At docs-only a96b5cc hosted plan/Rust/Anchor jobs passed and Linux Chrome 154.0.8037.57/Node passed all channel groups before the cleanup race failed the job. New-fix hosted CI is pending (first evidence-based CI fix); no container run needed. Earlier full pinned repository runner passed default/all-feature Clippy, workspace build/debug/release tests, 29 repository tests and all 17 SDK/process tests; that is not a fresh full-repository run for this harness-only fix. Foundation: 27 phases/36 documents; focused 13 plan-validator tests and diff check pass. On-chain source unchanged, so no local SBF rerun. No HTTP/WS financial workflow or fresh browser Nitro attestation claimed. All four PLAN P21A acceptance criteria remain open.
Unexpected: Snow lacks qualified zeroizing Drop for opaque key/chaining state and has no formal audit; this candidate cannot silently become production crypto. Removed unused Snow std feature (otherwise enables ring/Blake2), pinning only selected suite/entropy and 43 registry packages. Existing risk_report uses journal cut time and needs qualified read-time validation. READ remains own-account scoped. Tool setup used task-local downloads, not global replacement; no AWS, live venue, wallet, container or agent.
Review continuation: all four hosted jobs passed at cleanup-fix 3c1b593ae3731bea5c5fa619023ef20a546006a7. The single completed CodeRabbit finding is a valid minor status-clarity issue; fixed in ADR 0021 and locally validated with the foundation checker, 13 focused tests and diff check. All four hosted jobs also pass at documentation-only review-fix 9aefdd9d3478870ccf58b3cc1cf1ade449f13cd3, and the finding's thread is resolved. No production/API gate closes.
Next: #47 remains ready against product/tee-v1 at 76f32956e3ee5d31707ec1bf7fa81f8711666c97. The browser verifier/context layer is published above it as #48 through native stack #49. Keep source-head and docs-head CI evidence distinct; do not merge automatically. Dependency secret-lifetime hardening remains separate before HTTP/shared SDK, reads/WS and joined acceptance. No parent-readable fallback or execution/onboarding expansion.
PR: https://github.com/arnabnandikgp/cinder/pull/47.
Merge: none.

### P21A.2a continuation — independent browser verifier/context

Work: tee/p21a-browser-attestation above #47; isolated pinned PKI.js/ASN1.js/WebCrypto
verifier, closed CBOR and AWS-shaped typed certificate constraints, fixed AWS root,
purpose-separated exact 430-byte web context, prologue and P18-sized binding.
Shared Rust/WASM/native canonical functions agree with WebCrypto vectors; only
confirmed endpoints expose the handshake hash. Fresh synthetic signing keys are
never exported to the public corpus. Forty cases use independent native checks;
the native oracle is not a client trust service. Added root/path/signature/time/
issuer/context/CBOR/substitution/entropy/input-snapshot tests and historical public
AWS DER certificates, without promoting the original quote or research tree.

Verification: pinned complete isolated runner passes twelve native tests in each
debug/release profile, nine dependency/teardown regressions, native/WASM strict
Clippy and formatting. Node/Chrome 154.0.8037.93 pass fourteen groups each; verified
synthetic key -> native Noise -> both confirmations -> matching binding/private
echo passes. Historical AWS path validates at capture time only and expires later.
Root workspace guard and all 29 repository tests pass; no shipping Rust/Anchor
source changed and no fresh full workspace/SBF run is claimed for this layer.
All four hosted jobs pass at first publication head cff23613fdb26c780e7487edc320328876fc6a9b;
review disposition remains separate, and later receipt heads need their own checks.
No fresh NSM web quote or
production API/crypto approval; all four parent-phase criteria stay open.

Unexpected: PKI.js needs explicit path-length checks, and actual AWS CA paths
include noncritical CRL metadata while the leaf omits identifiers and has
noncritical KU. Historical hardware-derived public certificates found these
compatibility details before publication. The candidate is intentionally stricter
than general OpenSSL PKIX (e.g. ES384 certificate signatures only); unsupported
shapes fail closed. Native trust/revocation policy is unchanged. NPM lifecycle
scripts are disabled, exact manifest/lock/integrities guarded, build-only esbuild
separate. Snow erasure remains open and no vendor fork has been selected.

CI continuation: docs-only 43516d7 passed plan/Rust/Anchor but failed while
building the signed corpus, before Node/Chrome groups. The native oracle rejected
a nominally valid CRL-metadata path. The synthetic serial generator could produce
a nonminimal INTEGER; deterministic forced leading-zero entropy reproduces a
positive-case rejection locally. Fixed only fixture serial encoding, with positive,
minimal, nonmutating boundary tests and real certificate/native/browser-verifier
regressions. The complete isolated runner passes again, including actual Chrome;
no verification constraint is relaxed. First source-fix hosted checks remain
pending. No container or new live/hardware run is needed/claimed for this defect.

Next: review the bounded layer above the clarified #47 base (9aefdd9), then
P21A.2b dependency/secret-lifetime and remaining transport gates
before HTTP/SDK integration. Do not close the parent phase or merge automatically.
PR: https://github.com/arnabnandikgp/cinder/pull/48 (ready; native stack #49 above #47).
Merge: none.

### P21A provider-selection continuation — 2026-10-03

Work: user selected Snow for V1 after the transport-alternatives research. Recorded
pinned upstream 0.10.0, shared native/WASM Noise NK, no custom handshake/fork and
the explicit opaque-secret erasure limitation. Offline implementation with
synthetic data is permitted; production dependency/security review is not waived.
Updated PLAN/ADR and the tip handoff; HTTP/WS implementation has not advanced in
this decision-only continuation. Existing P19 Node TLS is unchanged.
Verification: #48 source-fix head df1e71f9110c4c4b940c689db4540839656ec473 has all
four hosted jobs green. Decision-doc foundation validation (27 phases/36 docs),
all 13 focused validator tests and diff check pass. No fresh crypto, API, SBF or
hardware receipt is inferred from this selection.
Unexpected: key-erasure hardening and client-to-enclave confidentiality are
different requirements. The known cleanup gap remains visible before customer use,
without holding synthetic-data API integration behind an unselected fork.
Next: implement P21A.2b/3 on tee/p21a-http-sdk above #48; server web quotes/entropy,
SDK bounds/lifetime/fencing and HTTP commands over the same financial journal.
Keep the two-further-PR target and all four parent-phase acceptance gates open.
PR: none yet for this branch; stack #49 above #48 and #47.
Merge: none.

### HTTP/shared SDK implementation, 2026-10-03

Work: grouped milestones 2b/3 on `tee/p21a-http-sdk` above #48. Promoted the one
Noise core into an explicit guarded root package; the WASM workspace compiles its
source through a frozen manifest path. Added NSM-only RNG resolution and distinct
web quotes, finite socket/session ingress and exact complete-response framing.
Explicit `--web` selects the existing enclave runtime, not another financial
server. Node built-in HTTP/net was selected for a fixed-target opaque parent
carrier with no new runtime framework dependency. AWS-only web SDK and public
bundle use the same signed command client and independent browser verifier.

Verification: pinned complete root runner passes default/all-feature Clippy,
workspace build, debug/release tests, 30 repository guard/vector tests and all
17 existing SDK/TLS-process tests. Complete isolated runner passes original 12
native tests per profile, 9 dependency/teardown tests, 2 serial regressions,
native/WASM Clippy/build and 14 original verifier/channel groups per Node/Chrome.
New HTTP suite passes eight groups each in actual Node/Chrome, with accepted
cancel/payout/leverage receipts, scoped agent signatures and revocation, encrypted
errors, record exhaustion, killed-service restart and no second economic attempt
after a lost reply. Core maximum-reply/entropy and service fence/clock/expiry
regressions pass; targeted relay checks pass after the final routing fixes.
These are synthetic offline qualifications, not fresh NSM web hardware evidence,
completed venue fills or final customer payout receipts. Full phase stays open.

Unexpected: first local end-to-end test exposed `ServerResponse.close` occurring
after a successful flush but before promise continuation. The relay now treats
only an unfinished response as a disconnect, with a regression. Full-run checks
caught a test tuple complexity warning and a fixture binary path incorrectly
depending on working directory; both fixed locally before any push. Self-review
also made fragmented reads linear/bounded, protected existing routing from a
duplicate public handle and retained a complete final ciphertext reply when the
enclave closes at the record cap. No financial change or Snow fork was needed.

Publication: ready/open #50 through native stack #49, based on
`tee/p21a-browser-attestation` at `df1e71f9110c4c4b940c689db4540839656ec473`.
Source head `8cf077298d8cc288223ba88133472d8b52187c61` has hosted plan/handoff
passing; Rust, browser and Anchor were running at the receipt check. New receipt
heads need their own checks. No review disposition or merge is claimed. #47 and
#48 remain open; their source heads are recorded in the current handoff.

Next: review #50 and its checks, then authorized reads/WebSocket with joined
offline acceptance in one further PR. The user's isolated-margin question was
answered from BASELINE/API_SCOPE:
V1 shares a private customer's collateral across markets within the configured
pool; leverage is not native/customer isolated margin. No new margin feature was
authorized or introduced.
PR: https://github.com/arnabnandikgp/cinder/pull/50.
Merge: none.

### P21A.4/5 — reads, WebSocket and joined offline acceptance

Work: `tee/p21a-reads-websocket` above #50. New signed Read version/fixed rows,
account-local filtered cursors, current READ/owner checks, original eight tags
unchanged. Existing risk/mark evaluation gives absent derived fields on stale
evidence. Actual applied ordinary fills/funding and immutable pre/post book-history
index are private projections of the authoritative journal. Active grants are
owner/self-scoped. One opaque binary socket ingress shares Handler/Session/API;
one signed read subscription coalesces replacement pages with authenticated
correlation/ordinal, fresh authorization/witness checks, finite budgets and resync.

Verification: final complete pinned workspace and web-channel runners pass;
local Node/actual Chrome each pass 24 HTTP/WebSocket process groups and 14 verifier/
core groups. Twelve bounded carrier tests, all five native web ingress tests,
25 API behavior tests, three interop tests and 19 SDK/TLS tests pass. Debug/release
workspace, default/all-feature and native/WASM strict Clippy/formatting pass.
Final joined rerun passes after queue tests synchronize real emitted frames,
not scheduler sleep assumptions. First published source `a0442d5` has all four
hosted jobs passing. A narrow unused-iterator cleanup follows it: strict SDK
typecheck/all 20 SDK/TLS tests, public bundle build and Node/actual Chrome's 24
joined groups each pass after the fix. New-head CI/review is not inherited from
the first publication; the broader workspace/core receipt remains at `a0442d5`.
No source/check receipt is inherited from earlier PRs. See exact contract for
command/history/stream scope and synthetic-vs-live boundaries.
Unexpected: final reply flush must precede orderly socket close; incoming traffic
cannot starve stream checks; a five-second per-read timeout is not an absolute
partial-frame budget. Added bounded absolute delivery guards and safe shutdown.
Dropped-notification harness must assemble public TCP frames before dropping.
Async-generator `finally` does not run if `return()` precedes first `next()`;
an explicit single-consumer lifecycle wrapper now closes both iterator layers.
The joined abandonment regression synchronizes public delivery so its discarded
read cannot race the next fixture read's journal lock.
No dependency/security/economic profile changed; Snow erasure remains unqualified.
Next: review #51's current source/checks, then bottom-up stack merge only when
authorized. P21B is the next planned implementation, not started in this slice.
PR: https://github.com/arnabnandikgp/cinder/pull/51, stacked above #50 in stack #49.
Merge: none.

Review follow-up: CodeRabbit's one actionable finding at `9fa8748` is a legitimate
assurance gap, not a demonstrated runtime bypass. Reproduced a stalled iterator
passing the old timeout/rejection composition. Replaced all five terminal checks
with a shared test-only closure assertion; actual rejection/`done: true` passes,
timeouts/pages fail. Four deterministic regressions are included in the normal
SDK runner. Strict typechecking/all 24 SDK/assertion/TLS tests and Node/Chrome's
24 joined groups each pass. Prior closure evidence is superseded by this rerun;
all four hosted jobs passed at `9fa8748`, not automatically at the review-fix head.
No production source, policy, native action or dependency changed. Published fix:
`4cf0462c91f1ac424e9ecf9ebee29845500e6b58`; its review thread is replied/resolved.
Next: inspect the new-head CI/review, then await authorized bottom-up stack merge.
No automatic merge; subsequent docs-only receipt checks are not inherited.

## P21B — Public documentation

Work: started `tee/p21b-public-docs` on P21A merge `0530374`. 37 MDX content pages cover product/account/funds/trading/agents, security/risk/recovery, SDK/authentication/units/lifecycle/errors, eight commands, ten private read families, paging, WebSocket and limits. References use native Mintlify request/response examples and field schemas, while accurately identifying the encrypted binary carrier. Content is for the user's existing separately hosted Mintlify repo; no new site, config, package, dev server or CI is included.
Verification: source mapping and the initial 26 temporary documentation/codec checks, strict TypeScript, Mintlify validation/local links and a real-browser order-page preview passed before the format revision. Final native-component MDX: all 37 pages compile with temporary MDX 3.1.1; all 158 internal page links resolve; all 56 API examples strictly typecheck with pinned TypeScript 7.0.2 against merged SDK types. All 18 command/read pages contain the reference components and real encrypted carrier with playground disabled. The foundation checker (27 phases/37 documents), all 13 validator tests and tracked diff check pass. Final rendering in the user's existing Mintlify repo is separate; the earlier preview is not claimed for this revision. No hosted publication, live endpoint or independent audit is claimed.
Unexpected: the initial local site/tooling scaffold exceeded the intended content-only scope and was removed after clarification. Public pages stay `.mdx`, not `.md`; the README is only a copying/navigation note. Temporary validation tools stay outside the repo. Structured examples must document SDK objects without implying a plaintext JSON exchange or unsupported interactive playground.
Next: local delivery is complete and its tracked receipt merged with P21. The user supplied `docs.json`; all 37 navigation entries resolve and all pages compile after replacing relative `api:` metadata with ordinary carrier text. `/docs-content/` is ignored and contains no tracked files. Per user instruction no docs-source PR was created. Hosted publication remains separate.
PR: https://github.com/arnabnandikgp/cinder/pull/52.
Merge: 1c2c3eb836003e9aeb252e85e3aacdf5eb32ecf7.

## P21 — Recovery integration

Work: completed the approved offline phase on `tee/p21-recovery-integration`, base `0530374`. One accepted journal derives complete settled claims and P16 payment bases. Explicit recovery permissions fence ordinary paths, bound closes/returns and durably seal dispatch. Wallet-authorized RSA-3072 keys are immutably registered before cutover; OAEP-SHA256/AES-256-GCM kits require both ciphertext readbacks. The governed signed manifest and independent Node claimant retain exact P17 root/context/counter semantics. Actual SDK HTTP/WebSocket outages join normal payout, return, sealing, replica failure and independent compiled SBF claims. No parallel seeded recovery ledger or amount supplied by a claimant exists.
Verification: all three PLAN criteria are checked for offline scope. Final pinned workspace runner passes 30 foundation/boundary tests, 12 carrier tests, both strict Clippy configurations, all-target build, workspace debug/release and all 24 SDK/TLS tests. All 55 vault codec/SBF tests pass. Full native/WASM/browser runner passes, including 14 verifier/core and 24 joined process groups each in Node/Chrome. Focused additions include 16 journal cut tests, recovery-close allocation/replay, 26 passing P16 funding tests, actual signed bounded recovery dispatch and four package/key/readback/tree tests. Limits/commands/environment are in Current handoff; known Nitro tests remain explicitly ignored, not silently counted as passing hardware. Initial failures and fixes are recorded there. Earlier slice-only receipts are superseded, not rebranded as final checks.
Unexpected: integration revealed token-recipient, create/cancel routing and synthetic fixture margin/readiness mismatches; these were corrected without weakening guards. New controls/state require engine revision 21 and reject old revision 20 histories pending explicit migration. Delivery evidence precommits store/locator layout before root construction; separately signed final ciphertext digests avoid a circular commitment. Package availability is not code-enforced live store independence or a proof of complete liabilities. Dedicated keys are retained, not derivable/reissuable by the operator.
Review follow-up (2026-10-04): both CodeRabbit findings at `bfe4ae64e05bb3504df9332a83149abbf7e6ecbc` were relevant. Source checkpoint `8e528db870d81a7cd8011780726b8ab32795b1be` fixes register-once/exact-key resume and strengthens both joined carrier tests with positive restart, missing/replacement-key refusal and an epoch-only fencing probe. The default vault setup now documents/provisions the recovery prerequisites and reports missing/wrong bindgen clearly; mandatory full-suite recovery coverage remains. Full `scripts/check.mjs`, DEFAULT `scripts/check-vault.mjs` (55 tests) and two prerequisite-diagnostic checks pass at this source checkpoint; see Current handoff for scope and limits.
Publication: review fixes pushed at `521dcceed2b3bcc063140061407af9dfb43ed1e4`; both CodeRabbit threads replied/resolved. Plan CI passed at that head; Rust/vault/browser CI were still running, not success receipts for this documentation-only successor.
Merge follow-up: all four hosted jobs and CodeRabbit passed at `ea23a17abcf8d91525962ce6ec2c50579ed21b66`; the user authorized merge. Two new minor README findings are fixed (PATH fallback and explicit fake native-fencing evidence); the date warning is explained by Asia/Kolkata versus UTC. Foundation validation, all 13 validator tests, both shell fallback cases and diff checks pass locally. This docs-only successor needs its own hosted checks before merging; runtime evidence above is unchanged.
Merge disposition: all four hosted jobs passed at `ea23a17abcf8d91525962ce6ec2c50579ed21b66`; all five review threads were resolved. Final head `ff3b4c3752656ffeffdc1b88e92da0a223b5f710` contains only the two minor README corrections and bookkeeping, verified with the foundation checker, 13 validator tests, shell fallback cases and diff checks. The user explicitly waived waiting for that docs-only head's CI rerun; no fresh executable success is inferred.
Next: P22 expands complete offline adversarial lifecycle after authorization; P23/G01/G03 own live capability/source/store/entropy and changed-image qualification. Public recovery MDX remains local/untracked with the user's docs.json preserved. Initial implementation/focused rerun checkpoint `dcb685f277d2c04e1256258d7cb3b10d2c0be266` is historical, not the reviewed source head.
PR: https://github.com/arnabnandikgp/cinder/pull/52.
Merge: 1c2c3eb836003e9aeb252e85e3aacdf5eb32ecf7.

## P22 — Offline adversarial acceptance

Work: full HTTP/WebSocket normal lifecycle with actual SDK/relay/private service, Gateway/funding Controller, risk-derived holds, replicated AEAD journal and SBF vault movements. Checks each independent customer/house/native book, physical cash, positive claims, holds, source cuts, late fills, funding settlement, streams and final recipient payment. A separate widened-integer reference covers 4,632 position transitions and eight fixed seeds/576 financial cuts; reverse stress retains net-flat shortfall counterexamples. The 25-case manifest links all W01–W09/V01–V10 to existing and new executable tests; default CI includes the manifest and joined tests without another duplicate job.
Verification: `node scripts/check-acceptance.mjs` passes at source `f14a4400f9bf3664193f51fb5c055dc10bccac65`, using Rust 1.97.1, Node 24.21.0, Anchor 1.2.0, Surfpool 1.5.0, Agave/SBF 3.1.10/platform tools 1.52 and wasm-bindgen 0.2.129 on local Apple silicon. Default workspace debug/release, Clippy and SDK checks: 91.402s (600s budget). Real Node and Chrome 154.0.8037.98 each pass 14 core/attestation and 24 HTTP/WebSocket/private-SDK process groups: browser runner 59.101s (600s budget). Default vault runner passes all 57 tests, none skipped: 456.807s (1,500s budget), including both normal and recovery carriers. HTTP/WebSocket normal cases take 67.768s/66.557s; each checks 64 signed reads in 64ms/47ms. Encrypted journals are 249,130/249,124 bytes and Node-harness RSS 155,566,080/158,187,520 bytes, below the declared 8MiB/512MiB bounds. These are test-workload measurements, not production service/Nitro benchmarks. All 16 observation regressions and three new reference tests pass, including 576 differential cuts and 289 shortfall cuts. Tracked-only source export independently passes manifest/foundation checks (27 phases/37 tracked documents) and all 33 metadata regressions with no `work/` or user docs. Earlier failed/focused runs are diagnostic history, not the final success receipt. Actual hardware/live qualification is intentionally not run; crash-helper tests marked ignored are invoked explicitly by their parent matrices. Hosted CI/review for the published head remain pending.
Unexpected: ordinary native history parsing has no certified causal cut; introduced an explicit trusted covered-input port, not inferred completeness or an enabled live provider. The fixture control codec required canonical decimal strings for lossless i128 values. Initial SPL mint setup expired under implicit helper defaults; explicit execution/confirmation settings match the existing sandbox sender. Duplicate native observations correctly advance audit version while economics stay unchanged; the regression now asserts both facts. The first reverse-stress range included downward shocks; its named upward-shock search now starts at 100. Regenerated Rust caches were removed after disk exhaustion and the pinned WASM target installed; no source/research or global toolchain versions were changed.
Publication: ready/open #53, `tee/p22-offline-acceptance` → `product/tee-v1`, using native `gh stack init/submit`. Initial published head `ed32d9f6624b5b6ec705d95227d5e72ecdf6947e` contains only evidence documentation after the fully tested `f14a4400f9bf3664193f51fb5c055dc10bccac65` source. This final bookkeeping-only successor records the actual PR and passes focused foundation/plan/manifest/diff checks. Hosted Linux CI and CodeRabbit review remain pending for its own head; no automatic merge.
Review follow-up — 2026-10-06, tested source `b7ba0c9bb28bec6b58b2a8d8c4a5c87e9514dda2`: all four hosted Linux jobs passed at `9a4174527757e120ee81bfaf8d073480b153a37e`, but the two minor CodeRabbit findings were valid. Deposit/reconciliation controls now inspect normalization receipts; reconciliation also rejects an open NativeSnapshot issue for its own event. Zero deposits, changed same-signature amounts, mismatched native cash and repeat mismatches return errors without erasing committed evidence. Valid duplicate deposits and separate matching snapshots remain accepted. Both joined carriers reproduce the original false-success failure before the fix, then pass with unchanged financial books and retained restart state. Local `node scripts/check.mjs` passes complete debug/release, Clippy, boundaries and all 24 SDK/TLS tests. `node scripts/check-vault.mjs --acceptance-only` passes strict build/IDL/TypeScript checks and both joined normal lifecycles (71.216s HTTP / 68.169s WebSocket); its four reported passes include the two empty filtered file wrappers, not a new 57-case full-suite receipt. The manifest now checks bounded declaration/registration forms, rather than arbitrary substrings; nine stale-comment/string/renamed-entry substitutions are rejected, and all 34 metadata regressions pass after the final edit. Declaration presence is not proof of runner reachability or execution. No shipping protocol, Solana program or economic policy changes. New-head hosted checks and thread disposition are pending publication; earlier source receipts are not reused as new-head success.
Review publication receipt: source/evidence pushed at `baf340f239223067d95b856e17bf28d14b67c47e`. Both CodeRabbit threads were answered and resolved; hosted checks were queued when inspected. The ensuing documentation-only successor records this receipt and still requires its own current-head hosted results. No merge was requested or performed.
Merge closure — 2026-10-06: all four hosted checks passed at reviewed head `24fe0796fbcfb2151de779922ae1932defc745f6` and both relevant review threads were resolved. The user authorized merge. Native `gh stack merge 53` reported that the PR was not a remote stack; exact-head-guarded `gh pr merge --squash` succeeded and the merge receipt was independently verified. Local product trunk was fast-forwarded to the resulting merge.
Next: the user authorized a separate CI follow-up for audit recommendations 2/3/4 only. Preserve fake-port versus SBF evidence limits; P23 and all live gates remain unopened. The next live phase requires a current bounded G05 environment/account/funding/cleanup manifest, not reuse of P20 permission or historical experiment receipts.
PR: https://github.com/arnabnandikgp/cinder/pull/53.
Merge: 8247d24fa6d77b1e0b478ef7b8513ace80f34d4b.

### CI hardening follow-up — 2026-10-06

Work: source checkpoint `297327faca999977581ed407e7acc9db3e1ad828` implements audit recommendations 2/3/4.
Maintained Node test files are discovered and assigned, actual required acceptance
execution is observed, and ignored/skip statuses require the documented policy.
Shipping-default Rust execution joins the debug/release all-feature profiles.
Contracts, lint, Rust and SDK/relay run independently; a separate network-disabled
ARM64 package job checks the default release, application-crate reproducibility
and ordinary-host boot refusal. No live/hardware tests become routine CI.
Caching follow-up: the user additionally authorizes caching in #54. Pinned
dependency-only Rust caches cover the root, WASM, Solana/SBF and isolated ARM64
build targets; npm caches use lockfiles without saving `node_modules`. Public
tool downloads remain checksum-verified on restore; SBF platform tools have an
exact version key, and the public ARM64 tool-only image uses BuildKit caching.
Both application builds in the ARM64 reproducibility gate remain mandatory.
No test result, IDL, fixture state, wallet, provisioning or credential is cached.
Verification of caching source `28a78924254581d6b3aaed09f7be4e19bcaf9e6c`:
the pinned Node contracts entrypoint passes 41 guards and workspace boundaries;
official actionlint 1.7.12 validates all four workflows, and shell syntax passes.
The exact source export is qualified with the existing pinned ARM64 image,
read-only public source/vendor and no network, NSM, git/research or credentials.
Cold and populated-cache runs both pass as the host UID/GID, build all seven
shipping-default tools, explicitly rebuild the application a second time,
compare identical ELF files and refuse ordinary-host boot. The warm run also
cleans/rebuilds the application before its initial build. These local runs verify
the cache mount and mandatory application checks; they do not execute GitHub's
cache service or establish hosted speedup/retention behavior. Public build outputs
and logs stay in the task's explicit temporary cache directory for reuse; both
test containers are removed automatically, and no builder is running.
Verification: complete pinned local workspace checks pass: 41 script guards,
12 relay and 24 private SDK/TLS tests, both strict Clippy configurations and
all-target build; Rust all-feature debug/release each pass 400 tests (10 intentional
ignores per profile), with 380 passing shipping-default tests (9 intentional
ignores). The shipping-default profile also passes without any feature-disabling
flag. Final guard-only checks pass after the small reporting/receipt-shape edits.
Full browser runner passes 11 tool/fixture guards, 12 relay and both isolated Rust
profiles; actual Node/Chrome each complete 14 core/attestation and 24 joined
HTTP/WebSocket groups. ARM64 packaging passes on exact committed source `297327f`
with no network, source/vendor read-only and no git/research/host credentials.
Pinned base digest is `2775a09d208ff0d7c1f50490c45b62db929e87ba1dcbc3f2132ac71a704bcdd3`;
local tool-image index is `33107aa751a9d69fb65acb26474fe3954dd30f8580b4150b4a63daf58bac434a`,
ARM64 manifest `d3d2f4db64dfa8e64503cc08102b68c70d9af963cc9508d0bc7927934dd814e2`.
Linux 6.18.35 / Rust 1.97.1, OpenSSL 3.0.22-1~deb12u1 and pkg-config 1.8.1-1:
all seven default-feature release tools build, the recompiled application ELF is
byte-identical, and ordinary-host boot refuses with exactly `cinder runtime fenced`.
The unchanged complete SBF/vault rerun passes all 57 tests, none skipped, in
456.083s of test execution, including both normal and recovery carriers, locked
SBF/IDL/source equality and strict Rust/TypeScript checks. Final reporting and
browser-receipt-shape edits pass all 41 guards; actual Node/Chrome carrier
qualification each passes 24 process groups again after that final receipt edit.
These are offline tests, not EIF/PCR, actual-hardware or live financial receipts.
Unexpected: local guard qualification caught module-qualified Rust output names;
exact terminal-name matching now has regression coverage. The first browser
receipt edit performed an unused extra fixture setup, correctly rejected by the
parent-observation oracle; that extra setup is removed. Neither required weakening
an acceptance oracle or changing financial logic. The first full SBF run failed
one normal WebSocket stream with a channel-closed error during concurrent builds;
the runner rejected the failure AND missing required execution. Its unchanged
full rerun without those builds passes all 57 tests. The root cause
of that isolated closure is not established; no timeout/oracle was relaxed and
automatic reruns are not added to CI.
Hosted browser prerequisite correction: job `112092854155` at initial published
head `d655f39` fails with `cinder-verify-fixture ENOENT`. Automatic discovery moved
the previously late serial tests ahead of their native prerequisite. The fix
moves the existing locked/offline fixture build before the discovered web suite.
Fresh-export complete browser qualification passes at source `ad136c6` before
the first evidence-based CI fix push: no preexisting native/WASM outputs,
locked/offline Cargo and npm preparation, all 11 tool and 12 relay checks,
native debug/release, WASM/strict checks, and 14 core/24 carrier groups each in
actual Node/Chrome. Local Linux reproduction is unnecessary for this platform-
independent missing-build cause. This evidence is separate from warm-target
results and does not claim the new hosted jobs already passed.
Hosted independent SDK prerequisite correction: at source `97c7fdc`, six hosted
checks pass, but SDK/relay job `112097236435` encounters `cinder-relay ENOENT` in
seven process tests and reaches the 15-minute bound with partial fixtures alive.
Splitting CI groups removed the previous all-target build that supplied that
binary and the production verifier. Corrected source `9c0e5c1` explicitly builds
the shared four-binary list and checks executable prerequisites before any SDK
fixture starts. The production-root negative test cannot pass solely because
its verifier is absent. The new regression checks early entrypoint failure,
each missing prerequisite and non-executable files. The exact committed-source
export has no existing Cargo target, git/research/user files or built binaries;
offline locked preparation and the complete independent client group pass
12 relay and 24 SDK/TLS tests, strict TypeScript, plus 42 contract/inventory guards
and workspace boundaries. No protocol code, feature pin, timeout or financial
assertion changes. This is the second evidence-based CI fix push; do not reuse
these Mac receipts as new hosted Linux results or guess at a persisting failure.
Publication: ready/open [#54](https://github.com/arnabnandikgp/cinder/pull/54),
`tee/ci-offline-gates` → `product/tee-v1`, initially published at
`d655f39776ac777ec9617a4c5b743b187381dd55` using native `gh stack`. The exact
committed source-only export passes all 41 guards and workspace graph; earlier
full-suite receipts and the final focused guard/carrier checks are detailed above.
This docs-only successor records the actual publication, not fresh hosted success.
Next: await hosted checks and review/merge authorization; P23 remains open.
Merge closure — 2026-10-06: all seven checks and CodeRabbit status pass at exact
head `9d9dd760eae57cf920ac9956d6eb903c954ad43a`, with no unresolved review threads.
User-authorized, exact-head-guarded squash merge is independently verified at
`833955f8a4ead02a71eb5ae69a6aebd92bb85091`; local `product/tee-v1` matches upstream.
This local documentation receipt is reserved for the next authorized PR. No
unreviewed trunk push, deployment or P23 work occurs.

<a id="p23--live-integration-qualification"></a>
## P23 — Live integration qualification

Local continuation: added purpose-separated confidential chain ports and original
owner-deposit recognition in [ADR 0024](../architecture/0024-confidential-chain-ports.md).
The actual Controller signs only persisted/simulated mandates; unknown sends
reconcile the original wire. Authenticated fixed-origin RPC, finalized exact
transaction/SPL/receipt/counter checks and bounded full-code streaming share the
runtime's one journal. Version 2 adds a distinct Funds role and read-only finite
polling; version 1 remains compatible. Native diagnostics consume durable credits
and archive missing/unsupported schemas without manufacturing setup/credit/payment
or coverage. Funds-signing and trading activation remain rejected. Local joined
tests use synthetic RPC/clock, not real deployments or a native finality provider.
All fixed local gates pass, including final serial SBF 59/59 and the final
receipt/freeze correction's full host and exact-source shipping package checks.
No AWS/RPC/venue/wallet/deployment action has occurred.
The original initial-slice receipt below remains historical, not these results.

Checked application checkpoint: `1c423c9c6bfaf40d591b2655db7c41881d14639e`.
All pinned host gates pass in `/private/tmp/cinder-p23-local-root-final.log`:
42 contracts/inventory guards, dependency boundaries, default/all-feature strict
Clippy, all-target build, all-feature debug/release and shipping-default Rust,
12 relay and 24 SDK process tests. Actual Node and Chrome/WASM checks pass in
`/private/tmp/cinder-p23-local-browser.log`: 14 attestation groups and 24 joined
carrier/SDK groups in each runtime, with synthetic fixture trust explicitly
retained. The five release tests also exercise actual local private provisioning
in both five-/six-role versions; 0700/0600 files and purpose-specific capsules.
The seven joined chain cases include original owner deposit recognition without
seeded customer cash, duplicate/restart, bad receipt/missing history, actual
component binding, lost ACK, simulation/fee refusal and pending native debit.

Network-disabled Apple container ARM64 package passes from exact committed
source export, reusing only locked public dependencies and dependency cache.
Tool image `sha256:33107aa751a9d69fb65acb26474fe3954dd30f8580b4150b4a63daf58bac434a`;
default enclave ELF `4a707a56f1e41adf93a29e1fe36b70a0b213f6d8a709dafbd57940e210c442ab`.
Independent application rebuild is byte-identical; ordinary non-Nitro boot
refuses. Local public artifacts are retained in
`/private/tmp/cinder-p23-arm64.EdZ5gW/artifacts/bundle`, package log in
`/private/tmp/cinder-p23-local-arm64.log`; the temporary container was removed.
This is an ELF/rootfs preparation result, not an EIF/PCR/NSM receipt. Exact
fixture regeneration also matches both tracked JSON files without RPC.

Local failure disposition: first SBF launch required explicit pinned wasm-bindgen;
then strict TypeScript found extra closing parentheses in the new fixture tests,
fixed before the suite ran. Full concurrent SBF run passed 58/59, with the HTTP
financial stream closing at its existing 30-second session deadline during parallel
release builds. No timeout/security boundary was relaxed. The full serial gate
passes all 59 cases with no skips in 455.498 seconds, including HTTP/WebSocket
normal and recovery workflows. Log: `/private/tmp/cinder-p23-local-vault-serial.log`.
The compiled program is 522,200 bytes, SHA-256
`90ac6651c922822e3f156357fc7f1e2a35e14f92bc8964ba362dffc9920b7310`.
The program, client and SBF test sources are unchanged by the final receipt fix.

Final receipt review reproduced rejection of an already completed payment after
the later one-way vault freeze. Recognition now accepts its immutable original
receipt through the known recovery modes and single freeze epoch increment;
pre-sign counters still reject any new action under the old epoch. Unknown modes,
regressed epochs and unexplained larger increments reject. The focused 25 chain
tests pass, including a real-journal owner deposit recognized once after freeze.
Logs: `/private/tmp/cinder-p23-local-freeze-repro.log` (expected failure before fix)
and `/private/tmp/cinder-p23-local-freeze-fixed.log` (pass after fix). Current token
state, original transaction deltas, exact receipt and approved-code checks remain;
this is not a historical consensus proof or relaxation of native settlement gates.
Final checked application source: `33dae28b2c3846d9956a7e906f5663d78d93cd76`.
The full pinned `node scripts/check.mjs` rerun passes all three Rust profiles,
both strict Clippy modes/build, 42 contracts/inventory guards, dependency boundaries,
12 relay and 24 SDK process tests. Log:
`/private/tmp/cinder-p23-local-root-freeze-final.log`. The 25 focused chain tests
also pass. No required workflow is skipped; intentionally ignored subprocess
helpers remain covered by their registered parent matrices.

The exact committed-source ARM64 package rerun passes on the same locked tool
image above, with no network, NSM, git, research or host credential mounts.
All seven shipping-default tools build; the independently rebuilt enclave is
byte-identical and ordinary-host boot is fenced with no public stdout. Final
enclave SHA-256:
`f1a725d6a023cb52528d253c6bedccc8edcb4bdd4c2d03e7f2de90998964a521`.
Artifacts: `/private/tmp/cinder-p23-arm64-final.VAUBqX/artifacts/bundle`;
log: `/private/tmp/cinder-p23-local-arm64-final.log`. The disposable package
container was removed; the pre-existing stopped builder was preserved.
Browser and SBF owning program/client/test sources are unchanged from the
`1c423c9` run above. This is completed local preparation, not an EIF/PCR receipt,
live settlement qualification or closure of any P23 live criterion. The final
documentation-only successor records these checked sources and the same draft's
PR description records its independently verified publication head/base. Final
documentation contracts/inventory/workspace and whitespace guards pass. No new
head's hosted checks or CodeRabbit review are inferred from the earlier green head.

Publication receipt: native `gh stack submit --auto` updated only draft #55,
`tee/p23-live-qualification` → `product/tee-v1`; the independently verified
published evidence head was `ed5c8abefb357a08832413c52f986595664e6ca5`.
New-head hosted checks were queued, not passed. Final documentation validation
caught a missing explicit anchor in the new receipt link. This successor adds
that anchor and reruns the full documentation/contract guard group before push;
the earlier failed log is `/private/tmp/cinder-p23-local-docs-anchor-failure.log`.
Application/program/client sources and all their test receipts are unchanged.

Prior hosted PR #55 head `11c864626e2e1840f1475f1f5a8d721b166f70c9` had all seven
checks plus CodeRabbit green and no review threads; those results do not qualify
this later source. Historical draft status was superseded by the user's readiness
request; no merge is authorized.

### Review correction — 2026-10-06

Three CodeRabbit findings at `d26945fc7c749dc3f464f720db047ff64a8ee0a2`
were independently confirmed: fixed-five capsule release blocked version 2;
an ineligible finalized owner deposit stopped the polling worker; and failed
pre-send preparation could strand an exposed no-wire reservation indefinitely.
The fixes preserve exact role purpose and error propagation; no-effect closure
requires an expired physical plan, a freshly verified head, no retained wire,
no recorded effect and no conflicting terminal history. It atomically finalizes
zero effect without erasing the identity or allowing another dispatch. Native
HTTP withdrawal, retained/uncertain sends and unverified storage cannot use it.
Joined tests cover rejection followed by valid credit, expiry/replay, failed or
durable-but-unacknowledged wire appends, fencing and retained-wire negatives.

Two later connector findings are also addressed: signature-located receipts use
`source_cut = None`; SDK requests wait at most 250 ms for the single runtime lock
and recheck lease/time afterward. Poll cadence starts after I/O, leaving a full
service interval instead of continuous slow polling. Continued contention returns
an error; this is bounded backpressure, not async I/O or concurrent journal writes.
No on-chain/client/dependency changes or financial activation are introduced.
Verification: exact application checkpoint
`5acf79bd2f931e85325841a06bd87b038cb1cf28` passes pinned
`node scripts/check.mjs` with Node 24.21.0/Rust 1.97.1,
`CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0` and
`CARGO_PROFILE_TEST_DEBUG=0`: 42 contracts/inventory guards, workspace/dependency
boundaries, formatting, both strict Clippy modes/all-target build, all-feature
debug/release and shipping-default regressions, 12 relay and 24 SDK process cases.
The all-feature service suite has 86 cases, including the new joined/controller
and lock-wait regressions. Log: `/private/tmp/cinder-p23-review-final-offline.log`.
An initial new restart test reused time 100 after committing the closure at 10000;
the monotonic-history check correctly refused. The fixture was corrected without
weakening the production guard. Final checked suites all pass.

Exact-source `git archive` packaging passes with Apple container's cached
tool-only image `cinder-ci-arm64:check` (index SHA-256
`33107aa751a9d69fb65acb26474fe3954dd30f8580b4150b4a63daf58bac434a`),
source/vendor read-only, network disabled and no NSM/git/work/credential mounts.
`tools/nitro-runtime/package-check.sh` builds all seven default-feature tools,
independently recompiles the enclave to an identical ELF, and verifies ordinary
host boot refuses without public stdout. New enclave SHA-256:
`0f5da0b061380ad16d1e139765b505ee3d6e3cdf94fc9195b5dc50975d8a1ff1`.
Bundle: `/private/tmp/cinder-p23-review-arm64.jvrOvc/artifacts/bundle`;
log: `/private/tmp/cinder-p23-review-arm64.log`. The disposable container was
removed; the pre-existing stopped builder was preserved. This is not a Nitro
EIF/PCR/hardware receipt. Program/client/browser owning sources are unchanged;
their prior separately identified receipts are not relabeled as new runs.

Publication: same ready/open #55 via native stack submission. Exact published
head/base and current hosted/review status belong in the PR description and must
be verified after push. New-head CI is pending, not inherited from `d26945f`.
No merge, AWS, RPC/venue request, wallet, funding or deployment occurred.

Work: started on `tee/p23-live-qualification` from merged trunk `833955f8a4ead02a71eb5ae69a6aebd92bb85091`. Audited shipping production connections and current primary venue docs; added bounded native GET composition over the existing Gateway budget, encrypted TLS egress and observation journal, plus regression tests and the G05 runbook. No live capability is enabled; no new dependency, custody policy or economic promise is selected.
Verification: full pinned `node scripts/check.mjs` passes on 2026-10-06 for source checkpoint `33e13646cee84be8874d01bd676bd95a518a7bb7`: 42 contracts/inventory guards, dependency boundaries, default/all-feature Clippy, all-target build, all-feature debug/release and shipping-default suites, 12 relay and 24 SDK tests. All 10 new read cases pass in each Rust profile; GET framing and authenticated GET/POST root/hostname/time negatives pass. Logs retained at `/private/tmp/cinder-p23-offline.H7uWbj/check.log`. No new SBF/browser/ARM-container/hardware/devnet/live venue run is claimed; hosted checks/reviews for the new PR remain pending. This documentation-only publication receipt is not a new application/hardware result. No program/client wire/dependency/feature changes occurred.
Unexpected: production runtime has no chain signer/RPC recognizer or observation scheduler; the five-role release lacks a Solana funds seed. Current venue docs still do not establish complete causal/funding/payment semantics. Historical research and P22 fake ports are not shipping connections or live qualification. Initial new-test failures exposed missing synthetic gateway activation and the existing conservative one-minute cooldown floor; fixtures were corrected without weakening production gates.
Next: finish offline validation and connected confidential ports, qualify source cuts/precision/setup/finality, then obtain a fresh bounded G05 manifest before actual SDK/relay/Nitro/program/venue tests. Use the [live runbook](../operations/live-qualification.md); no mock or historical replay counts as live qualification.
PR: https://github.com/arnabnandikgp/cinder/pull/55
Merge: none; ready/open and in progress, live acceptance and review still required.

### Bounded read-only hardware receipt — 2026-10-06

Authority: user-approved `p23-20261006-ro1`, one `c6g.large`, $5/four hours/eight
boots, no financial/native signing activity. The user separately approved bounded
opaque-ingress idle handling, skipped busy Read polls/post-I/O cadence, minimally
encoded AWS certificate sign padding and a fifteen-second application-reply
deadline. Authentication, fixed-root/path/PCR/context verification, freshness,
short handshake/frame bounds, capacity, finite lifetime and no-retry semantics
are retained. Only the owned host and fresh disposable identities were used.

| Check | Actual evidence | Remaining limit |
| --- | --- | --- |
| Non-debug application and six-purpose key release | Actual NSM/recipient KMS; independently reviewed EIF/PCRs and parent denial checks | No production governance sign-off |
| Private HTTP and attestation negatives | Node/Chrome own-only zero account and ten read families; wrong release and replayed quote reject | Not financial activity or a full stream pass |
| Accepted-operation restore | Original image restores accepted grant; refined image restores the same READ grant without resending | Original post-replica-fault restart was not completed before its STS expiry |
| Replica/witness/old-writer faults | Replica loss refuses acceptance with unchanged strong head; witness loss refuses reads; epoch 1→2 preserves head and fences old worker | Not an independent audit or arbitrary-failure guarantee |
| Finite lease | Boot 7 fresh positive reads, then refusal after its natural 90-second lease | Separate from credential expiry |
| Witness STS expiry | Boot 8 positive reads finish at 13:13:28.876 UTC; actual expiry 13:14:04 UTC; refusal at 13:14:19.001 UTC | Longer boot lease/parent credentials and unchanged routes bracket the check |
| Application reply | Before-fix slow regression fails; approved reply fix passes locally and actual grant delivery reaches 12,933 ms | Full live agent/subscription/revocation remains incomplete |
| Native/chain diagnostics | Two native TLS connections; zero actual enclave RPC requests | Neither a complete native schema/read pass nor a devnet genesis request |
| Exposure and resources | Six accepted READ grants, no resend, zero funds/positions/financial exposure; owned host/backing cleanup independently verified | Six disabled KMS keys remain PendingDeletion for the seven-day window; billing is delayed |

Refined default-feature enclave ELF SHA-256:
`54bfa130a2f108443651dd05d383fef7fdb933670211eacac7f33c5c38b3923a`.
READ EIF SHA-256:
`61e8e05d66db51ba331ca9272488f8412fc145b6e9dce75aec6f59f2c03efcc0`.
Boot 7/8 use separately reviewed/pinned lease/expiry manifests over that same ELF;
their exact EIF/PCR/manifest records are retained privately, not substituted for
the READ image. Subsequent SDK serial/reply corrections do not change the enclave.
Rust correction patch SHA-256:
`3093512a3bc77a4232f0001c03752bff50d53b60377f01267eb5e84f21fc3898`;
reply-only patch SHA-256:
`f79232547c3b2334201134655f4cef31b9e509d6f237fed3f3e15f094add38e6`.

Verification: Node 24.21.0/Rust 1.97.1 full `node scripts/check.mjs` passes after
all tracked application corrections, now checkpointed as
`dc8844bda90ab78b672967b96b0efaf282abd711`
(`/private/tmp/cinder-p23-final-offline.log`).
Independent actual Node/Chrome web checks pass 26 groups per runtime; the opaque
web relay has eight passing regressions. The source-only network-disabled ARM64
default-feature correction build and ordinary-host refusal are recorded above;
program/SBF owning sources are unchanged and are not relabeled as a new live
deployment. The public certificate-only regression capture is promoted without
quote context, keys or customer data. Local runtime failures stay recorded.

Final application counters: S3 1,889/988, witness 4,785, KMS 42, native TLS
connections 2, devnet RPC 0. Operator archive/cleanup calls are separate. Before
cloud deletion, 106 ciphertext objects (69,394 bytes) and all four streams' strong
witness heads were saved in protected ignored local storage. Temporary source
exports, image/ELF/patch receipts and reusable test helpers are retained; they are
not shipping wallet loaders or untracked prerequisites for normal CI.

Cleanup: exact owned instance `i-07e2c3fb878e056c0` is terminated; tagged test
EBS, both buckets, witness table, runtime roles, SSH key and security group are
independently verified absent. All six test KMS keys are PendingDeletion with
seven-day deletion scheduled; administrator-owned setup policies are preserved.
The first termination acknowledgement did not yet prove the writer stopped:
cleanup correctly kept backing until a later actual terminated observation.
Only then were the archived replicas deleted. Final cleanup helper returned no
errors, followed by a separate successful verification. Local 18080/19001 test
listeners are absent. No resource beyond the owned run was deleted. Operator
counts at verification: STS 42, IAM 68, DynamoDB 23, S3 234, KMS 186, EC2 26;
these are separate from application counters, not a final billing statement.
All 42 offline contracts/inventory guards, workspace boundaries and whitespace
checks pass again after the evidence update; application source is unchanged.

Remaining blocker: `Replicated::append` reloads accepted history and verifies its
replicas under Runtime's shared lock. Some live views/initial subscriptions refuse
despite corrected reply delivery and poll cadence. Confirm exact refusal stages
with deterministic local cloud delays before selecting a change; preserve one
journal, accepted-head freshness, revocation, restart/repair and unknown-commit
semantics. A larger deadline or skipped durability check is not the next fix.
P23 stays in progress; original full native/chain/financial/recovery qualification
and G01–G04 approvals remain separate. No new hosted checks or merge are claimed.
Publication is authorized for the completed corrections and this evidence only,
via the existing one-layer stack/ready PR #55 against `product/tee-v1`; normal
offline checks pass, but a new head needs its own hosted CI/review. The next local
diagnosis is not part of this checkpoint and is not a durability redesign.

## P24 — Release safety case

Work: not started.
Verification: not run; acceptance in PLAN P24.
Unexpected: none yet.
Next: assemble evidence/independent reviews and bring G01–G04 production policy and assurance decisions to the user before release authorization.
PR: none.
Merge: none.
