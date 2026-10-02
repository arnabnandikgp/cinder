# ADR 0020: Nitro application assembly and qualification

Date: 2026-10-02. Status: implementation in progress; hardware not qualified.
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

## Remaining assembly and qualification

- TLS/library RNG integration; NSM entropy alone does not
  qualify OpenSSL or journal nonce generation. NSM ioctl timing/deadline behavior
  also needs Linux/hardware observation.
- Complete consumed-configuration manifest, measured executable, vsock ingress
  and fixed-destination parent relay; actual enclave-side authenticated egress.
- Recipient-bound KMS release with wrong/debug/replayed release rejection and
  key-role separation. No plaintext key delivered to the parent.
- Encrypted external replicas and separately authenticated fresh witness; test
  lost witness, rollback, competing writer and crash/restore. Local witness files
  and shared parent credentials are not independent freshness evidence.
- Bounded controller scheduling and reconciliation, with no automatic retry of
  escaped/unknown native actions. The P19 fake dispatch/flat holds are not promoted.
- Actual release/fencing rehearsal, secret-safe health/runbook, SDK operation
  through the tagged enclave, and cleanup/cost receipts.

No production topology, witness independence or live financial limits have been
approved by these changes. G01–G04 stay open as applicable. This slice cannot run
the financial application on AWS yet and does not satisfy any complete P20
hardware acceptance criterion.

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
billable test resources have been created; pricing, remaining narrowly scoped
permissions, runnable assembly and a bounded cleanup plan still precede launch.

## Primary sources

Checked 2026-10-01: [pinned AWS NSM API/driver source](https://github.com/aws/aws-nitro-enclaves-nsm-api/tree/v0.5.2),
[AWS KMS enclave integration](https://docs.aws.amazon.com/enclaves/latest/user/kms.html),
[KMS recipient contract](https://docs.aws.amazon.com/kms/latest/APIReference/API_RecipientInfo.html)
and [supported enclave instance constraints](https://docs.aws.amazon.com/enclaves/latest/user/nitro-enclave.html).
Timestamp/nonce semantics rechecked 2026-10-02 against
[AWS's attestation document and validation contract](https://docs.aws.amazon.com/enclaves/latest/user/verify-root.html).
