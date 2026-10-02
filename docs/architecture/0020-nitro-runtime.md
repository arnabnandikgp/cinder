# ADR 0020: Nitro application assembly and qualification

Date: 2026-10-02. Status: pre-hardware assembly implemented; qualification pending.
Scope: [P20](../implementation/PLAN.md#p20). [ADR 0019](0019-attested-service.md)
and [BASELINE](../implementation/BASELINE.md) remain the security/economic contract.

## Implemented first slice: actual loaded application policy

`ApplicationContract::derive` hashes the canonical validated journal configuration,
the actual API owner/session contract, the loaded execution profile/policy plus
agent key and epoch, and the loaded funding profile/route. It rejects mismatched
journal configurations, conflicting API/custody beneficiaries, and a trading key
reused as the broker or Solana funds authority. No secret key is hashed/exported.
API initialization uses the same existing persisted fingerprint, so this does not
change P18 history or create a second ledger. These are private-runtime Rust
interfaces, not customer endpoints or a public owner directory.

This **application-policy component is not the complete release manifest**. The
assembled runtime must also bind its actual storage stream/generation, witness
identity/epoch, KMS recipient/context policy, authenticated egress/clock ports and
capability gates before constructing the attested server. Merely putting this
digest into a caller-provided attestation policy does not qualify those ports.

## Implemented first slice: actual NSM boundary

Pin AWS's `aws-nitro-enclaves-nsm-api =0.5.2`, with its official driver. Public
construction opens Linux `/dev/nsm`; non-Linux/missing hardware fails. Require
major-version-1 SHA384, bounded module/PCR metadata, locked PCR0/1/2 exactly equal
to the nonzero approved values, and a successful bounded NSM `GetRandom` request
before use. Every entropy call is one request with no retry or alternate source;
temporary entropy buffers zeroize. Reject undersized/oversized/all-zero output.
This health check does not statistically prove entropy quality.

Attestation requests bind the live challenge, SPKI and P19 user-data digest. The
adapter rejects changed policy, malformed/expired context and malformed SPKI. It
checks the returned quote through the same strict pinned-AWS-root verifier used
by the SDK before returning it. There is no injected public device provider, file
quote loader, fixture root, debug bypass or parent-approved boolean. Unit-test
exchanges are module-private and are not available to a service build.

The driver brings pinned `nix 0.31.3`/`cfg_aliases 0.2.2`, the existing libc graph
with `extra_traits`, and nix's platform build script. The exact lock digest,
resolved features and build-script allowance are updated explicitly; arbitrary
driver features or new target/provider dependencies still fail the guard. Safe
workspace code delegates the hardware ioctl to AWS's pinned upstream driver;
source inspection is not an independent security audit.

## Historical first-slice gaps

The entries below describe the first checkpoint, not the current remaining-work
list. The consumed-manifest, key-release, remote-storage and executable assembly
are implemented in the continuation below. Hardware evidence remains separate.

- TLS/library RNG integration; NSM entropy alone does not
  qualify OpenSSL or journal nonce generation. NSM ioctl timing/deadline behavior
  also needs Linux/hardware observation.
- Complete consumed-configuration manifest and measured financial executable;
  qualify the implemented vsock ingress/relays and enclave-side HTTPS on hardware.
  Native reads, chain, storage/witness and KMS egress remain unwired.
- Recipient-bound KMS release with wrong/debug/replayed release rejection and
  key-role separation. No plaintext key delivered to the parent.
- Encrypted external replicas and separately authenticated fresh witness; test
  lost witness, rollback, competing writer and crash/restore. Local witness files
  and shared parent credentials are not independent freshness evidence.
- Bounded controller scheduling and reconciliation, with no automatic retry of
  escaped/unknown native actions. The P19 fake dispatch/flat holds are not promoted.
- Actual release/fencing rehearsal, secret-safe health/runbook, SDK operation
  through the tagged enclave, and cleanup/cost receipts.

No production topology, witness independence or live financial limits were
approved by that checkpoint. G01–G04 stay open as applicable. That slice could not run
the financial application on AWS yet and does not satisfy any complete P20
hardware acceptance criterion. It is superseded by the assembly below, not by a
hardware qualification claim.

## Local NSM time integration (2026-10-02)

Boot and every `Clock::now` require a new NSM-random challenge and a purpose-bound
attestation. The timestamp is returned only after strict CBOR, exact PCR/policy/
nonce binding, pinned-root X.509 validation and the COSE signature pass. The local
clock checks certificate validity at that signed timestamp to bootstrap without
parent wall time. This path is crate-private and cannot replace the public SDK's
independently timed freshness verification. Clock quotes and TLS-session quotes
have separate key/data purpose bindings and cannot be substituted for one another.

The boot's last verified time may remain equal within a millisecond but cannot
move backward. Failed clock/entropy/quote verification permanently fences that
NSM handle; no cached-time, parent-time or retry fallback. Operations reject
results taking more than five seconds **after the ioctl returns**. This is not
kernel-ioctl cancellation or proof against hypervisor pauses; driver latency and
real timestamp semantics still require hardware observation. Fresh time is not
a durable accepted-head witness or a replacement for authoritative financial
event/finality evidence.

Local verification now handles the actual ordering of NSM requests: a TLS quote
can be generated after the initial clock sample. The initial implementation would
incorrectly reject such a quote as future-dated. Its freshly signed timestamp is
verified locally and checked against the prior cut and monotone clock; the SDK's
strict client-time rules are unchanged. TLS certificate validity now uses the
selected clock explicitly instead of OpenSSL's host-wall-clock helper. OpenSSL
key/boot entropy remains pending measured-image qualification.

Six additional offline regression groups cover request binding, sticky failures,
operation timing limits, synthetic signed timestamp/chain/nonce/purpose tampering,
later-quote ordering and certificate clock selection. These are synthetic/local
tests, not real Nitro evidence. No complete P20 acceptance criterion is closed.

## Socket dependency decision (2026-10-02)

Add exact `socket2 =0.6.5` with only its explicit `all` feature to obtain safe
owned AF_VSOCK sockets, timeout/connect/accept and peer-address APIs. Workspace
unsafe remains forbidden. The already pinned NSM driver's nix ioctl wrapper is
unchanged; using nix's raw-fd `accept` would require introducing local unsafe
ownership conversion, which this dependency avoids. The reviewed socket boundary
has no alternate provider, environment endpoint override, raw-fd injection or TCP
fallback. Platform-specific implementation is Linux-only; non-Linux refuses.
The exact graph and five negative dependency-guard mutations are recorded. It
adds `windows-sys 0.61.2` on its Windows dependency edge, not another Linux provider.
The socket-checkpoint Cargo.lock SHA-256 was
`43d0fc8000bdfd4ccdf892a0d613aeca992cad25a5bd51f06396c795e9ef842e`.

## Implemented socket and venue HTTPS boundary (2026-10-02)

`Server::run_vsock` uses the existing attestation, TLS 1.3, exporter-bound session
and private handler contract directly on an owned AF_VSOCK stream. Ingress allows
one explicitly selected peer CID, with eight workers and the existing handshake/
session watchdogs. A CID is routing, not authentication. Connections reject
unconfigured peers before allocating a worker; no raw-fd ownership conversion or
TCP fallback is exposed. Accepted sockets use explicit five-second I/O limits.

The `cinder-nitro-relay` parent executable accepts loopback TCP only and forwards
opaque bytes to one exact enclave CID/port. `cinder-nitro-egress` accepts one
enclave peer and forwards to one selected Pacifica origin on port 443. It resolves
that origin once at startup and attempts one TCP connection per accepted socket:
no caller-selected URL, address rotation, TLS termination or retry. Both use
bounded workers, 16 KiB copy buffers, 4 MiB per direction and the session watchdog.
These qualification entrypoints require stdin to remain open; stdin closure
fences/stops them. They are not a completed deployment supervisor/runbook.

`Egress` consumes the non-clone signed `Outbound` only over parent CID 3 vsock.
HTTPS terminates inside the enclave. It verifies the exact native hostname and
chain against a bounded canonical DER CA certificate with an independently
expected digest; this store replaces OpenSSL's system trust, rather than adding
to it. Certificate time comes from the selected clock, not parent wall time.
TLS 1.3 is required; hostname verification, SNI and peer verification remain on,
with no permissive callback, resumption, tickets or early data. Its commitment
binds the actually consumed native origin, vsock endpoint and CA digest. Clock,
capability and other port commitments are still needed in the full manifest.

The intentionally narrow HTTP/1.1 profile permits only the existing create,
scoped-cancel and withdrawal paths and preserves the signed body exactly. Bound
request/response bodies to the adapter's 16 KiB limit, headers to 8 KiB/64 fields,
and a one-shot socket to the five-second watchdog; reject results beyond the
ten-second overall elapsed/clock budget. Require one nonzero Content-Length and
JSON content type. Redirects, duplicate headers, transfer encoding, compression,
truncation and malformed framing fail as `Unknown`; there is no automatic retry.
Numeric Retry-After is bounded to one hour; other forms leave the adapter's
conservative default in effect. No connection is reused or second response read.
This is not a general HTTP client: chunked/missing-length or alternative TLS
venue behavior still requires qualification and an explicit profile change.
The gateway retains durable exposure/holds on uncertain results; an ACK is not
a fill. No native capability is activated by constructing this transport.

Offline tests cover fixed routes/ports, bounded opaque copies, exact request
bytes, response ambiguity/bounds and real loopback TLS handshakes with a synthetic
CA. Wrong CA, wrong hostname and future/expired certificate time all reject.
Synthetic certificates never enter a default runtime policy. Linux additionally
checks safe owned-socket cloning/timeouts/read/write/shutdown using a Unix pair;
that is not an AF_VSOCK device test. Hardware vsock routing, real NSM/clock/RNG and
actual venue response compatibility remain unqualified.

## Local Linux qualification (2026-10-02)

Exact exported source tree: `f9221a4df607d362a17f9d3ee37799371d6d4ae0`.
The complete Rust debug/release suite, format and default/all-feature strict
Clippy/build passed on Linux ARM64; ordinary Linux refuses `/dev/nsm` construction.
The macOS pinned full runner passed too, including 27 repository guard tests and
all 17 SDK tests (seven actual local-process tests). SDK/Node process tests were
not rerun in Linux; Anchor/SBF source is unchanged.

Apple Containers 1.4.1, recommended Kata 3.32.0 kernel, Rust 1.97.1, Debian
Bookworm OpenSSL 3.0.22 (`3.0.22-1~deb12u1`) and pkg-config 1.8.1. Check-image index:
`sha256:c3867809f5ba272041b6c5fed0abed01476cc186e4e8ba2367e64ac1e037f546`;
ARM64 manifest:
`sha256:7b3f6c004538ca245185c7e6aa138aeec51b077448e06a53550d3ba40a3a1caf`.
Tests used four CPUs/4 GiB, networking disabled (only loopback interface),
read-only exported source/public vendor mounts and container-local outputs.
Incremental compilation and debug symbols were disabled for disk capacity;
debug overflow/assertion semantics remain enabled. No `work/`, `.git`, wallets,
cloud files or the local-only user journey were mounted. The container is removed
on exit. This ephemeral toolchain image is **not** a production EIF, measured
release, immutable Debian repository snapshot or attestation qualification.

Setup initially failed on missing kernel, DNS, missing lint components and disk
space. Installed the recommended kernel, used explicit DNS only during public
image setup, and removed the task's builder/cache after export. Removed only
regenerable Cinder Rust incremental cache; source/research are untouched. These
setup failures are not failed financial tests or excuses to relax qualification.
No complete P20 acceptance criterion is closed by this offline checkpoint.

## Authority and cost boundary

The user approved **at most $5 for the initial disposable AWS test session**,
the replacement IAM profile `cinder_new` in `us-east-1`, no mainnet/customer funds. This is not permission to
leave instances running or expand to a larger test budget. Check non-root identity,
current instance/storage pricing, narrowly scoped permissions and a cleanup plan
before provisioning; keep an elapsed-time/resource ledger and stop before the
conservative estimated allowance is exhausted. AWS budget notifications are not
a hard spending cap. On 2026-10-02 a fresh STS call verified the IAM user
`cinder_new`, and read-only EC2 inspection confirmed `c6g.large` supports enclaves
with two vCPUs and 4096 MiB. The prior root-identity blocker is resolved. No
billable test resources have been created. The user now requests completion of
all pre-hardware work and a stop **before hardware testing**. Do not provision or
launch from this branch until that work is resumed. Refresh pricing/permissions
and record the exact disposable resources before spend; the earlier $5 limit is
not permission for a larger or unattended deployment.

## Consumed release and enclave executable

`boot::Manifest` is a canonical public policy embedded in the measured rootfs.
Its digest binds the actual application component, full network/deployment domain,
storage stream and generation, writer epoch, all ingress/bootstrap/native/cloud
routes and consumed CA digests, five distinct KMS key roles/body hashes and finite
boot lifetime. `cinder-prepare-release` derives the application component by
constructing the actual API/gateway/funding objects from confidential configuration,
not by echoing an operator-supplied label. Conflicting owners or key roles reject.

PCRs cannot be embedded in their own image. `Nsm::discover` reads the actual locked
nonzero PCR0/1/2 and then applies the ordinary device/profile/entropy/time checks.
KMS policy and the independently delivered SDK Policy approve those actual PCRs;
discovering them inside a changed image does not make its keys or client accepted.

`cinder-enclave` loads only the bounded public manifest from its image, obtains
the one-shot bootstrap over exact parent-CID vsock and releases each KMS role
inside the enclave. It constructs the actual API and one encrypted replicated
journal before opening the attested listener. There is no parent HTTP framework,
fixture attester, flat fixture holds, seeded customer funds, fake exposure or
second ledger. Native trading, funding and reads are explicitly disabled and
the manifest rejects activation in this qualification build. G01/G02/P23 must
qualify the real observation/chain ports and policy before enabling them; this is
not a silently incomplete live-trading promise.

The reservation helper derives gross bounded notional with upward rounding,
selected customer leverage, native initial margin and maximum fees from installed
policy. It does not replace the journal's joined pending/stress admission. Tests
cover asymmetric customer/native margin, signed quantities, missing qualification
and negative fees. The helper is not an excuse to activate native capability.

The one-cut supervisor and every private request re-read authenticated accepted
state and trusted time. Clock/storage/writer-epoch failure or finite lease expiry
sets a sticky fence; sessions and scheduling stop and keys drop on process exit.
Unknown/possibly exposed native actions are never automatically re-signed or
resent after restore. No queued operation acquires a second financial ledger.
NSM ioctl cannot be cancelled by this safe wrapper: late results reject after
return and actual stall/latency remains a hardware observation.

## Recipient-only key release

Roles are private configuration, storage AEAD key, trading seed, broker seed and
independent witness credentials. Each uses a different exact KMS key ARN, a
purpose/body commitment and encryption context binding release, role, generation
and stream. The trusted preparation tool creates a new 0700 local directory with
0600 files and syncs them; plaintext role files never belong on the parent, image
or in git. The bounded capsule format is not scalable customer onboarding.

Every Decrypt request creates a new RSA-2048 recipient, fresh NSM challenge and
purpose-bound quote; it is not a replayable shared recipient. Enclave TLS/SigV4
authenticates the fixed KMS endpoint and exact request. Require the configured
KeyId and SYMMETRIC_DEFAULT, request RSAES_OAEP_SHA_256, reject a plaintext response
and validate the decrypted role/domain/generation/stream and body commitment.
Recipient private keys cannot be exported and are dropped after one response.
Safe OpenSSL handles the bounded canonical CMS; no custom RSA/AES primitive or
workspace unsafe code is introduced. CMS decryption without an X.509 recipient
certificate is confined to the authenticated KMS response for that fresh public
key, not exposed as a caller-controlled decryption oracle.

Synthetic CMS tests establish fresh-recipient separation/replay rejection, not
AWS's OAEP envelope compatibility. Correct/wrong/debug/context KMS denial and
real CMS interoperability must be observed on hardware. KMS disable only prevents
future release; loaded-key fencing also requires current witness/epoch, bounded
leases, enclave termination and future native-agent revocation/reconciliation.

## Closed cloud storage and fresh witness

`cloud::Client` uses exact `aws-sigv4 =1.5.1` and `aws-credential-types =1.3.0`,
not a handwritten signer or full SDK middleware. Exact serde 1.0.229, serde_json
1.0.151 and base64 0.23.1 provide bounded explicit protocols. Signer features are
only `sign-http` and `http1`; default providers, retries, discovery, environment
credentials and redirects remain absent. The closed dependency policy explicitly
records the 37 added registry packages and feature changes. The current Cargo.lock
SHA-256 is `7550e0a91ed766e2c7ed2d40fd2be2d21697f66e7764864ef5a562332ad8eaf5`.

All AWS requests are one-shot TLS 1.3 inside the enclave with exact hostname,
consumed CA, NSM certificate time and bounded socket/elapsed budgets. Temporary
STS credentials have explicit finite expiry; no long-lived credential fallback.
S3 uses its required signed payload-hash header for PUT and empty GET. A narrow
Content-Length HTTP profile rejects ambiguous headers, redirects, compression,
chunking and over-size/truncated bodies. Actual AWS response compatibility is
still a hardware qualification dependency, not assumed from unit fixtures.

Two immutable content-addressed S3 replicas use conditional put and exact existing
object read-back. The journal verifies hash chains and AEAD. Region-local DynamoDB
uses strongly consistent reads and exact Epoch/prior Sequence/Hash CAS; writes
change only Sequence/Hash. The row must be explicitly provisioned. Missing row,
incomplete head, zero hash, failed CAS or expired credentials fail; no boot resets
the witness or chooses an old local file. Only an authenticated empty pre-provisioned
anchor can start a new journal.

The parent knows only KMS-recipient/S3-ciphertext credentials. The distinct witness
credential is released privately inside KMS. Different credential IDs or bucket
names alone do not prove IAM separation or failure-domain independence. Hardware
denial tests, narrow policies and declared administrative trust are required;
production G03 governance is not approved by this disposable topology.

## Pre-hardware package and handoff

[Packaging](../../tools/nitro-runtime/README.md) builds the default-feature enclave
and parent/operator tools separately from a source-only public vendor export.
The enclave rootfs includes only the ELF, five explicit runtime libraries, loader
and public manifest. No shell, toolchain, fixture root, source/research, AWS CLI,
wallet, owner directory or plaintext role files enter it. The selected OpenSSL
provider/kernel RNGs still need measured-kernel observation; NSM GetRandom does
not alone qualify OpenSSL or journal OsRng. A same-input recompilation comparison
is a local reproducibility check, not an EIF/PCR or universal reproducible-build
guarantee.

The [operations runbook](../operations/nitro-qualification.md) defines trusted
preparation, exact resources/permissions, bounded startup, secret-safe coarse
health, SDK policy delivery, failure/restart/fencing tests and budget/cleanup.
Final public resource identities, actual EIF measurements and independently
approved KMS/SDK PCRs can only be recorded during the resumed hardware session.
All four complete P20 hardware criteria remain open. P21 recovery, P22 offline
workflows and P23 live capabilities retain their own scope and gates.

Final offline assembly receipt, 2026-10-02: exported source tree
`629c4a0fe4a04f0ef027c04481d9eeee1dabcff5`; same ARM64 check image and library
versions as the earlier Linux checkpoint. Linux and macOS each pass 332 Rust
tests plus two compile-fail doctests per debug/release profile, strict default/
all-feature Clippy and build. Pinned macOS runner also passes 28 repository
tests and all 17 SDK tests. Linux tests do not establish real NSM/vsock/AWS
behavior. No on-chain source changed, so SBF was not rerun.

Default-feature enclave ELF SHA-256:
`d0679f2d0fa908369f1816413dd3333b9d2fb86862f54f89dcf3b7a533dc0399`.
Fresh application recompilation is byte-identical. All seven parent/operator/
enclave binaries and the exact runtime-library hashes are preserved in the local
temporary `/private/tmp/cinder-p20-final-06tHDY/bundle/SHA256SUMS`, outside git.
The minimal checked rootfs loads its explicit libraries in an isolated chroot
and refuses ordinary non-Nitro boot with only the redacted error. Its finalized
public manifest is still a hardware-session input; this refusal is not a real
NSM boot or a measured EIF receipt. The task test containers have been removed;
no source, research, wallet or cloud configuration was deleted.

## Primary sources

Checked 2026-10-01: [pinned AWS NSM API/driver source](https://github.com/aws/aws-nitro-enclaves-nsm-api/tree/v0.5.2),
[AWS KMS enclave integration](https://docs.aws.amazon.com/enclaves/latest/user/kms.html),
[KMS recipient contract](https://docs.aws.amazon.com/kms/latest/APIReference/API_RecipientInfo.html)
and [supported enclave instance constraints](https://docs.aws.amazon.com/enclaves/latest/user/nitro-enclave.html).
Timestamp/nonce semantics rechecked 2026-10-02 against
[AWS's attestation document and validation contract](https://docs.aws.amazon.com/enclaves/latest/user/verify-root.html).
Socket semantics checked against the pinned
[socket2 source](https://github.com/rust-lang/socket2/tree/v0.6.5) and
[Nitro parent/vsock concepts](https://docs.aws.amazon.com/enclaves/latest/user/nitro-enclave-concepts.html).
TLS hostname/time/store behavior checked against the pinned
[OpenSSL Rust connector source](https://github.com/sfackler/rust-openssl/blob/openssl-v0.10.81/openssl/src/ssl/connector.rs).
Cloud contracts checked 2026-10-02 against
[KMS Decrypt](https://docs.aws.amazon.com/kms/latest/APIReference/API_Decrypt.html),
[recipient policy conditions](https://docs.aws.amazon.com/kms/latest/developerguide/conditions-nitro-enclave.html),
[DynamoDB strong GetItem](https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_GetItem.html),
[conditional UpdateItem](https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_UpdateItem.html),
[S3 conditional PutObject](https://docs.aws.amazon.com/AmazonS3/latest/API/API_PutObject.html)
and the [official pinned SigV4 interface](https://docs.rs/aws-sigv4/1.5.1/aws_sigv4/http_request/index.html).
AWS SDK C's KMS/attestation implementation was inspected at
`cd61b6187c8b20867ba4368d1ae62c5790c0269a` as a recipient-envelope reference;
it is not a dependency or substitute for actual AWS CMS/OAEP qualification.
