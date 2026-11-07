# Retained-pack Nitro qualification receipt

Date: 2026-10-07. **All fixed read-only application cells, ciphertext archive and
independently verified owned cleanup passed. This invocation is closed. P23
financial qualification remains open.**

Authority: [approved run contract](P23-NEXT-RUN-PROPOSAL.md). This receipt records
observations, not a production-capacity recommendation, independent security
audit, deployed format migration or deposit/trading/recovery qualification.

## Exact release and environment

- Application source: `0845ca67b93134783156d0ce8ce58f0f4c8b9b47`.
- Enclave ELF SHA-256:
  `653e1643770084ad58b1120a46d21e8c40032b04a40734b9909ee2ed75474209`.
- Main EIF SHA-256:
  `dcc7099a1141a3f206ea2ac249e939d44691e59f1e24739c3e0e138023cb6172`.
- Main public manifest file SHA-256:
  `1fe5f92fcac0e4e2df21cd090c356b342dedfa452cd58851e96d69439f674087`.
  This is the file hash, not its semantic release digest.
- One `c6g.large`, us-east-1, no instance role, encrypted 16-GiB auto-deleting
  root, operator-IP-only SSH and four-hour automatic termination.
- Fresh `p23-20261007-pack1` / `pack1b` resources: six purpose-separated KMS
  keys, two private ciphertext buckets, one witness table and two bounded roles.
- Version 3: 64-KiB original frame / 8-MiB original history / 256 accepted-frame
  ceilings; ordinarily 512 PUT attempts / 128 MiB per replica per boot.
  Financial/native-read/risk-admission gates remain off.

Main PCRs, independently checked before release and pinned by the actual SDK:

| PCR | SHA-384 measurement |
| --- | --- |
| 0 | `d494ee6da6233551ebc1d3d4d564a9fdab556a7b62421a5d6d0556d6230bb5b0a7ee7f140ef641183e8ab45e9868840f` |
| 1 | `3b4a7e1b5f13c5a1000b3ed32ef8995ee13e9876329f9bc72650b918329ef9cf4e2e4d1e1e37375dab0ba56ba0974d03` |
| 2 | `4e1f67b420d2cd6b1c2ed22b2c2dcc52d4f140a73c78076366742015a4b1e9b65ca365b21ac696855342bdf7319196a6` |

Separate fault/expiry profiles use fresh identities, keys inside the capsules,
streams and manifest commitments, with the same application ELF and wrapping
KMS resources. Each new EIF is independently inspected and its exact PCR/context
policy read back before release. No accepted witness is reset or reused.

| Profile | EIF SHA-256 | Boot lease / per-replica PUT budget |
| --- | --- | --- |
| Main | `dcc7099a1141a3f206ea2ac249e939d44691e59f1e24739c3e0e138023cb6172` | 40 minutes / 512 |
| Repair | `4ec167fdec41c2228693230e5ed85c2932f6a721544c383b3b894e266ae6e983` | 40 minutes / 512 |
| Quota | `5a49b0942f4d7edf59f06ea007c0bde8d8c46f7d06cb340127eeca3a3671ad85` | 60 seconds / 1 |
| Lease | `9a12373d21e4fbd0d7955e234adc4c570c63fddb537480a4e576b4f4b7f3e08e` | 90 seconds / 512 |
| Witness expiry | `dc8eb0c751f37f61137a661a74f261e96d458c7cbc7ea9a7086e2482f00d404c` | 40 minutes / 512; witness STS 900 seconds |

The lease stream's witness credential remains valid beyond its test. Conversely,
the witness-expiry stream's enclave lease and parent credentials remain valid
beyond witness expiry. Those independent boundaries prevent one expiry from
masquerading as evidence for the other. The instance's independent four-hour
shutdown and the operator's eight-boot bound are separate again.

## Fixed acceptance cells

| Cell | Observation | Result |
| --- | --- | --- |
| Package and measured release | All thirteen uploaded hashes match; EIF v4 CRC/entrypoint checked; non-debug boots; six recipient/context policy readbacks and parent witness/plaintext-decrypt denials | passed |
| Actual Node/Chrome mixed HTTP/WS at heads 2/8/32/64 | Shipping SDK/WASM, real NSM/AWS-root attestation and Noise NK; one/two watches plus parallel own-account reads at each exact independently read head | passed |
| READ authentication | Wrong measured pin and replayed quote refused; own-only ten read families; disposable READ grant, owner revocation and natural grant expiry | passed |
| Lost reply | Original ID and SDK digest saved before exposure; encrypted reply discarded; original lookup matches; no resend | passed |
| Exact process restart | Same accepted head 64, auth epoch 3 and original receipts/digests; no seed, padding, witness reset or mutation retry | passed |
| Authority epoch fence | Witness epoch 1→2 preserves sequence/hash; private connection/read refused | passed |
| Old required pack missing | Delete one owned old copy; intact replica used; exact old ciphertext repaired/read back before next acceptance | passed |
| Old required pack corrupt | Flip an opaque byte in one owned old copy; intact replica used; exact repair/readback before next acceptance | passed |
| Repair denied | Actual same-parent-role S3 PUT is denied for the single old object; one fresh READ grant cannot advance head/hash; original is not resent | passed |
| Fault-stream restart | All eighteen accepted original receipts/digests retained; failed original remains not-found; head 20 unchanged | passed |
| Witness unreachable | Stop only the owned witness proxy; private connection/read refused; strong accepted head 20 remains unchanged | passed |
| PUT quota | One PUT attempt per replica accepts only genesis (head 0); both replicas contain only that genesis object; private channel refused; six keys enabled and host running | passed |
| Natural finite boot lease | Healthy fresh account before the 90-second measured lease expires; private read refused after natural expiry; head 2 unchanged, six keys Enabled and host running; no clock/credential manipulation | passed |
| Natural witness credential expiry | Healthy fresh account before natural 900-second STS expiry; private connection/read refused afterward; head 2 unchanged, parent credentials valid, six keys Enabled and host running | passed |
| Ciphertext archive | All writers stopped; retain and independently rehash all 188 ciphertext objects (1,183,396 bytes) and five strong witness heads | passed |
| Owned teardown | Instance terminated; test EBS, buckets, witness, roles, SSH key and security group absent; six test keys PendingDeletion; pre-existing managed policies preserved | passed |

## Measured main-stream timings

Parallel own-account reads in milliseconds; one/two columns denote watch count,
not distinct production capacity limits. These are single-run observations.

| Accepted head | Node: one / two | Chrome: one / two |
| --- | --- | --- |
| 2 | 840 / 1,074 | 1,027 / 1,073 |
| 8 | 912 / 1,056 | 926 / 1,076 |
| 32 | 915 / 1,062 | 873 / 1,140 |
| 64 | 937 / 1,086 | 2,704 / 892 |

The 32 grants growing head 32→64 have recorded reply times of 2,147–2,606 ms
(median 2,440.5 ms). Application reply/I/O/session/freshness deadlines are unchanged.
Head 64 means **65 zero-indexed accepted frames**, including genesis and setup;
there are 62 post-initialization authentication operations. No financial position
or nonzero customer balance is used to manufacture growth.

The HTTP/WebSocket carrier uses the shipping opaque web relay on local loopback,
an authenticated SSH tunnel to the owned parent ingress, and vsock into the
actual Nitro application. Client/enclave authentication and encryption are real;
this is not qualification of a public HTTPS deployment, wallet onboarding flow
or trading terminal. The SSH observation connection was renewed once within its
40-minute operator bound; no enclave lease, session or witness credential was renewed.

Natural witness expiry was observed at `2026-10-07T10:17:06.006Z`. The enclave's
credential/freshness guard refused use after the real STS deadline; no AWS
`ExpiredToken` response is claimed. The witness head stayed at 2, all six wrapping
keys were Enabled, the parent instance remained running, and its separate STS
credential had not expired. All owned application workers were then stopped.

## Traffic and retained evidence

Final parent HTTPS connection counts are S3-first 580, S3-second 379, witness
1,134 and KMS 42. Native/RPC HTTPS connections and API requests are zero; all
financial gates stayed off. Startup DNS/address-selection calls did occur,
including route probes, so this is **not** a claim of zero native-host network
metadata. Pure connect traces retain destination/port metadata, not TLS payloads.
The observed parent `ens5` transmitted 143,896,858 bytes, below the 1-GiB bound;
this is an interface observation, not an exact billing/whole-run traffic meter.

After all writers were stopped, both stores' 188 objects were archived locally
and independently checked against their retained sizes and SHA-256 hashes.
The five strongly read final heads are main 64 (epoch 2), repair 20, quota 0,
lease 2 and witness-expiry 2 (the latter four at epoch 1). This preserves orphan
and partial-tail versions as evidence, not just accepted packs. No plaintext
archive or witness reset is used.

Independent teardown verification completed at `2026-10-07T10:30:46.136Z`.
The host launched at 09:15:47 UTC and was confirmed terminated by 10:25:15 UTC,
about 69.5 minutes, within the four-hour cap. Local opaque relay and SSH tunnel
also exited. Test buckets and their cloud ciphertext copies, witness table,
runtime roles, SSH key and security group were deleted; the encrypted root volume
is absent. The six owned KMS keys are unusable in PendingDeletion with seven-day
deletion scheduled. Ciphertext and receipts remain recoverable from the local
private archive; the closed invocation's credentials/resources must not be reused.
No pre-existing managed policy, older key or user-owned resource was removed.

The final active-invocation operator ledger records 47 STS, 74 IAM, 52 DynamoDB,
431 S3, 198 KMS and 30 EC2 attempts, including setup/archive/cleanup. Adding the
rejected initial attempt, the two explicit read-only diagnostic calls and each
runtime HTTPS connection gives conservative request-bound totals of **242 KMS,
1,392 S3 and 1,188 DynamoDB**, below 512/10,000/10,000. These are request-budget
observations, not final billing. Seven non-debug boots used the one host; no
replacement, new acceptance case or financial activity was added. Billing can
lag; no precise all-in AWS bill is asserted.

## Limitations

Original private receipts, measurements, role/capsule files, operator scripts,
opaque fault archives and the owned-resource ledger remain local and ignored
under `work/experiments/p23-live/run-pack1b/` and `packed-tools/`. The tracked
receipt deliberately excludes private seeds, credentials, account IDs, signed
commands and raw attestation contexts. Test tooling is distinct from the measured
shipping application. Disposable plaintext preparation material never goes to
the parent, but the trusted test preparer knows it locally: this is **not** an
operator-independent production key ceremony. Two buckets and a separate role in
one AWS account do not prove independent administration or failure domains.

The earlier provisioning attempt was rejected before any KMS key/host/ciphertext
existed and its two empty roles were removed, independently verified. The corrected
operator policy is not a production privilege recommendation. During quota
preparation a read-only KMS CLI call failed; identity/grant diagnostics succeeded,
all mutations were acknowledged and the same exact pre-boot policies were
reverified before continuation. No hardware acceptance cell was retried to hide
failure, no financial command was sent, and no shipping code changed.

Full-history bytes still grow with history, and retained partial tails duplicate
prefixes. Passing the bounded head-64 workload is not indefinite journal scaling,
full-size live memory qualification or an approved checkpoint/GC scheme. The
[financial gate map](P23-FINANCIAL-GATES.md) remains separate: actual funding,
venue credit, order lifecycle/funding and returned-backed recovery are unqualified
by this read-only run. P23 stays in progress; no stack merge follows automatically.
