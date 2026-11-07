# Two-unit Nitro qualification — 2026-10-20

Read/admin-only changed-image qualification, not live financial activation.
Historical exact-source receipt: October 21 recipient/initialization-lease fixes
change the application. This receipt does not qualify their new image; a fresh
bounded run is required after the final source is ready.
[Scope and reproduction](../../operations/unit-qualification.md),
[capability comparison](../../venues/experimental-parity.md).

## Exact artifact

- Application source: `91a01ffaa7530297bdde7ac445a44cb5f6163264`.
- Default features; Rust 1.97.1, source-only network-disabled ARM64 build.
  A fresh application rebuild is ELF-identical; ordinary-host NSM refusal passes.
- Tool image index: `sha256:33107aa751a9d69fb65acb26474fe3954dd30f8580b4150b4a63daf58bac434a`.
- Application ELF: `4f2a026e75563763b76b1c91d1c16810c8931c5b630a3225ec0ad8e92f6f71a2`.
- Corrected public manifest SHA-256: `f1392525202b94d22c332b1eb3c43e44d41be4e6727073eeb64e0250507d38c1`.
- Manifest domain-separated digest: `f80cd572b53e2f1d628f27423691b46e4fdbf5ed615e87c2d88259764ab89f29`.
- Final EIF: `628cb2fb91d87431e1300f6cc2f0beae442a1a14135477160955f44936885f6f`.
- PCR0: `1448e869a572a51f59b0f161af37de09c24b65bb40dacff74b86d7d3691bae373c60415158f7c2102dd129a68f83e80e`.
- PCR1: `3b4a7e1b5f13c5a1000b3ed32ef8995ee13e9876329f9bc72650b918329ef9cf4e2e4d1e1e37375dab0ba56ba0974d03`.
- PCR2: `b107c31235da25343f2d078db1d3de007a18600c4aa0d22eade046b1f27c707f07313348b71315fe7289e3bc266bc68a`.

Independent `describe-eif` checks CRC, EIF version, measurements and the exact
`cinder-enclave --web` entrypoint before all seven KMS policies and the parent
inline policy are read back. Non-debug hardware reports the same measurements.
Only public package files and recipient ciphertext reached the parent. Private
role files and disposable client signing seeds remained local to the trusted
operator. No fixture feature, native owner/funds key or synthetic balance was
deployed.

Post-run review tightens only the trusted local `qualification-units` converter:
it now always emits the approved 2,400,000ms lease rather than inheriting a legacy
value. An offline one-hour input reproduces the old refusal regression and passes
after correction; a permanent unit test covers both read flags and short/exact/
one-hour inherited leases. The executed manifest already used 2,400,000ms. This
helper-only successor changes no measured enclave code, image or historical
hardware result.

## Observed checks

| Check | Result and limit |
| --- | --- |
| Parent isolation | Actual plaintext KMS decrypt and witness-read probes denied; recipient-bound KMS key release succeeds in the measured enclave |
| Node 24.21.0 | Production AWS-root attestation and Noise, HTTP/WebSocket UnitClient access to both units; signed account/operation/ingress reads pass |
| Chrome | Actual headless browser runs the same production SDK/WASM against AWS, not a fixture root or a recorded page; HTTP/WebSocket checks pass |
| Authorization | Wrong measured release and cross-account/unit access refused; BULK-only READ agent works only in its unit; revoke through Pacifica invalidates it globally |
| Durable restart | Same image, capsules, history and identities; original grant digest/unit/outcome and global epoch 3 restored without re-enrollment or replaying mutations |
| Independent fence | Exact accepted-head CAS advances witness epoch 1 → 2 while preserving sequence/hash; both units subsequently refuse private state and the enclave exits |
| Native query bounds | Two account requests per venue across the two healthy boots; no order, native registration, transfer, faucet, Solana RPC or financial exposure |

The fresh Cinder books remain zero. That is a statement about Cinder's initialized
ledger, not a fabricated native balance or proof of settled venue credit. Native
query outcomes cannot activate the financial gates or certify causal coverage.

The offline archive audit verifies 28 ciphertext objects / 234,426 bytes, equal
replica inventories and all 14 accepted frames' pack/chain and AEAD integrity.
The retained fenced witness snapshot is epoch 2, sequence 13; this offline audit
does not claim a new live freshness check. Both BULK account responses are HTTP
404 with an error body. Pacifica retains two original 120-credit-tenths read
reservations and no accepted account observation: its result is unavailable,
not zero. The ordinary Pacifica read path does not retain the exact non-200 or
unknown reply code, so that distinction cannot be recovered from this archive.
No replacement request was made to guess it. The archive contains zero normalized
financial input events.

## Setup failures retained, not counted as passes

Four boots were consumed, below the eight-boot cap. Boot 1 failed because the
trusted packaging script wrote the public host loader/relay files mode 0600.
Correcting their execute bits changed no application/image bytes. Boot 2 released
all seven roles but failed fresh storage initialization: copied experiment
tooling used `ro2-*` bucket names while the installed boundary allowed only the
approved `units-*` pair. The runtime boundary correctly denied access.

The stopped boot had no accepted witness head and both mistaken buckets were
verified empty. They were removed and replaced with the exact approved names;
the witness was not reset. Trusted local re-preparation preserved all identities,
storage/trading keys, generation and stream, renewed only the finite scoped
witness credential, and changed the public routes. The new EIF measurements were
independently inspected and repinned before boot 3. A new local preflight compares
bucket names with the approved boundary before provisioning. No IAM boundary was
broadened. Original manifests/images and correction receipts are retained.

Boots 3/4 passed initial access/restart. One immediate restart connection arrived
before replay/listener readiness; a later fresh read-only session passed. No
administrative mutation was resent. The ciphertext collector was corrected to
recognize the manifest's exact `packs-1` prefix rather than a legacy frame key;
this changed no remote data or runtime behavior.

<a id="retention-and-closure"></a>
## Retention and closure

Ignored `work/experiments/p40-units/` retains the exact operator tools, public
package/image/measurement records, failed-run receipts, original administrative
intents, client results, offline audit and owned-resource/request ledger. These are not checkout
or CI dependencies and contain no customer identities. Only sanitized evidence
belongs in tracked documentation.

Independent cleanup verification confirms the exact host terminated, no tagged
EBS volume, both approved buckets and the witness absent, both runtime roles
absent, and SSH/security-group resources absent. All seven KMS keys are
`PendingDeletion` with seven-day scheduling; they cannot be used for new crypto
operations in that state. The two empty mistaken buckets were also removed.
User-created `CinderP40Operator`/`CinderP40RuntimeBoundary` policies are preserved.
Local task-owned relay/tunnel processes are stopped. No customer/mainnet data or
assets were deleted; encrypted test evidence remains in the local archive.

Host launch: 17:37:26 UTC; termination confirmed by 18:15:05 UTC. The session
stayed within its one-host / $5 / four-hour / eight-boot limits (four boots used).
Operator request ledger at cleanup verification: STS 26, IAM 40, DynamoDB 19,
S3 104 and KMS 100, EC2 24. Runtime connection-only trace totals: KMS 21, first
replica 56, second replica 41, witness 173, Pacifica 2 and BULK 2. These totals
remain below the retained run caps. Usage/billing settles later; an estimate is
not an invoice or a claim that every cent is already reported.

This run does **not** satisfy P40's separate live paper lifecycle through the
assembled service. No capital, native credit, margin/risk cut or complete
financial frontier was supplied. BULK funding/return rails remain unqualified;
Pacifica's missing-credit investigation remains deferred. Read-only hardware
qualification must not be promoted to production, native financial parity,
venue-backed recovery or customer-fund approval.
