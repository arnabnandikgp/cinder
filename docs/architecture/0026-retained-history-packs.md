# Retained encrypted history packs — local prototype

Date: 2026-10-07. **Local design/prototype approved; not selected by the shipping
Nitro runtime, not a cloud migration or new hardware qualification.**
The [growth diagnosis](../implementation/P23-GROWTH-DIAGNOSIS.md) established
history-dependent object calls. This candidate reduces those calls without
removing records or the later historical-copy repair boundary.

## Format and authority

`journal::packed::Packed` implements the existing Backend. No new dependency,
financial equation, API wire, cipher, timeout or witness schema is introduced.
The new module is compiled normally, but **only the explicit local-fixture
Runtime can select it**. `Loaded::open` and the shipping S3 adapter still select
legacy per-frame Replicated storage. There is no fallback between formats.

- Fixed canonical groups contain sequences `[16k, min(16k+15, head)]`.
  Sixteen is a prototype format parameter, not an approved capacity limit.
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

Original bounds remain: 4,096 frames, 64 MiB total opaque history, 1 MiB per
opaque frame. A pack is at most `MAGIC.len()+100+16*(MAX_RECORD+80)` bytes,
approximately **16 MiB**, including headers. The local FileStore checks size
before reading, bounds the read itself, rejects direct symlink objects and uses
private directories, exclusive temporary files, fsync/rename/readback.

**Shipping S3 currently accepts only a frame-sized response. It cannot be used
as this pack port.** Promotion needs a pack-specific bounded transport, reviewed
allocation/copy/RSS and timeout/credential/budget behavior. Do not enlarge all
cloud responses or claim that small-record fixture RSS qualifies maximum-size
packs. The prototype holds complete history and temporary pack copies in memory;
this is deliberately not a streaming production-capacity implementation.

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
approved here; storage quota/orphan handling is a promotion gate.

## Append and failures

1. Read the fresh epoch/head and require the exact caller-expected head.
2. Load every required pack from a valid copy; verify the full chain/limits and
   check the fresh head again. One lost copy can be read from the other.
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
migration runbook. Before shipping: fence every old writer/capability, bind format
and pack policy to the governed application/recovery manifest, qualify cloud
ports, archive access, interruption/restore, resources and changed measurements.
Never infer permission from an old C5 artifact. The local fixture's durable
layout marker refuses a format change on reopen rather than attempting migration.

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
Next: review this candidate's format/resource/migration contract; only then
promote bounded cloud ports and propose a fresh measured hardware run. Hardware
cut 64 and the [financial gates](../implementation/P23-FINANCIAL-GATES.md) remain
open. P23 is not closed by this prototype.
