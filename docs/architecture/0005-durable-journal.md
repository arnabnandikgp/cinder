# ADR 0005 — Atomic private journal and deterministic replay

Date: 2026-09-30. Scope: P05, above P04. Status: implemented for local synthetic
qualification; unmerged until TRACKER records the actual merge. This selects a
local storage mechanism, not production encryption, a financial policy or a venue
authority. [BASELINE](../implementation/BASELINE.md) remains controlling.

## Boundary and decision

`cinder-journal` owns the durable transaction coordinator, bounded canonical event
codec, shared reservation records and SQLite backend. `cinder-kernel` remains
dependency-free and `no_std`. The only kernel additions are read-only exact market
conversion and unresolved-attribution getters. The test-support toy journal is
not promoted into production storage.

Use `rusqlite =0.40.2` with bundled SQLite, default features disabled, and
`sha2 =0.10.9` for deterministic commitments. The 21 transitive registry packages,
versions, enabled features and three build-script exceptions are explicitly
listed in `scripts/dependency-policy.json`; its digest pins the full lockfile,
including registry checksums and edges. Bundled SQLite introduces native C/FFI
outside our Rust `unsafe_code = forbid` boundary; that policy does not audit or
eliminate dependency unsafe code. This is a deliberate storage exception, not
permission for dependencies in the financial kernel.

CI fetches locked dependencies during preparation, then runs Cargo offline. A C
compiler is required for bundled SQLite (Xcode command-line tools on macOS; the
hosted Ubuntu toolchain includes one). No installed database server, wallet,
venue/RPC access or AWS account is needed. Cached dependencies are not a license
to change their versions without updating the policy and reviewing the diff.

## One transaction, one authoritative projection

An immutable `Transaction` contains an independent nonzero `CommitId`, exact
expected `(sequence, opaque hash)` head, injected time, raw/normalized `Input`
records and an all-or-none control subproposal. Raw bodies exclude credentials,
bearer tokens and transport headers. Each input retains configured source scope,
optional causal cut, authority revision and observation time. These fields carry
P13-qualified provenance; constructing them is not authentication.

Replay invokes P04 `Ledger::ingest`, **not** only `apply` or the accepted event
list. Applied, duplicate, conflicting, rejected and unnormalized observations are
retained with their exact bytes and dispositions. The transaction also contains
the deterministic receipt and resulting state commitment. Postings, economic-key
consumption, holds and attempted actions are all derived from this single record;
there is no separately updated balance/dedup/reservation table to tear apart.

Actual facts and proposed controls have deliberately distinct outcomes. Real
adverse facts are still recorded when a hold can no longer cover an action. All
controls then either apply together or return one explicit refusal, without
undoing those facts. A non-applied input cannot accompany a new control mutation.
Neither a duplicate native receipt nor a conflict can release a different hold.
Failed transactions before persistence change no authoritative state.

Unknown raw evidence increments a retained containment count; P05 has no generic
admin reset. P13 owns qualified per-input resolution, keyed by the original
CommitId/input ordinal and exact source/evidence fingerprint. Its qualification
port must establish the complete normalized effects or authenticated absence of
an economic effect; an acknowledgement or a later matching snapshot is not enough.
Resolution and effects must commit once, retain original evidence, reject conflicts
and leave unrelated raw/lifecycle faults blocked. PLAN P13 makes this an explicit
acceptance criterion; P05 cannot clear the gate by fiat.
A reader must independently supply the expected genesis configuration.
Missing, unsupported, mismatched or corrupt history is never replaced by a new
empty customer account. Audit access returns the original private transaction;
it belongs inside the trusted runtime, not a public endpoint.

## Holds and exact attempted actions

P05 reservations are infrastructure commitments, not another set of assets or a
margin engine. They cover exact quote amounts on customer, house and physical
location dimensions. Capacity is intentionally restricted to **flat, settled
cash** or vault liquidity; open positions/unsettled funding are not spendable via
this placeholder rule. Negative customer claims are not recovered cash. Shared
capacity sums all active holds under the same serialized state cut.

Reservation identities remain consumed after release. Customer dimensions must
match the request owner. Authorization, coverage commitments, leverage, complete
freshness checks and risk envelopes remain P07/P09, not inferred from passing the
storage check. This low-level API must not be exposed to customers as an admission
or funds-transfer API.

`Prepare` retains an immutable exact unsigned preimage, attempt ID, parent
request, authority epoch and expiry. For this bounded phase, one reservation may
have **one** prepared attempt: allocating the same hold to a second independently
exposable action is forbidden. `Expose` checks retained evidence, held capacity,
active identity and expiry, then durably records `possibly_exposed = true` before
returning a non-cloneable local delivery. The delivery is not a signature or proof
of external exactly-once execution. A later qualified signer still needs policy,
capability fencing and native action checks.

Only unexposed holds can be released here. Expiry, timeout or restart never
discharges an exposed commitment. P07/P14 must add authoritative resolution and
any safe further attempts. An exact journal transaction retry returns its stored
receipt with **no second delivery**; changed content under the same ID conflicts.
No native callback or signer runs before/during SQLite commit. A crash immediately
after commit may lose the delivery: reconciliation, not automatic resend, is the
required response. Database serialization cannot revoke an already escaped native
capability; P06/P14 own that separate problem.

## Durable backend and failure semantics

SQLite stores only `records(seq, previous, digest, opaque)` and a singleton CAS
head. Immutable-record triggers guard accidental SQL rewrites; they are not
host-security controls. `BEGIN IMMEDIATE` takes the write transaction, compares the
exact head, inserts the opaque body and advances the head in one commit. A
250-millisecond busy wait is bounded; there is no hidden retry loop. Contending
writers return `Stale` or pre-write `Busy`; callers explicitly reload/re-evaluate.
Two requests cannot reserve the same capacity merely because both read an older
state. Reads obtain the head and full ordered history in one read transaction.

Use rollback-journal `DELETE`, `synchronous=EXTRA`, `fullfsync=ON`, in-memory temp
storage, and explicit file/directory sync at creation. New directories/files are
0700/0600 on Unix; existing paths are not overwritten or followed as final-component
symlinks. Use a trusted local directory, not NFS or an adversarial ancestor path.
SQLite's durability still relies on the OS/filesystem/hardware honoring locking
and sync. These settings follow the primary documentation for
[atomic commit](https://www.sqlite.org/atomiccommit.html),
[synchronous EXTRA](https://www.sqlite.org/pragma.html#pragma_synchronous) and
[IMMEDIATE transactions](https://www.sqlite.org/lang_transaction.html).
Killed-process tests are **not** hardware power-loss certification.

Any backend failure other than known pre-write `Busy`/CAS `Stale` poisons the
coordinator. No receipt/delivery is issued and no further commit/state access is
allowed until explicit successful reload/reopen. The outcome may be unknown, not
definitely rolled back: the last commit might exist despite a lost reply. Seal or
preflight errors before append do not publish a record or expose an action.

Bounds: 1 MiB per canonical/protected record, 1,024 entries per encoded collection,
4,096 stored frames, 64 MiB opaque history; 1,024 holds/attempts, 64 KiB attempted
preimage, 4,096 normalized observations and an additional 65,536 audit-cell budget.
Limits compose, so another bound can trigger first. The initial book matrix is
also bounded. Reaching a limit fails closed; it does not prune evidence or make
the journal suitable for indefinite production operation. Compaction/checkpoint
retention must preserve consumption and unresolved capabilities before these
limits are increased or real traffic is admitted. Host denial of service is not
eliminated by bounded application decoding.

## Versions, migration and replay validation

Canonical frames use `CINDER-J\0`, big-endian u16 wire version 1, and a record tag:

| Tag | Meaning |
| --- | --- |
| 1 / 2 / 3 | Complete genesis configuration / kernel event / transaction |
| 4 | Deterministic receipt |
| 5 / 6 / 7 | State / observation / issue commitment components |
| 10 / 11 | Protected genesis / protected committed transaction |

Tags 10/11 also carry financial-engine revision 2. Lengths are bounded u64s;
option tags are exactly 0/1; primitive financial encodings retain P02 versions and
units. Trailing bytes, truncation and unknown revisions reject. Every P03/P04 event
variant, including all native PnL and funding conventions, is encoded losslessly.
An economically invalid but structurally representable event is retained for
deterministic rejection, not silently normalized into another event.

SQLite application ID is `0x43494e44`, current layout version 2. An explicit
`V1ToV2` migration transaction preserves opaque bodies and adds the CAS head plus
immutability guards. V1 is a **synthetic compatibility fixture**, not a claim that
an earlier production journal existed. Unknown schemas reject; failed migration
leaves the old schema/records intact. Engine revision 2 incorporates P03's reviewed
whole-fill reversal allocation; old revision 1 records reject, even if a particular
history would happen to replay identically. This is separate from wire version 1
and SQLite layout 2. No automatic financial migration is supplied and no production
journal is claimed to exist. Disposable fixtures can be recreated; any retained
revision 1 evidence requires an explicitly reviewed migration before reuse. Future
financial semantics must likewise advance the engine revision rather than silently
reinterpreting history.

Opaque frame hashes bind sequence, predecessor, body length and body. Replay checks
the contiguous chain, independently expected configuration, unique transaction
IDs, expected heads, recomputed receipts and derived state commitment. Financial
bindings/funding history are rebuilt through complete observations, not inferred
from a cash snapshot. Tests compare the entire joined `State`, including private
kernel fields, after reopen. Collection lengths delimit commitment sections.

## Privacy and the P06 boundary

The coordinator requires a `Protection` implementation; there is **no production
plaintext fallback or default cipher**. It passes domain, sequence and previous
hash as binding context. Only opaque records reach the SQLite interface. Debug
for private bodies, transactions, attempts, exposures and state is redacted;
errors contain no SQL parameters, raw evidence or secret-bearing paths. Explicit
private accessors are not safe logging APIs, and callers must not format kernel
books/events into host logs.

Test fixtures use a conspicuously insecure reversible transform solely to exercise
the seam. It is **not encryption or confidentiality evidence**. P06 must supply
qualified AEAD, keys, independent authenticated freshness, replication, rollback
detection and writer fencing before deployment with private material. P05's public
hash chain cannot stop a malicious host from replacing a whole valid history.
P06 must also bind key/writer epochs and guarantee per-key nonce uniqueness for
every seal, including losing CAS proposals: sequence alone is not a unique nonce
when multiple writers prepare different records against the same head.
A regression intentionally demonstrates acceptance of an older valid prefix;
that counterexample must be closed by P06, not relabeled a security success.

## Promoted evidence and tests

Reviewed local provenance (not required by fresh-checkout CI):

| Source under ignored `work/` | SHA-256 | Requirement promoted |
| --- | --- | --- |
| `specs/financial-model/IMPLEMENTATION_HANDOFF.md` | `5a36ccf84c6cf8c2a15f6e5de5ad716d74febc36e3f23074e0d0e18c99542b93` | W01 joined atomic postings/consumption/holds and exact attempted action |
| `venues/pacifica/experiments/funding-round-trip/fixture/controller/README.md` | `f52158e0ff2af17db1e04f09048cecb557862e88d9878d80a67917025688d973` | W04 immutable attempts, unknown outcome, no automatic resend |
| `experiments/private-recovery/README.md` | `a126011e384b9eed365aa4f71cf9364dc54d6850b6fb0cf31ad98b251522e6ae` | W05 process loss and valid-history rollback counterexample |

These are mechanism/test promotions, not copied prototype code, live credentials
or claims that M1/M2 were rerun. The DeFi skill informed checked accounting and
adversarial testing without importing any deployment workflow.

Tracked tests in `crates/journal/tests/` exercise all event codecs, bounded mutation
decoding, exact replay, duplicate/conflicting receipts, real concurrent database
connections, customer/house/location double reservation, raw evidence retention,
expiry/unknown exposure, failed migration, corruption, unsupported versions,
bounded locks and actual SQLite page exhaustion. Two parent tests kill ten real
child processes at BeforeBegin, AfterInsert, AfterHead, BeforeCommit and AfterCommit:
one matrix joins a posting, consumption, hold and exposure; the other exposes an
already prepared action. The ignored worker is invoked by these parent tests,
not skipped crash coverage. Test hooks are absent from default builds; no service
or production deployment may enable them.

Evidence level: tested finite traces on the recorded platform, not a mathematical
solvency proof, independent audit, malicious-host guarantee or Nitro qualification.
Default and hook-enabled builds are both linted; debug/release tests use disposable
files and no live transport. Exact verification and PR state live in
[TRACKER](../implementation/TRACKER.md).
