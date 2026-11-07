# Nitro hardware qualification: 2026-10-02

Status: partial hardware evidence; P20 remains **in progress**. This is a
disposable qualification run, not a production deployment or native trading test.
The user authorized a $5 total AWS allowance in `us-east-1` under a verified IAM
identity and explicitly upgraded the account to the Paid plan. The earlier Free
plan launch blocker is historical, not the current blocker.

## Actual release

One ARM64 `c6g.large` parent; one non-debug enclave, 1 vCPU / 1024 MiB. Amazon
Linux 2023 kernel `6.1.188-233.386.amzn2023.aarch64`, Nitro CLI `1.5.0`, Docker
`25.0.16`. Linux executable built offline with Rust `1.97.1` / pinned dependencies
and default features. The parent relays use the packaged Linux loader/libraries;
TLS, signature verification, KMS unwrap and financial state remain in the enclave.
No instance role, load balancer, NAT gateway, native venue relay or wallet is used.

The approved candidate includes the three compatibility fixes below and a public
manifest binding renewed, sealed witness credentials. Original storage/trading/
broker keys, domain, stream, generation, epoch and application contract are
preserved. Renewal happened before any journal head existed and did **not** reset
the witness. Temporary fixed-stage/no-key diagnostic images are not this release.

```text
enclave ELF SHA256
34d8172d69258ef5739622246b63a70429619c94269a31dcda4177589fac6264
EIF SHA256
ef952dee678fb106a8b46a1be52fbcdfd4b1354c9352e562f4cc42fea0a44364
PCR0
fe250bc0bea3a624a8195c5796b6058e1ab13bec87e41ffa72772c78a1ebeff9dc28225657b8f490dd06f02dc0a0d3b9
PCR1
3b4a7e1b5f13c5a1000b3ed32ef8995ee13e9876329f9bc72650b918329ef9cf4e2e4d1e1e37375dab0ba56ba0974d03
PCR2
4caab8b9f3e6038f4e064779f8529e14e8a644539dad8693258305956a4602dac93a4f161a91c843b07026804ecc4a63
```

Every KMS key policy pins exact non-debug PCR0/1/2 and exact release, purpose,
generation and stream context, with explicit mismatch denials. Policy readbacks
match; no grants exist. Parent credentials permit only recipient-bound Decrypt
and ciphertext Get/Put in the test stream. Witness Get/Update is a separate,
enclave-released credential restricted to the exact row. Administrative access
to all test services is still a declared trusted-operator boundary; two buckets
in one account/region do not establish production independence.

## Observed compatibility failures and fixes

1. Actual NSM payload uses an indefinite **top-level nine-field map**. Accept
   that closed form as well as a definite map, requiring exactly nine unique
   fields and the closing break. Nested indefinite data, duplicates, truncation,
   trailing data and bounds violations still reject. Verify the original signed
   bytes; never normalize a quote before checking its signature.
2. Actual NSM leaf lacks Authority Key Identifier. OpenSSL strict validation
   rejects it with error 85 at depth zero. The narrow compatibility path requires
   that exact error, a v3 leaf with absent AKI, a valid issuing relationship, a
   strict issuer-to-pinned-root CA path and a complete standard PKIX leaf path.
   P384/COSE signature, certificate times, depth/security level, pinned root,
   purpose, nonce and release checks remain required. No generic verification
   callback, new trust root or ignore-errors switch is introduced.
3. Actual recipient KMS returns a valid **BER CMS** envelope which does not
   re-encode byte-identically as DER. Replace DER round-trip equality with a
   bounded complete ASN.1 frame check (6144 bytes, depth 16, 256 objects) before
   OpenSSL CMS parse/decrypt. Reject trailing/truncated/primitive-indefinite or
   unbounded objects. Fresh recipient, exact response key/algorithm, no plaintext,
   purpose/domain/generation/stream and approved body-hash checks still apply.

The first two were reproduced with public no-key NSM probes. Fixed-stage KMS
diagnostics recorded only constant stage numbers and HTTP status, never plaintext
or response values. The final default-feature image contains no diagnostic hooks.
The [AWS attestation specification](https://github.com/aws/aws-nitro-enclaves-nsm-api/blob/main/docs/attestation_process.md),
[KMS recipient response contract](https://docs.aws.amazon.com/kms/latest/APIReference/API_Decrypt.html)
and [CMS BER specification](https://www.rfc-editor.org/rfc/rfc5652.html) are the
primary reference contracts; they are not substitutes for the observed receipts.

## Hardware results

| Check | Actual result |
| --- | --- |
| Non-debug boot, NSM, clock/entropy, OpenSSL and five-role release | Passed: real application initialized accepted encrypted history and opened attested TLS. This is functional evidence, not an entropy-quality or arbitrary-pause proof. |
| Owner-signed view and bounded grant | Passed through the real SDK, SSH loopback tunnel, opaque parent relay and enclave. Zero financial balances and a configured zero-quantity position; no seeded funds. |
| Revocation and current/stale authorization epochs | Passed: epoch advanced to two and the revoked epoch was refused. |
| Wrong SDK release, altered quote, old quote on fresh TLS socket | Passed before any customer request on those connections. |
| Disabled native order | Passed: unavailable; no native relay or dispatch. |
| Crash/restart | Passed with the same measured image and accepted encrypted history. |
| First replica removed | Passed: second replica restored the accepted head and revoked authorization remained effective. |
| Both replicas removed | Passed: boot exited and private API refused; accepted witness was not reset. |
| Both current-head ciphertexts altered | Passed: boot exited; accepted head unchanged. Restoring original ciphertext recovered service. |
| Loss of loaded enclave's witness route | Passed: private API refused, enclave exited, accepted head retained. |
| Retained-head writer epoch advancement | Passed: a serving loaded enclave stopped after epoch one-to-two advancement, with exact sequence/hash retained and KMS keys still enabled. |

Accepted witness reached sequence four (five immutable frames), with no financial
movement. Original ciphertext-only backups were used for repairs; no stale witness
was written, no new ledger initialized over a missing accepted history, and no
customer projection was printed. Early probe failures also included a timed-out
SSH tunnel, probing before boot completed, and an incorrect test expectation that
a zero-position view omitted configured markets. Those are not successful fault
tests or evidence of economic mutation; readiness and accepted state were checked
before the substantive observations above.

## Offline verification for the compatibility changes

- macOS ARM64: complete workspace all-feature **release** tests, including two
  compile-fail doctests; service default/all-feature strict Clippy and formatting.
- Linux ARM64 source-only, read-only, network-disabled Apple container: all 30
  service unit tests, eight service integration tests and strict all-feature Clippy.
- Pinned Node 24.21.0: all 17 SDK tests, including seven real separate-process
  transport tests, and TypeScript checks; all 28 repository tests/dependency guards.

These are current changed-source checks. The earlier full debug/release runner is
dated pre-hardware evidence, not a full-debug rerun for this source. No SBF source
changed, no new SBF deployment was performed and no independent audit is implied.

## Still required before complete P20 qualification

Dedicated hardware cases for individual wrong/debug/context/role recipient release,
old CMS response against a fresh recipient; real competing-writer CAS/uncertain
append; stale/orphan history beyond the tested missing/altered accepted-frame cases;
finite credential/lease expiry; upstream CA/hostname/ciphertext faults and native
origin TLS without financial activation. Keep synthetic coverage and actual AWS
receipts separate. Actual native/chain workflows and possibly-exposed native
attempts remain P23/P22 respectively, not fabricated by this zero-funds test.

All four complete PLAN hardware criteria remain open until their full matrix is
qualified, relevant reviews resolved and the PR merged. P19 is unmerged; this run
does not authorize a merge, a live capability or a production topology/policy.

## Cleanup / spend

Verified teardown completed on 2026-10-02. Instance launched at 11:44:52 UTC;
termination requested at 13:29:34 UTC, about 1 hour 45 minutes, and its actual
state was subsequently verified **terminated**. Exact test EBS volume is absent;
both buckets and their objects, the witness table, both runtime roles, SSH key
pair and test security groups are absent. All five KMS keys are **PendingDeletion**
with the seven-day service minimum, not immediately erased. Preexisting managed
operator/boundary policies were preserved. Cleanup returned no errors.

Removed exactly 13 generated local plaintext/credential/private-key files after
teardown; retained public/ciphertext receipts. The source-only task container and
bounded SSH tunnel are gone. No user wallet, research, unrelated container or
preexisting cloud resource was deleted.

Conservative estimated total for compute, IPv4, small encrypted EBS, short KMS
lifetime/requests and bounded S3/DynamoDB requests is **below $0.50**, within the
$5 authorized allowance. This is an estimate from observed resource lifetime and
the refreshed $0.068/hour c6g.large rate, not a finalized billing invoice. AWS
billing/credit updates lag; credits and alarms are not a hard budget mechanism.
