# P20 Nitro qualification follow-up — 2026-10-02

Status: **bounded P20 hardware matrix passed**; PR review/merge is pending.
This is not production approval or a native trading workflow. This follows the [first hardware
session](nitro-hardware-2026-10-02.md); its historical receipts are preserved.

## Scope and execution

The user authorized the remaining batch without another permission prompt.
Disposable IAM profile `cinder_new`, `us-east-1`, the existing $5 session ceiling,
fresh independent keys/streams and no existing wallets or mainnet/customer funds.
Native trading, funding and reads remain disabled. Pacifica paper-origin tests
perform TLS handshakes only: no HTTP, credential or order is sent.

The second invocation uses the existing IAM-authorized namespace with consistent
`CinderRun=2` tags and an isolated resource/cleanup ledger. Earlier resources were
verified absent; earlier KMS keys pending deletion were not adopted. New runtime
policies deny Decrypt until the exact measured image is approved, then bind each
role's exact ARN, context, run tags and PCR0/1/2. Policy/grant readback and actual
parent plaintext-decrypt/witness-read denials are required. No operator or runtime
boundary managed-policy expansion was applied.

Host: `i-0743b8eaecdbba93f`, c6g.large, launched `15:21:33 UTC`.
An automatic 120-minute terminate-on-shutdown deadline is installed before
package setup. Enclaves use one CPU/1024 MiB; actual accepted boots report `NONE`,
not debug flags. Nitro CLI 1.5.0 and Docker 25.0.16 run on the disposable Linux
parent. AWS public console pins its SSH host key; the client tunnel is loopback
only. Teardown is recorded below; this page does not claim a finalized bill.

## What the new tests establish

`boot::hardware` is compiled only under `cfg(test)` in a separately measured,
default-feature test executable. It is absent from the shipping application.
Fixed PASS/FAIL and allowlisted phase/error labels are returned through the actual
production AWS-attested TLS server and independently verified SDK. Enclave exit,
parent stdout and a skipped local test are not passing hardware receipts. No AWS
response body, credential, key or private projection is returned as a diagnostic.

The current shipping Linux executable SHA-256 is
`27509b0c92ad3a29cc7a4aed62ed8da69c6e7f599c270856f598182bbcfec479`;
its final application regression passed. Public source export
`5557d3350f371d616965006a90ea03b55420400f` built the default binaries with locked,
offline Rust 1.97.1 in a network-disabled Apple container. Only explicit public
binaries, matching runtime libraries and the public manifest were uploaded.
Private preparation directories and role plaintexts never enter the image or
parent. Later test-only revisions and their results are separate provenance.

| Case | Observed result / remaining work |
| --- | --- |
| Actual NSM boot, non-debug PCRs, attested verdict | Passed on the second host. |
| Five-role KMS recipient/context matrix | Passed after correcting the test's wrong-role error expectation. Positives bracket the negatives; no response with released material counts as a denial. |
| Paper-origin TLS CA/hostname/encrypted-record negatives | Passed before entering the journal test. Real SNI/route is retained for hostname verification; the distinct Nitro root is the wrong-CA input. No HTTP is sent. |
| Real encrypted orphan, no CAS acceptance | Passed: both real replicas contain an authenticated proposal; reload excludes it and retains genesis. |
| Real CAS accepted, acknowledgement suppressed | Passed in the final security batch: reload includes the accepted transaction exactly once; duplicate and stale-snapshot handling, current-snapshot restore and reopen succeed. Earlier failed streams remain preserved. |
| Wrong measured image, unchanged contexts/capsules | Passed for all five roles, with verified positive runs of the same approved image before and after. SDK-only selection of the negative probe left KMS/parent approvals unchanged. |
| Competing independent CAS writers | Passed with an independently verified attested receipt: one real CAS succeeds and one receives ConditionalCheckFailed; the accepted winner survives the stale follow-up. The register is separate and never used as an application head. |
| Loaded boot-lease | Passed: initial actual SDK view, then natural 90-second lease expiry with API refusal/enclave exit; parent ingress still listens, accepted head unchanged and all five KMS keys Enabled. No KMS/policy revocation manufactured the fence. |
| Actual STS expiry | Passed: actual 900-second witness credential expired at 16:56:38 UTC with a 40-minute boot lease. Independently verified SDK view shortly before expiry; API refusal and unchanged epoch-one/sequence-two accepted head afterward at 16:56:58 UTC, with all five KMS keys Enabled. Same-expired-capsule restart also refuses; only parent relay STS was refreshed. At 17:03:04 UTC the enclave inventory is empty while parent ingress still listens; 17:03:17 UTC readback confirms the same head and enabled keys. No manufactured expiry, witness renewal or policy/key change. |
| Final shipping application/SDK regression | Passed on the current shipping ELF: actual owner view, valid grant/revoke, disabled-native-order refusal, wrong policy/altered quote/replayed quote rejection, restart with epoch two and retained head, and stale epoch-one refusal. Failed operator probes remain separately recorded. |
| Debug-mode application refusal | Passed with the exact application EIF and unchanged KMS approval: no private API, enclave exits, ingress stays listening, accepted head unchanged, all five KMS keys remain enabled. A subsequent non-debug boot succeeds. This is refusal before release/API, not a claim that the debug application sent an actual KMS request. |
| Exact AWS and local-secret cleanup | Verified: instance terminated, test EBS/buckets/table/roles/SSH key/security group absent; five run-two KMS keys PendingDeletion, preexisting managed policies preserved. Removed 45 validated generated local secret files; retained operator source/public/ciphertext evidence. Only the P20 build container was stopped/deleted. |

## Failures and corrective work

1. The public binary copy lost its executable bit. The first image exited without
   an attested verdict. Explicit `COPY --chmod=0755` fixes image entrypoints;
   executing the image's `--list` with no network validates packaging before boot.
   This was not a key-release or application logic fix.
2. With exact role-specific KMS/IAM context policies, wrong-role ciphertext returns
   authenticated `AccessDeniedException`, before the `IncorrectKeyException`
   anticipated by the test. Actual attested failure identified
   `recipient/configuration/wrong-role/unexpected-access-denied`. The corrected
   expectation still requires HTTP 400/403, the exact error class and absence of
   both Plaintext and CiphertextForRecipient; outages/expiry/5xx do not pass.
   [KMS Decrypt reference](https://docs.aws.amazon.com/kms/latest/APIReference/API_Decrypt.html)
   describes the incorrect-key error, not unconditional precedence over denial.
3. At source export `45b9c33982df8a85c6d567681508fab6f514ae1a`, test ELF
   `de605957ca218af399c365e70888a633a8e5351e03f3bf5e95de73c0884c2b36`
   returned an actual attested `security-batch/journal/FAIL`. Recipient/TLS checks
   completed; the accepted register reached sequence one. Fixed journal phase
   labels isolated `journal/current-restore/FAIL` on a separate fresh stream.
   Both failed accepted histories remain preserved, not reset.
4. Current-snapshot restore unconditionally rewrote already-valid immutable S3
   objects. A local create-only-replica regression reproduced the failure before
   the fix. Restore now validates/reuses both valid copies and only repairs a
   missing/invalid copy, subject to the store's immutable-write policy (repair
   can still refuse); snapshot verification and the independent accepted head
   remain mandatory. No witness reset, CAS advancement or weakened authorization
   is introduced. The regression and macOS/Linux checks pass after the fix.
5. The shipping-application grant probe used arbitrary random bytes as its
   Ed25519 public key. That is not a guaranteed valid verification key. The
   ignored operator probe now uses a real generated keypair, with fixed response
   categories only. The valid grant returns Complete; owner-only operation
   reconciliation confirms the saved successful grant. The first failed probe
   is retained, not recast as a pass. No application authorization check was
   weakened. A separate transient disabled-order exchange failure is also
   retained; a later verified response returns the required Unavailable, with
   no native relay/capability enabled.

At the final export above, separate test ELF SHA-256
`40c1642bce420e70f1e624407b5a3ec5c671576f825cfae3088f38525b772acb`
returned an independently verified actual AWS-attested
`CINDER-P20-RESULT-1/security-batch/PASS`. Its non-debug boot was observed at
16:00:30 UTC. The complete recipient, upstream TLS and real encrypted-journal
batch passed, not merely its process exit. No native financial action ran.

The witness-ack fault is injected **inside the test enclave** around the real
AWS CAS. This is not a physical packet-drop experiment. It still exercises real
encrypted persistence, actual acceptance and journal reconciliation; its precise
limits must accompany the final result.

The runtime's credential safety window can fence shortly before AWS expiry;
the negative observation above is after the actual issued expiry. This tests
loaded-runtime expiry and restart refusal, not an HTTP `ExpiredToken` response.
The boot lease was longer, parent relay remained available, accepted history was
retained and KMS keys/policies were not disabled to manufacture the result.

## Public artifact register

These are separate measured images, not interchangeable approvals. Only public
source/binaries/libraries/manifests enter their contexts. The application, lease
and expiry images use the same shipping ELF; their consumed manifests differ.
The unapproved image is accepted only by the SDK verifier for observing its
negative verdict, never by KMS.

| Image | EIF SHA-256 |
| --- | --- |
| Final security batch | `80d21c40ab98aae8b706995108bec1531a3bfe441eb3afdaeeef7be8a3a06099` |
| Approved recipient matrix | `b808c0372658c8dd6ebb8d3d3347910f1e4f73733545d06e7102da5195effe21` |
| Unapproved measurement probe | `5b107fe9023f4dcf84cc5555305c064a174c6a243541d40bc9d8477120ab3800` |
| Separate-register race | `30fc67c1046df4263192de8419520aa045507099856eb7b32a191e9fe4404da0` |
| Final application | `e26c8bee820c27609dc9d119217088087261a27efb16582758aa4a67d06a8b64` |
| 90-second lease | `5a7be5fc95857c097757724d7cccfa33ed4fcf718affbe297f97b71201053a2b` |
| Actual STS expiry | `4e200124bb27758ca9e67ada7be56ffa7fe955646e41b93dcea2fab679abfd51` |

Final application's non-debug measurements, independently pinned in both KMS
and the SDK before collecting its receipts:

```text
PCR0 45bc3070d13ef66f4a592c0fc86e74e5451f4e94c086ddaca7af902118f23bbae4a0b5d5196e17220946d25a3140bc86
PCR1 3b4a7e1b5f13c5a1000b3ed32ef8995ee13e9876329f9bc72650b918329ef9cf4e2e4d1e1e37375dab0ba56ba0974d03
PCR2 3e3504e6930f7dd5a94e1425b946bc233e3b226ab15f0c2bbb6175308b2dd33152d642039c2e65ce745390f724d857e7
```

Actual verification happens at collection through fresh AWS quotes and the
pinned TLS channel. Fixed logged verdicts are observed test evidence, not
standalone signed solvency proofs or independently replayable offline receipts.

## Offline evidence and next step

For the corrected recipient test: macOS all-feature release checks pass 32 service
unit tests plus eight integration tests, with seven Nitro-only cases ignored.
Linux default-feature release checks pass 25 unit tests with seven ignored;
strict all-target/all-feature Clippy passes on both platforms. Formatting and
dependency guards pass. Node 24.21.0 planning validation passes 25 phases/33
documents and all 12 validator tests. These are not full workspace/SBF reruns or
proofs of hardware acceptance. The subsequent diagnostic test source export is
`e78a1060f28cb1627309be029339ba96cddb99c1`; its Linux 25-test check and macOS
strict Clippy pass. These are historical checkpoints; the current-export results
below and the actual hardware table above supersede their earlier pending status.

Current export verification: the full macOS locked/offline release workspace
passes. Journal/service release suites and strict all-target/all-feature Clippy
pass on macOS and network-disabled Linux ARM64. The new create-only regression
passes, and Linux default binaries were rebuilt. All 17 SDK tests and TypeScript
checks pass with pinned Node 24.21.0 and current release binaries. An earlier SDK
invocation failed because its expected debug binaries had been removed; a
temporary target-directory alias to the checked release binaries fixes the test
setup without changing tracked tooling. The corrected plan check passes 25
phases/34 documents and all 12 validator tests. No on-chain source changed; SBF
was not rerun.

## Test tooling retention

| Location | Purpose / retention |
| --- | --- |
| `crates/service/src/hardware.rs` | Versioned reusable Nitro tests, `cfg(test)` only; seven explicit hardware cases ignored in normal CI and absent from the shipping application. Requires fresh independently approved public manifest/image and disposable cloud scope. |
| `crates/journal/tests/encrypted.rs` | Versioned ordinary offline regression for current-snapshot restore against create-only replicas; no AWS/wallet dependency. |
| `crates/service/examples/qualification-profile.rs`, service helper binaries, `tools/nitro-runtime/` | Versioned typed preparation, measured application/relay/bootstrap tools and reproducible public-only packaging. Not an account-specific deployment automation service. |
| Ignored `work/experiments/p20-hardware/` | Retained local session operator scripts, case recipes, public measurements/verdicts, ciphertext and exact resource ledger. These contain fixed disposable account/profile/namespace assumptions; they are not a production launcher or a fresh-checkout prerequisite. Closed-session guards prevent reuse of cleaned resources. |
| Task-specific `/private/tmp/cinder-p20-*` exports and Apple container | Temporary pinned Node tooling, read-only source/vendor exports and Linux artifacts. Not the only source copy of tests. No work/wallet/AWS directories mounted in the Linux build. |

Final teardown requested instance termination at **17:04:07 UTC**, after
approximately 1 hour 43 minutes from launch. Independent readback verifies
termination, absent test EBS/buckets/table/runtime roles/SSH key/security group
and all five fresh KMS keys PendingDeletion (seven-day minimum). Exact EBS ID
readback also returns `InvalidVolume.NotFound`. User-managed operator/boundary
policies remain intact. Conservative estimated invocation charges are below
**$0.50**, not a finalized bill; billing can lag and credits are not a kill switch.

The local cleanup first validates the verified-cloud receipt and every exact
file's parent/type/link count. It removed 45 generated files: five role bodies
and one disposable owner fixture in each of seven prepared profiles, plus the
root owner fixture, parent-STS bootstrap and SSH private key. Those disposable
secrets are intentionally deleted, not recoverable test authority. Retained
operator source and public/ciphertext evidence are sufficient provenance, not
credentials for another session. The task's network-disabled Apple build
container was stopped/deleted; exported public artifacts remain in task-specific
temporary storage. No broad pruning was performed.

The user's research, wallets, AWS profile and unrelated untracked work are
untouched. Do not commit session
credentials or blindly promote the hard-coded operator scripts. A later AWS
reproduction needs new permission/resource identities/credentials and independent
measurement approval, not replay of this session's retired authority.

Next: publish P20 above P19 for review, and use
the versioned tests/runbook for later independently authorized reproductions.
Exact measurements and evidence limits above do not imply production approval.
P20 stays in progress until review/merge; no P19 merge is implied.
