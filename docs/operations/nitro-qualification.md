# P20 Nitro qualification runbook

Status: pre-hardware implementation/verification. **Do not provision or launch
from this document until the user resumes hardware testing.** Initial authorized
disposable session: maximum **$5**, IAM profile `cinder_new`, `us-east-1`, no real
wallets, mainnet/customer funds or native trades. No billable resources yet.
The [tracker](../implementation/TRACKER.md) owns actual completion/evidence.

## Release and trust boundary

The image contains a public `boot::Manifest`. Its full canonical digest binds the
actual application policy, domain, storage stream/generation, writer epoch,
ingress/bootstrap/native/cloud routing, loaded CAs, five KMS role contracts and
finite key-use lease. `Configuration::construct` builds the real API, trading and
funding controllers; `Loaded::open` compares their actual contract with the
manifest before serving. No supplied digest replaces that comparison.

PCRs cannot be embedded in their own measured image. `Nsm::discover` reads the
actual locked nonzero PCR0/1/2, then applies the full normal NSM validation. SDK
release policy and KMS key policy independently approve the resulting PCRs. Do
not learn an expected release policy from the relay, live quote or console log.

Five keys/purposes: private configuration, journal storage, trading agent, broker
owner and witness credentials. They use separate KMS key ARNs/context slots and
fresh RSA-2048 recipients; trading/broker/storage secret reuse rejects. Solana
funds authority is **not loaded** in this qualification runtime. Trading, funding
and native-read gates are all false. This does not turn a native unrestricted
broker credential into trading-only authority or authorize future live use.

The parent has temporary **KMS-recipient/S3-ciphertext** credentials, not journal
AEAD keys, user bindings, native seeds or witness credentials. HTTPS and SigV4
terminate inside the enclave. Witness credentials arrive only inside a recipient
KMS capsule. Different credential IDs alone are not proof of correct permissions:
verify policies, grants and actual denial tests before considering the topology
qualified. Cloud administration remains a declared trust boundary; this is not
protection from a malicious administrator of every cloud/KMS/witness account.

## Prepare on the trusted operator machine

1. Choose a new disposable domain/stream, generated independent storage/trading/
   broker seeds, public customer bindings and a precisely labeled qualification
   profile. No inherited CLI wallet, Phoenix account, customer funds or test seed
   from repository fixtures. Native gates remain disabled. Numerical live risk
   policy and source qualification are G01/G02/P23, not selected by this runbook.
2. Provision the bounded tagged test resources only after hardware work resumes:
   five symmetric KMS keys, two distinct private ciphertext buckets, one
   **region-local, non-global** DynamoDB table, temporary narrow runtime/witness
   roles, and the EC2 session. Pin exact returned ARNs/resources; never aliases.
   Test topology is provisional/disposable, not approval of production G03.
3. Construct the CBOR `Prepare` contract consumed by `cinder-prepare-release`:
   `manifest`, `configuration`, `storage`, `trading`, `broker`, `witness`.
   Configuration uses the existing canonical `wire::encode_config`; it is not a
   guessed financial config. Credentials are scoped temporary STS material and
   their expiry is Unix milliseconds. Witness credentials are distinct and must
   not permit or be available to the parent. Bound input to 64 KiB.
4. Pipe it privately into `cinder-prepare-release /absolute/new/private-directory`.
   It makes a new 0700 directory and 0600 files; refuses an existing output path,
   aliased seeds and invalid contracts. Each `<role>.plain` is a purpose-tagged
   KMS plaintext (maximum 4096 bytes), with `<role>.context.json` and the public
   `manifest.cbor`. It recomputes the application commitment and role body hashes
   from consumed objects. It neither contacts AWS nor prints secret material.
   This bounded qualification profile does not implement scalable dynamic private
   onboarding; do not silently put an oversized customer directory in a capsule.
5. On the trusted machine, KMS Encrypt each role file under its exact slot ARN and
   `context.json`. Capture only CiphertextBlob in protected local output; never
   echo Plaintext, STS credentials or seed files. Use `fileb://` for binary input
   and `file://` for the JSON encryption-context map. No role plaintext is copied
   to the parent, source archive, image, console or git. Delete disposable plaintext
   output after successful sealed preparation; retain only approved protected
   recovery material under the actual operator policy, not an ad hoc master file.
6. Build the [default-feature release package](../../tools/nitro-runtime/README.md)
   with **only** the public finalized manifest. Generate EIF using the recorded
   Nitro CLI/kernel/init versions; independently record EIF hash/PCR0/1/2 and
   distribute the SDK Policy. Update each KMS key's narrow recipient policy with
   the approved non-debug PCRs plus exact release/role/generation/stream context.
   Audit all grants/IAM/key-policy paths: one conditional Allow does not defeat
   another unconditional Decrypt grant. Deny missing/wrong recipient conditions
   explicitly. No Encrypt/GenerateDataKey permission is needed by the parent.
7. Prepare `Bootstrap` CBOR: the finite parent KMS/S3 STS credential and exactly
   one `{role, ciphertext}` per role. These are the only bootstrap bytes the parent
   receives. The witness row must already contain the exact full-domain Stream
   key and nonzero Epoch; absent `Sequence`/`Hash` means explicitly provisioned
   empty stream. Never create/reset a missing row during application boot.

Actual KMS CMS/OAEP envelope compatibility remains a hardware test: synthetic CMS
tests establish fresh-recipient replay rejection, not AWS wire qualification.
The recipient has no reusable export method; its private key is dropped after
one response. Require exact KMS response KeyId/SYMMETRIC_DEFAULT and no plaintext
response; validate role/generation/domain/stream prefix and approved body hash.
No KMS error causes cached-key, plaintext, software-attester or debug fallback.

## Storage / witness policies

- S3: exact two buckets and full domain/stream prefix; private access, encryption
  at rest, no runtime Delete/List/ACL/policy permissions. Each immutable digest
  object uses conditional put; an existing object is accepted only after exact
  read-back equality. The journal validates ciphertext head/hash and AEAD. Both
  writes/read-backs precede witness acceptance. Distinct buckets are not proof of
  independent administrators/failure domains; qualify those properties separately.
- DynamoDB: exact table and Stream key, GetItem + UpdateItem only for the enclave
  witness role; no Put/Delete/Scan/CreateTable and no administrative rights.
  Parent role must be explicitly denied the table, including through broader
  attached policies. Strong reads are mandatory. CAS conditions bind exact Epoch
  and prior Sequence/Hash; the writer updates only Sequence/Hash, never Epoch.
  Administrative epoch advancement retains the accepted head and is a separate
  operator action. Never reset to empty to make a failed restore pass.
- KMS: each key has exactly its governed role/context/PCR recipient scope.
  Runtime cannot change policies, mint grants, rotate/delete keys or re-encrypt.
  Release-policy changes require new approval, manifest and measured artifact.
  Provisioning/rotation administrators cannot be the unconstrained parent runtime
  role. The budget test is not production governance approval.

All ports use one-shot enclave TLS 1.3 with pinned consumed CA, exact hostname and
NSM certificate time. SigV4 comes from the pinned official AWS signer without
default credential chains or retry middleware. Strict bounded HTTP/1.1 accepts
Content-Length, not redirects/chunking/compression/duplicate headers. Real AWS
response compatibility must be observed; if it differs, record Unknown/fence and
fix/review the closed profile—never relax verification to make a test green.

## Start and observe

Keep the session resource/tag ledger before launching. Candidate `c6g.large` has
two ARM64 vCPUs/4 GiB and enclave support per prior read-only inspection. Verify
current allocator minimums and AMI/CLI versions before allocating one enclave
vCPU/appropriate RAM. Build artifacts are prepared beforehand; do not compile an
unbounded Rust workspace on the paid instance. No debug-mode accepted image.

Start fixed cloud relays for the manifest ports, with the exact enclave CID.
`cinder-cloud-relay MANIFEST ENCLAVE_CID PORT` maps only to its selected AWS
hostname:443. Keep supervisor stdin pipes open, with no signed requests in logs.
Start `cinder-bootstrap BOOTSTRAP_PORT ENCLAVE_CID` with protected Bootstrap stdin;
it serves one connection, then exits, or refuses after 60 seconds. Launch the EIF
and start `cinder-nitro-relay LISTEN_ADDR ENCLAVE_CID INGRESS_PORT` on loopback.
Use an authenticated tunnel for the independent local SDK; don't expose an open
Internet listener or replace private TLS with a parent HTTP server.

Ready means private API/fresh storage ready, **not live trading enabled**. Enclave
public output is only `cinder runtime ready` or `cinder runtime fenced`. Logs must
never contain user/owner lists, balances, body/preimage bytes, credentials, keys,
or upstream exception text. Operator observations belong in protected receipts;
public evidence uses opaque IDs/redacted totals where needed.

For the AWS console select **us-east-1**: EC2 Instances (tagged session), KMS keys,
S3 buckets and DynamoDB Tables. CloudTrail shows actual control-plane actions;
Billing/Cost Explorer may lag and is not a kill switch. IAM user identity itself
has no geographic region; do not switch the test region to match console sign-in.

## Required bounded hardware receipts

| Test | Required observation |
| --- | --- |
| Real boot / entropy / time | Non-debug locked PCRs; actual NSM quotes, trusted-time monotonicity and latency; kernel/OpenSSL/OsRng behavior; no parent-time fallback |
| SDK / parent privacy | Same signed API auth/grant/view contract through real relay/enclave; altered/replayed quote and wrong SDK policy refuse before private bytes; negative order gating |
| Real key release | Correct per-role recipient succeeds; wrong/debug/missing recipient/context/key fails; old recipient response cannot decrypt into a fresh boot; parent cannot receive plaintext |
| Encrypted restore | Committed private request survives killed/restarted enclave with exact accepted head and no second exposure; one replica loss restores, two losses fail; corrupt/orphan/stale snapshot cannot advance head |
| Freshness / loaded keys | Witness loss/expiry stops serving/signing; retained-head epoch advance fences old loaded boot; concurrent writers cannot both accept a single prior-head CAS |
| Upstream integrity | Changed ciphertext/wrong CA/hostname/time fails, exact fixed routing observed; AWS framing that the closed client rejects is recorded as an unresolved compatibility issue |
| Cleanup / budget | All created resource IDs, lifetime and conservative charge estimate recorded; no EC2/EBS/bucket/table/role/grant left unintentionally active |

No customer/mainnet/venue trades are required for these P20 receipts. Later native
capabilities need G01/G02/G05/P23. An ACK never becomes a fill; possibly exposed
or unknown attempts remain persisted and are never automatically retried on boot.

## Fencing, restart and recovery

Every scheduler cut and private operation rechecks fresh accepted state. The
supervisor is one bounded cut per second with no work queue. Clock/storage/epoch
failure or finite boot lease expiry sets a sticky stop; server closes sessions,
scheduling stops and loaded keys are dropped on process exit. Slow external I/O
is watchdog-bounded; NSM ioctl lateness is rejected **after return**, not proof
against a hung kernel/hypervisor. Observe that latency separately on hardware.

Disabling KMS affects only future release. For loaded boots, advance the
independent writer epoch **while retaining head**, stop/terminate the enclave and
revoke escaped native trading-agent authority before any future native rollout.
Already signed/native-accepted orders require qualified reconciliation/cancellation;
a local fence is not native revocation or terminal-history proof.

After unknown append outcome, do not resend or erase history. Fresh restart reads
the actual witness and opens accepted encrypted history, including consumption,
holds, attempts and uncertain outcomes. Missing witness never initializes a new
journal. A rotated storage key cannot decrypt old generation automatically;
explicit qualified re-encryption/migration is required, not a fresh empty stream.
Treat replica repair/export as ciphertext operations checked against the current
witness; a snapshot alone is not freshness, solvent assets or a recovery claim.

P21 owns coordinated reconciliation/fund return/final claims. No heartbeat
expiry, file restore or this runbook opens the Solana recovery payout path.

## Budget and cleanup stop

Before spend, refresh non-root STS identity, exact region prices, permissions,
AMI/tooling and quotas. Keep estimates for EC2, EBS, KMS lifetime/requests, S3,
DynamoDB and transfer; avoid NAT gateways, load balancers, elastic IPs and other
unneeded fixed charges. Plan a short timed session, not days of compute. Reserve
cleanup time inside $5 and stop early when the remaining conservative allowance
is insufficient. Neither credits nor budget alarms enforce the cap automatically.

On normal exit **and** interruption: terminate enclaves and tagged EC2; verify
termination and DeleteOnTermination for test EBS; remove only exact recorded
disposable objects/buckets/table; revoke/delete test grants/role credentials and
detach/delete only test-created roles/profiles/policies; schedule test KMS keys
for deletion at the service minimum and record pending-deletion status. Remove
test-generated local secret files from their validated private directory. Do not
delete a preexisting AWS resource or broad path by prefix alone. Record anything
that cannot yet be removed; AWS eventual deletion is not an immediate receipt.

## Primary contracts

Checked 2026-10-02: [KMS RecipientInfo](https://docs.aws.amazon.com/kms/latest/APIReference/API_RecipientInfo.html),
[KMS Decrypt](https://docs.aws.amazon.com/kms/latest/APIReference/API_Decrypt.html),
[Nitro KMS policy conditions](https://docs.aws.amazon.com/kms/latest/developerguide/conditions-nitro-enclave.html),
[DynamoDB GetItem strong reads](https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_GetItem.html),
[DynamoDB conditional UpdateItem](https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_UpdateItem.html),
[S3 conditional PutObject](https://docs.aws.amazon.com/AmazonS3/latest/API/API_PutObject.html),
and [official SigV4 1.5.1 interface](https://docs.rs/aws-sigv4/1.5.1/aws_sigv4/http_request/index.html).
These establish API contracts, not completed hardware tests or approved topology.
