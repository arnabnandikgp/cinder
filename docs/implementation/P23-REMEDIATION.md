# P23 runtime remediation

2026-10-06. The user approved the broad architectural direction after the local
contention diagnosis and one independent read-only Astra review. This is a
bounded continuation of P23 on PR #55, not a new financial/account architecture,
permission for another deployment, or a claim that P23 is complete.

## Work and acceptance gates

| Slice | Progress | Deliverable / completion condition |
| --- | --- | --- |
| C1 — Shared interpretation | in progress | Implemented locally; final checks/publication receipt below. Replay-derived indexes and pure authorization preserve canonical responses, error ordering, epochs and grants. Warm indexes never replace a fresh check; review/merge remain open. |
| C2 — Accepted read publication | open | Central journal publication and independently witnessed immutable reads pass acceptance/revocation/uncertainty races. No remote I/O or NSM attestation under a publication latch; exact known-writer metadata distinguishes races from faults. |
| C3 — Scheduling and external I/O | open | One mutation owner remains authoritative; bounded prepare/I/O/completion work and connection-owned cipher/reply delivery avoid read and same-socket head-of-line blocking without weakening final release checks. |
| C4 — Joined local qualification | open | Fixed real-runtime and actual Node/Chrome workloads pass with measured dependency, latency and memory bounds, including retained generations and continuous unrelated writes. |
| C5 — Changed-source qualification | open | Exact-source ARM/EIF/client receipts and a separately authorized fresh hardware manifest qualify the changed application. Native, chain and financial capabilities remain independent gates. |

These slices stay on the existing shallow P23 branch until a justified review
boundary is needed; do not create a PR per helper or mark a slice closed solely
because a benchmark becomes faster. Keep exact results and next actions in
[TRACKER](TRACKER.md). The governing authority/race contract and full negative
matrix are in [ADR 0025](../architecture/0025-concurrent-private-reads.md).

## C1: first implementation boundary

- Pair accepted transactions with their retained receipts directly. Avoid a
  separate linear receipt lookup for every transaction in API, funds-controller
  and recovery history interpretation. Their acceptance/causal-cut rules stay
  unchanged.
- Build an immutable API record index bound to the exact opaque journal head and
  governed API-contract fingerprint. It is derived data, never a separate ledger,
  persistence format, independent grant store or cached permission to spend.
- Reuse that index only after the normal fresh journal check and current state
  authorization. A poisoned journal or malformed new history must not fall back
  to a previous index. A head change, including non-API commits, rebuilds it.
- Share envelope, signature, authority-epoch and agent checks between the current
  handler and the future read path. Preserve authorization before ID lookup and
  existing account-scoped grant semantics.
- Test warm/cold equivalence, all read families, receipts, replay, expiry,
  revocation, account-scoped IDs, malformed history and unknown acceptance.

C1 deliberately leaves Runtime locking, witness checks, NSM time, external I/O,
full-history replica repair and per-connection polling unchanged. It is a useful
foundation for C2/C3, **not** concurrent-service qualification.

### C1 implementation and test handoff

`journal::transactions_with_receipts` traverses each accepted transaction with
its retained outcome directly. API initialization/interpretation, funds history
and recovery custody-cut validation use it; single-ID lookups remain unchanged.
`api::records` builds one immutable ordered index plus account-scoped operation
and epoch-scoped first-accepted-grant lookup maps. The Service retains only the
latest exact-head/contract-fingerprint index. Build and final deallocation happen
outside its short pointer latch. No new persistence, wire or dependency is added.

`api::authorization` extracts the existing envelope/signature, logical-time/epoch
and agent checks as pure functions. Signature checks precede witnessed freshness;
current time/epoch precede index interpretation; agent authorization precedes
private operation ID lookup. This is unchanged handler ordering, not a separate
authorization policy or grant database.

Six new real-AEAD/SQLite unit tests cover index reuse with a fresh check on every
request, non-API head/config changes, warm expiry/revocation/invalid signature,
unknown accepted append and exact replay, malformed new history, account-scoped
IDs and poisoned index latches. A seventh integration regression compares all
ten canonical read families (including continuation pages), View and Operation
bytes with a cold interpreter and after replay. These are offline tests, not
independent witness/hardware evidence.

The next slice is C2. Start by abstracting the existing pure read projection over
the accepted source, then join central journal publication/invalidations to it;
do not add a second copy of auth or read schemas. Preserve shared accepted-history
references and qualify old-generation memory before Runtime read jobs use them.
Build exact writer-phase metadata and race tests before enabling independent
fresh reads. C3 owns the later same-connection polling/cipher delivery integration.
The exact commands/results and published/local distinction live in TRACKER.

## C2/C3 implementation constraints

Use the same authoritative journal for all API, scheduler, funding and recovery
commits. Publish only accepted projections; close publication after unknown
acceptance, poisoning, failed replay or writer panic. Independent read jobs must
obtain their own authenticated witness evidence, then revalidate publication,
current authorization and qualified expiry before release. No shared last-good
witness, TTL, automatic retry, account-only freshness or longer reply deadline.

Resolve fresh NSM time outside the short release latch and explicitly bound work
between time qualification and release. A connection owns its Noise state and
writer; worker jobs do not concurrently mutate that cipher. The executable selects
TLS or web ingress, so a second simultaneous listener is not required. Budget
read jobs, connection limits, writer candidates, index/history references, old
published generations and response buffers together.

## Fixed local qualification envelope

Start with accepted-history sizes 1, 8, 32 and 64; witness delays 0, 50 and 400 ms;
one/two watches; two simultaneous reads; and one concurrent mutation/controller
tick. These are test inputs, not shipping customer limits or calibrated cloud SLOs.
Add the exact boundary/fault cases in ADR 0025: post-CAS/pre-publication,
revocation/expiry during preparation and queued delivery, witness/fencing/STS
failure, slow consumers and sustained unrelated commits. No test silently
retries an uncertain mutation. Use actual Node and Chrome clients, not just direct
handler calls, before claiming the transport gap is fixed.

Record total work, latency and peak live memory for this declared envelope. If it
cannot fit the chosen enclave budget, constrain the qualified workload or stop
for a new design decision; do not discard accepted evidence or weaken checks.

## Storage decision and scope

For this bounded dev/test remediation, preserve the existing replicated append,
full-history repair/freshness checks and 4,096-record/64-MiB limits. Their quadratic
cumulative I/O and finite lifetime remain explicit limitations, not production
acceptance. The archive snapshot is not compaction. No segment/checkpoint/retention
replacement is approved here; its rollback, repair, proof retention and recovery
contract needs a separate security review before implementation.

No new dependency, crypto protocol, financial policy, custody route, funds test,
AWS session or automatic merge follows from this plan.
