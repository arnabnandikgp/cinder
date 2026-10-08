# P23 next hardware boundary

Prepared 2026-10-07. The original local authorization stopped before AWS.
The user subsequently approved this focused read-only hardware run ("let's go
for it then"). **Approved read-only scope, not financial qualification.** The
fresh IAM/resource/image gates below were checked for the recorded invocation;
this approval does not authorize native/RPC/financial activity or another run.

## Why this run

Close the remaining actual-cloud history-growth gate with the manifest-bound
shipping pack backend. The local fix preserves full history and original IDs;
it reduces object calls, not historical bytes or economic obligations.
[ADR 0026](../architecture/0026-retained-history-packs.md) and TRACKER identify
the source, resource bounds and offline evidence. This is one focused next run,
not a promise that read-only success closes all of P23.

## Approved run bounds

| Boundary | Approved limit |
| --- | --- |
| IAM/region | Reverify intended non-root `cinder_new` / `us-east-1`; fresh login and exact scoped operator/boundary documents required |
| Compute | One Nitro-capable `c6g.large`, at most $5/four hours from launch, eight non-debug boots, no replacement host; fresh pricing/quota/AMI verification before provisioning |
| Cloud resources | Fresh disjoint run namespace, two private ciphertext buckets, independent witness table, two scoped roles, six fresh purpose-separated KMS keys, encrypted root volume, operator-IP SSH; no NAT/LB |
| Image | Exact committed default-feature ARM package, newly finalized public version-3 manifest, independently checked EIF hash/PCRs before recipient key release |
| History | Explicit 64-KiB record / 8-MiB original history / 256-record ceilings; each replica at most 512 PUT attempts / 128 MiB per boot; no fallback, pruning, witness reset or automatic format migration |
| Whole-run volume | At most 512 KMS / 10,000 S3 / 10,000 DynamoDB calls; at most 128 MiB / 1,024 objects per bucket and 1 GiB outbound; include setup/fault/cleanup, reserve teardown capacity |
| Financial/native scope | Zero native/RPC requests or submissions; native reads, funding, trading and API risk admission remain off. Disposable non-economic READ credentials only |
| Stop conditions | Hard deadline/cost/resource limit, uncertain resource creation, inconsistent accepted head, failed authority or cleanup. Diagnose from retained original evidence; do not append cases to a failed invocation |

These bounds were approved for this run, not inferred from prior billing or credits. Old C5
policies, credentials, source measurements and terminated resources are closed
evidence, not authority for a new namespace. Prepare and inspect fresh exact
policies and an owned-resource ledger before launch. Nothing may be adopted from
a matching name alone. This document deliberately does not invent fresh IDs/PCRs.

## Recorded preflight — 2026-10-07

Read-only checks confirm the intended non-root `cinder_new` identity, Nitro
support for `c6g.large`, an 8-vCPU standard-instance quota, no active EC2 hosts,
and the available Amazon-owned ARM AMI `ami-065b1b834d2a83a7a` (owner
`137112412989`). AWS Pricing reports $0.068/hour for shared Linux `c6g.large`
in us-east-1; four hours is $0.272 compute, not the whole-run bill. Recheck
metadata, remaining budget and cleanup access at launch; no spot/capacity
availability is asserted. All eight PR59 jobs and CodeRabbit are green at
`800346b09510f1d7a7ea30c2b335416ffe3df4f9`, with no review threads when checked.
The final package's thirteen hashes and enclave/rootfs equality reverify.

Fresh namespace: `p23-20261007-pack1`. The administrator used local ignored
setup files in `work/experiments/p23-live/packed-permissions/` to create
`CinderP23PacksRuntimeBoundary` without attaching it to the user, and create
and attach `CinderP23PacksOperator` to `cinder_new`. They preserved the prior
reviewed action scope with new exact resources; key creation additionally
requires the new session request tags. The agent does not administer its own
grants or overwrite old policies. Existing user EC2/S3 privileges are not
narrowed by this operator policy; runtime roles still need their exact boundary,
stream and recipient/PCR/context policies.

After administrator setup, independent readbacks verified both original exact
policy documents, the correct operator attachment, no user-boundary attachment
and fresh resource names. Initial pre-provisioning created two empty tagged
roles, then AWS rejected the first tagged `CreateKey` with `AccessDeniedException`.
The supplied template omitted IAM `kms:TagResource`, which AWS requires in
addition to `kms:CreateKey` when tags are supplied. The local operator template
now adds account/region/phase/session/tag-name restrictions; the boundary is
unchanged. The tagging grant is not intrinsically creation-only: otherwise
authorized untagged keys can match, so the owned-resource ledger and exact key
policies remain mandatory. Do not adopt or retag existing keys.

Both empty roles were deleted after exact live-tag checks. Independent cleanup
confirmed absent roles, buckets, witness table and test volumes; the ledger has
zero instances and zero KMS keys. The failed attempt and cleanup remain in
ignored `work/experiments/p23-live/run-pack1/resources.json`, not a reusable
provisioning ledger. **No EC2 launch, EIF, runtime credential, key release,
native/RPC call or financial activity occurred.** At that checkpoint, the next
step required the administrator to edit only the existing `CinderP23PacksOperator`
default JSON; its attachment was already correct. The correction and independent
fresh checks are recorded below, preserving the rejected attempt's evidence.

Local operator preparation separately passes the unmodified trusted preparer:
version 3, six purpose-separated roles, committed history bounds and financial
gates off; a 257-record policy is rejected. These are offline configuration
checks, not measured hardware or completed cloud-fault tests.

### Closed retry receipt

The administrator saved the corrected operator default JSON; independent exact
policy, attachment and absent-resource-name checks pass. New attempt `run-pack1b`
has tag `CinderRun=pack1b`, a new ledger and fresh keys/credentials/streams. It
preserves the rejected, closed `run-pack1` unchanged. Its resource names were
independently absent, with no earlier keys/head/ciphertext/host to reuse.
Launch metadata was rechecked; one encrypted, auto-terminating Nitro
host was used. Exact uploaded hashes, new version-3 public manifest, independent
EIF/PCR/CRC/entrypoint checks, six-purpose recipient policy readbacks and actual
parent witness/plaintext-decryption denials pass. First boot is non-debug.
Actual Node/Chrome cuts 2/8/32/64 pass own HTTP/WS reads with one/two watches and
zero credit/positions. Lost-reply original-ID/digest reconciliation, exact process
restart and the authority epoch fence pass without resend or head reset. Fresh-
stream loss/corruption/failed repair, fault replay, witness loss, quota and natural
boot lease and natural witness-credential expiry pass. All application workers
are stopped; 188 ciphertext objects and five strong final heads are archived and
independently checked. Owned teardown passed independent verification at
10:30:46 UTC: instance terminated, test EBS/stores/witness/roles/SSH/security group
absent, six keys PendingDeletion, prior managed policies preserved. Seven boots
used one host for about 69.5 minutes, within all recorded request/resource bounds.
This invocation is closed; its ledger, identities and credentials are not reusable.
[The sanitized hardware receipt](P23-PACKS-HARDWARE.md) preserves the exact result boundary.
Local `run-pack1b/RUN.md` and exact receipts preserve this invocation;
TRACKER remains the tracked status. No financial acceptance is inferred.

## Fixed acceptance list

1. Recheck package/rootfs, version-3 actual policy commitment, six key purposes,
   non-debug attestation/recipient release and parent/witness-denial boundaries.
2. Actual Node/Chrome HTTP/WS: grow using real READ-grant commits, inspect heads
   2/8/32/64 with concurrent own-account reads and one/two watches. Retain cloud
   calls/bytes and latency. Use unchanged application deadlines/freshness; an
   unavailable result stays unavailable, not a timeout increase or omitted cut.
3. Lost reply and fresh process restart: original-ID/digest lookup, identical
   accepted history/receipts and no resend, reset or synthetic padding.
4. Actual old-pack copy loss/corruption/repair and budget exhaustion: both copies
   read back before CAS; failed repair/charge cannot accept a new head. Verify
   witness/epoch/finite-authority refusals and no stale private publication. Use
   fresh streams for destructive storage fault probes; never corrupt an unrelated
   or earlier evidence archive.
5. Preserve bounded ciphertext plus strong heads and sanitized exact receipts;
   verify zero financial exposure. Stop writer before owned teardown; verify host,
   volume, buckets, table, roles, SG and SSH cleanup, and the six test keys disabled
   and scheduled for deletion. Preserve old PendingDeletion keys/setup policies.

## Financial continuation is separate

[The native evidence contract](P23-NATIVE-EVIDENCE.md) identifies what the current
API supplies and what remains unqualified. After storage acceptance, prepare
authenticated source capture and the original-operation funding round trip.
Native WSS transport, bootstrap exception if required, final credit/withdrawal
semantics, lost-ACK UUID/batch correlation, complete execution/funding cuts and
actual recovery must receive their own evidence and bounded financial authority.
Do not flip forbidden manifest booleans, construct completion structs from
parser output, reuse historical funds/wallets, or call this read-only run a
deposit/trading/recovery demo.

The fixed P23 PLAN criteria are unchanged. Report each actual lifecycle result
or named blocking capability; no new arbitrary phases or frontend gate is added.
