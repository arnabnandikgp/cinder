# Retained encrypted history packs

Date: 2026-10-07. **Local implementation and shipping-port preparation approved;
changed-image hardware qualification remains open. No deployed migration.**
The [growth diagnosis](../implementation/P23-GROWTH-DIAGNOSIS.md) established
history-dependent object calls. This candidate reduces those calls without
removing records or the later historical-copy repair boundary.

## Format and authority

`journal::packed::Packed` implements the existing Backend. No new dependency,
financial equation, API wire, cipher, timeout or witness schema is introduced.
`Loaded::open` now selects bounded S3 packs only for an explicit version-3
manifest. Versions 1/2 retain legacy per-frame Replicated storage, their original
digest and role envelopes. There is no fallback between formats. A missing pack
at an accepted legacy head refuses boot; it never resets or initializes that head.

- Fixed canonical groups contain sequences `[16k, min(16k+15, head)]`.
Sixteen is the format parameter, not an approved service capacity limit.
- `CINDER-PACK-1\0`, network/deployment/stream IDs, a big-endian count, then
  length-prefixed original frame encodings. Frame hashes and original AEAD
  context/nonce/key generation are unchanged.
- Every append creates an immutable tail version keyed by its ending frame hash.
  Full groups become predecessor packs. The first frame's previous hash leads
  to the preceding group's ending hash. No trusted listing, mutable latest
  pointer, side index or plaintext checkpoint exists.
- The independent witness still accepts the exact original `Head(sequence,hash)`
  and writer epoch. That hash transitively binds the entire chain. Canonical
  groups reject alternative partitions, missing/duplicate/reordered frames,
  wrong context/head, trailing bytes and invalid frame hashes. Hash chaining is
  not a substitute for AEAD or authenticated witness freshness.

### Bounds and costs

Absolute codec bounds remain: 4,096 frames, 64 MiB total opaque history, 1 MiB per
opaque frame. A pack is at most `MAGIC.len()+100+16*(MAX_RECORD+80)` bytes,
approximately **16 MiB**, including headers. The local FileStore checks size
before reading, bounds the read itself, rejects direct symlink objects and uses
private directories, exclusive temporary files, fsync/rename/readback.

Version 3 explicitly binds `HistoryPolicy`: at most 64 KiB per original opaque
frame, 8 MiB total original history and 256 accepted frames including genesis.
Its largest encoded pack is **1,049,969 bytes**. These are qualification ceilings,
not calibrated production retention/capacity. Lower policies are permitted; an
existing history exceeding one refuses, never trims. The local growth fixture
uses these same frame/history/count ceilings.

`S3Packs` uses a separate `/packs-1/<ending-frame-hash>` namespace and checks
Content-Length before allocating a response. Only bounded S3 GET/PUT can use the
pack response limit; KMS, DynamoDB and legacy frames keep their original bound.
Existing TLS trust, signed time, credential expiration, ten-second I/O bound and
no-retry behavior remain. The ciphertext response buffer moves into the decoder
without a second body copy. Complete history and transient decoded packs still
occupy memory; full-size and browser RSS checks are separate from Nitro capacity.

Each replica has manifest-bound, per-boot PUT attempt/byte allowances, consumed
before signing/I/O, including failed or uncertain writes and repair. Maximums
are 1,024 attempts and 128 MiB per replica per boot. A failed charge performs no
I/O and cannot move the witness. These are not fleet-global quotas: restarting
does not recover a spent allowance or constrain unlimited authorized boots.
The hardware run therefore also requires independent whole-run request/object/
byte caps. Exhaustion refuses writes; it must not reset history. There is no GC.

Healthy append with N existing accepted frames and M=ceil(N/16) required packs
performs **3M+2 GETs and two PUTs**, instead of 3N+2 GETs and two PUTs. At existing
head 50 this is 14 GETs rather than 155; at existing head 64, 17 rather than 197.
Four direct witness reads plus one CAS remain; witness implementations may also
read inside CAS (the prior diagnosis counted that read).

All historical bytes are still read/verified. Total byte I/O across successive
appends remains quadratic; fewer requests is not constant-time persistence.
Retaining every partial-tail version duplicates prefixes: a full group stores
1+...+16 frame copies across its versions, up to 8.5 times the original payload
per replica before overhead/orphans. Failed proposals can leave additional orphan
objects. No pruning, garbage collection, checkpoint or retention policy is
approved here. Per-boot write bounds and whole-run limits bound new orphans for
qualification; production storage quotas and orphan handling remain separate.

## Append and failures

1. Read the fresh epoch/head and require the exact caller-expected head.
2. Load every required pack from a valid copy; verify the full chain/limits and
   check the fresh head again. Require the loaded history to match the original
   expected head **before any writes**, even if a competing same-epoch writer
   accepted the same frame. One lost copy can be read from the other.
3. Write/read back the complete new tail in both stores.
4. Independently GET **both copies of every old required pack**, including the
   previous partial tail. Repair missing/corrupt copies with exact readback.
   An earlier load is never reused as proof of later repair availability.
5. CAS the same independent witness, then verify the fresh accepted head before
   the journal publishes a view or receipt.

Unavailable history, failed repair/readback, stale epoch/head and bounds refuse
acceptance. Orphans do not become history merely because both objects exist.
Lost CAS acknowledgement or post-CAS check failure is uncertain: poison the
journal; reopen against the current witness and reconcile the **original ID**.
Do not resend the mutation or roll the witness back. The existing independent
read-publication/authority rules from [ADR 0025](0025-concurrent-private-reads.md)
continue to apply.

Replica identity prevents accidental same-port selection; two directories do
not establish independent failure domains. A hostile host can deny storage;
successful repair is an observed acceptance-boundary condition, not a guarantee
against later simultaneous deletion. These limitations also apply to legacy
replication.

S3 PUT is exact replacement, rather than create-only: a corrupt object at a
content-addressed key must be repairable. Canonical partition and original frame
hashes determine the unique correct bytes. New and old objects independently
read back before CAS; an untrusted overwrite cannot establish acceptance. The
S3 identity includes the bucket and region, so selecting the same bucket under
two ports fails the distinct-replica check; separate buckets alone do not prove
separate administration or availability.

## Explicit archive and migration

Both backends share the byte-identical full `CINDER-SNAPSHOT-1\0` archive codec.
The shared parser also corrects an inherited one-byte header-bound error (117
instead of the actual 118 bytes), deriving it from the unchanged magic/fields.
No per-frame/count/history bound or cryptographic validation is relaxed.
Import validates every frame/context/chain/limit, requires the independently
current **exact** head and epoch, writes/repairs both copies and rechecks authority.
It never advances/initializes/resets the witness. Journal::open must then AEAD
authenticate and replay the full archive with its original configuration and key.

A synthetic migration test fences the old writer using a new trusted epoch,
imports the archive and retains every original receipt. That is not a deployed
migration runbook. For a future deployed migration: fence every old writer/capability,
bind format and pack policy to the governed application/recovery manifest, qualify cloud
ports, archive access, interruption/restore, resources and changed measurements.
Never infer permission from an old C5 artifact. The local fixture's durable
layout marker refuses a format change on reopen rather than attempting migration.

The version-3 manifest uses `CINDER-RUNTIME-MANIFEST-3` and `CKR3` role envelopes;
every history field changes the digest and loaded application commitment. The
six actual key roles, chain contract and resource policy must agree before
release. Versions 1/2 reject a pack policy; version 3 requires it and the chain
contract. It still rejects trading and funding activation. A new public manifest
changes PCRs and requires fresh approval; a source/ELF build is not that approval.

## Qualification and remaining gate

Owning tests: journal `packed_tests` (canonical format, full old-copy repair,
loss during new writes, failure/orphans, bounds, witness uncertainty), integration
`tests/packed.rs` (legacy archive, real AEAD, epoch fencing, file reopen/original
receipt, bounded file port), and Runtime qualification tests (both layouts,
same-owner zero-credit replay, wrong-layout/owner refusal).

`node tools/web-channel/check-runtime.mjs --packed-only` joins actual Node/Chrome
WASM clients, HTTP/WebSocket, real Runtime/API/AEAD/FileStore and disposable
loopback processes. It grows with unique READ grants to head 64 under explicit
100-ms replica delay, checks independent reads at 2/8/32/64, two watches,
50-ms witness delay at the lost-reply cut, original-ID/digest replay after restart
and current-epoch revocation. It does not raise the 15-second reply deadline,
120-second fixture lease, I/O/session bounds or enable financial activity.
Genesis plus API initialization precede those grants; the shipping funding-bind
commit is absent. This matches history lengths, not C5's exact operation mix.
The owning browser CI matrix invokes this in a separate cached, fifteen-minute
job from the unchanged standard/C4 checks. `check.mjs --standard-only` retains
the existing job, `--packed-only` builds its real fixture/WASM/verifier prerequisites
and runs growth; no argument runs both locally. No normal check needs `work/`.

Results and exact artifacts belong in [TRACKER](../implementation/TRACKER.md).
This is local synthetic evidence, not cloud latency calibration, independent
storage/witness infrastructure, Nitro attestation or financial qualification.
Next: review the shipping port/resource contract and propose a fresh measured
hardware run. Full-size bounds and shipping selection tests are local evidence;
actual S3 repair, cloud latency and the new measured image remain unqualified. Hardware
cut 64 and the [financial gates](../implementation/P23-FINANCIAL-GATES.md) remain
open. P23 is not closed by this prototype.
