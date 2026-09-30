# ADR 0006 — Encrypted records, replication and fenced acceptance

2026-09-30. P06 implementation boundary, not Nitro/production qualification.
Extends [ADR 0005](0005-durable-journal.md); implements BASELINE S03 and the
durable portion of S04. P19/P20/G03 still own authenticated remote services,
attested release, independent administration, native fencing and qualification.

## Decision

Use the existing journal with `RecordCipher` and `Replicated<A,B,W>`. No second
financial ledger. Pin RustCrypto `chacha20poly1305 =0.10.1` and `zeroize =1.8.2`;
the existing pinned SHA-256 dependency commits opaque frames. The reviewed graph
adds 13 registry packages, no extra build scripts, and no kernel dependency.
Exact versions, features, checksums and graph remain guarded. This deliberately
uses the inspected 0.10.1 API, not an automatic update to a newer major minor line.

[Upstream cipher documentation](https://docs.rs/chacha20poly1305/0.10.1/chacha20poly1305/)
describes XChaCha20-Poly1305 and its 192-bit nonce; we use that implementation,
not custom cryptography. [Zeroize](https://docs.rs/zeroize/1.8.2/zeroize/) protects
the retained key and `PrivateBytes` allocations on ordinary drop. That does not
erase every transient/caller copy, guarantee register wiping or survive SIGKILL.
The key has no Debug/Clone/export API. Production entropy, platform suitability,
release/rotation and independent cryptographic review remain qualification gates.

## Record and replay-snapshot formats

Each opaque frame body is:

```text
ASCII CINDER-AE1 | key_generation:u64BE | random_nonce:24 | ciphertext | tag:16
```

AAD binds the storage-record domain tag, stream ID, network/deployment IDs,
sequence, predecessor digest and complete header. Every proposal draws a fresh
nonce from fallible OS randomness, including failed CAS/retry proposals at the
same sequence. We do not derive nonces from a rollbackable counter. Unknown
generation/version, altered AAD/body, wrong stream/key and truncated bytes reject
before private parsing. Plaintext capacity is the 1 MiB record limit minus header
and tag; oversized bodies fail before acceptance.

Frame encoding is `sequence:u64BE | predecessor:32 | digest:32 | length:u32BE |
opaque`. Frame digests are over ciphertext and existing context, not bare private
financial hashes. Public object names are ciphertext digests. Host-visible timing,
length, object counts, sequence and key generation remain metadata; no traffic
padding or traffic-analysis guarantee is claimed.

A snapshot is a versioned, stream-bound **complete encrypted replay archive**:
`CINDER-SNAPSHOT-1 NUL | network:32 | deployment:32 | stream:32 | count:u32BE |
(frame_length:u32BE | frame)*`. It does not contain plaintext state or introduce
an unaudited shortcut around replay. It preserves all consumed IDs, attempts,
receipts and private observations. It is a bounded restore format, not compaction
or a performance-optimized projection checkpoint. Full accepted history remains
required (4096 records, 64 MiB opaque data); no retention/GC feature is implied.
Empty/uninitialized streams have no restorable financial snapshot.

## Acceptance and failure ordering

1. Validate exact trusted stream and writer epoch against the fresh witness.
2. Compute the existing atomic financial transition and encrypted frame.
3. Durably write/read back both distinct configured replicas. Check and repair
   both copies of earlier accepted history; failed repair prevents new acceptance.
4. Atomically compare-and-set `(epoch, accepted_head)` at the witness.
5. Check the resulting authority/head, then release the journal receipt or exposure.

Acknowledgement therefore requires both copies and durable witness commitment.
Before step 4, objects are unaccepted orphans; they never advance financial state.
Content addressing lets a successor retry the same sequence without deleting or
replaying an orphan. A failure after acceptance is an **unknown outcome**, not a
rejection: the handle poisons, replay recovers the original transaction, and exact
retry does not re-expose it. New heads are bounded before witness acceptance.
Only a known stale CAS rejection is propagated as such. Every other witness-accept
error, including an incorrectly returned remote `Busy`, becomes an uncertain
storage outcome and poisons the caller. Read-only recovery uses either valid copy
lazily and does not require the other replica to be writable; this availability
does not authorize further single-copy acceptance. Repair is ciphertext-only and
read back before acceptance; it cannot change the independent accepted head.

`Witness` is an authenticated, replay-resistant, durably linearizable port, scoped
by stream and caller authorization. It is not implemented by the parent-host file
adapter. The operator provisions its nonzero epoch; only an authorized external
epoch transition may fence/replace a writer, preserving the accepted head. The
append API cannot rotate epochs or reset a missing register. Authentication, quorum,
admin separation, cost and actual failover topology need G03/P20 review.

`FileReplica` uses bounded reads and fsynced temporary ciphertext files followed by
rename/directory sync. Two references to the same canonical directory reject.
Other adapters supply stable storage identities. Distinct identities are necessary,
not evidence of independent machines/accounts/deletion rights. Two directories on
this test machine are **not** independently durable against its loss. A hostile
host can delete data or deny service; fsync tests are not power-loss certification.

## Restore and fencing

Read the independent current head, walk its exact digest chain using either valid
copy per record, then recheck head/epoch. Journal replay separately authenticates
each AEAD body, expected configuration, transitions and derived commitments.
No valid copy of an accepted record, wrong keys or unavailable witness stops
restoration; an older internally valid history is not a replacement. A replay
snapshot may rehydrate copies only if its final head equals the current witness.
It cannot advance/rewind that register. Already-present orphans are not authority.

Every commit, including duplicate receipt delivery, checks the current witness.
`verified_state()` checks before serving a current-authority view; `state()` is
only a cached local projection. A check linearizes an operation; it cannot stop an
epoch advancing immediately afterward. Old loaded storage keys alone cannot pass
the backend's witness CAS, but this is **not revocation of a venue signature or
credential already exposed**. P14/P16/P20/P21 must reconcile/fence those separately.
No heartbeat initiates recovery or releases payments.

Readback/restore walks the bounded history; this prioritizes explicit correctness
over throughput. Latency, resource SLOs and archive compaction are not qualified.
Orphans are retained; capacity exhaustion stops new acceptance rather than deleting
possibly needed evidence. No production witness, KMS client or AWS resource is
created. All tests use synthetic keys and private data.

## Evidence and limits

Tracked `encrypted.rs` tests cover AEAD mutation/truncation/domain binding, random
nonce proposals, wrong key/generation, replica loss, stale authentic tails, missing
witness, orphan retry, lost witness reply, loaded-key stale writer, aliased storage,
parallel writer CAS and snapshot rehydration. `encrypted_crash.rs` kills real
children after copy one, copy two, before witness CAS and after its durable commit.
The trusted SQLite witness there is explicitly a test double, not an independent
security service. Existing journal crash/atomicity and financial suites remain.

Promotion reuses M2's required scenarios, not its JS state machine or authorities:

| Local source | SHA-256 |
| --- | --- |
| `work/experiments/private-recovery/CLOSEOUT.md` | `42072e3058379769f799fc64ebc8de725e1ceeda269717fa1b334a7ac8295106` |
| `work/experiments/private-recovery/durable-store.mjs` | `5e7cae1a55054be079347892216c23d5ba8838de7b6b36bdc200d541f4c73a17` |
| `work/experiments/private-recovery/durable-store.test.mjs` | `7fc6655515c59e05c9499b70a34b5fdf5981d4fdec7b47beb79193d30295ca3a` |
| `work/specs/private-api-runtime.md` | `6ee0084666c8681f9b894a9d1a9c645997aa10d0bf352b257053f74798b03522` |

The tests establish behavior under the declared trusted-port assumptions. They
are not a machine proof, external audit, production entropy qualification, native
capability revocation, unconditional data availability or deployed Nitro evidence.
