# P23 runtime remediation

2026-10-06. The user approved the broad architectural direction after the local
contention diagnosis and one independent read-only Astra review. This is a
bounded continuation of P23 on PR #55, not a new financial/account architecture,
permission for another deployment, or a claim that P23 is complete.

## Work and acceptance gates

| Slice | Progress | Deliverable / completion condition |
| --- | --- | --- |
| C1 — Shared interpretation | in progress | Implemented locally; final checks/publication receipt below. Replay-derived indexes and pure authorization preserve canonical responses, error ordering, epochs and grants. Warm indexes never replace a fresh check; review/merge remain open. |
| C2 — Accepted read publication | in progress | Implemented locally; verification/handoff below. Central journal publication and independently witnessed immutable reads pass acceptance/revocation/uncertainty races. No remote I/O or NSM attestation under a publication latch; exact known-writer metadata distinguishes races from faults. Review/merge and joined C4/hardware qualification remain open. |
| C3 — Scheduling and external I/O | in progress | Implemented locally; verification/handoff below. One mutation owner remains authoritative; bounded prepare/I/O/completion and connection-owned cipher/reply delivery retain final release checks. Joined C4 qualification and review/merge remain open. |
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

The original C1 handoff was C2: abstract the existing pure read projection over
the accepted source, then join central journal publication/invalidations to it;
do not add a second copy of auth or read schemas. Preserve shared accepted-history
references and qualify old-generation memory before Runtime read jobs use them.
Build exact writer-phase metadata and race tests before enabling independent
fresh reads. C3 owns the later same-connection polling/cipher delivery integration.
The exact commands/results and published/local distinction live in TRACKER.

## C2: accepted publication implementation — 2026-10-07

`journal::read` is a sealed accepted-source interface and bounded publication, not
a second ledger or persisted snapshot. The journal shares accepted State and each
retained transaction/receipt/book-change record through immutable Arcs. Each view
owns a configuration copy and a vector of shared record references. Four global
read tickets bound retained generations; slot admission is not a per-user margin
or account policy. Index/candidate/buffer peak-memory qualification still belongs
to the joined C4 matrix, rather than being inferred from this slot count.

The actual boot attaches one reader only after replay, API/controller validation
and a fresh writer check. Each view binds exact stream/domain, writer epoch, head,
local generation and governed API fingerprint. Every central journal commit,
including rejected-control records, publishes after durable acceptance/postchecks
and before returning its receipt/capabilities. Exact retries do not advance the
generation. An armed writer guard fences on panic/uncertainty; failed freshness,
replay, latch poison, boot fence and journal drop invalidate publication. Reload
does not revive an attached reader; reopen a new qualified boot.

Independent Read/View/Operation requests never acquire Runtime.active. The shared
API signature/envelope rules precede remote I/O. Each ticket issues its own strong
witness read, then uses the same pure authorization and projection as commands.
An exact newer in-flight/accepted head is a normal refused race, not a healthy
writer failure. Unknown heads, backward heads, wrong epochs and witness failure
fence; there is no shared last-good witness, TTL, reload or automatic read retry.
Commands and mutation retries retain the single writer path.

The actual Dynamo read port shares only immutable TLS/SigV4 configuration and
finite credentials through Arc<Client>; each call has its own vsock/TLS exchange.
It exposes no CAS method and adds no IAM authority or new freshness provider.
Cloud completion checks the finite credential again, as well as both existing
10-second time bounds. Request/session/grant/boot expiry uses qualified signed
time, never host wall time. A final 250-ms processing budget starts BEFORE the
last clock/NSM sample; expiry is checked at the signed sample plus that entire
budget. Time/auth/encoding work is outside the publication latch. Release consumes
the ticket only if the same head/generation, healthy publication, actual boot stop
flag and monotonic processing deadline still qualify. Slow final qualification
refuses/skips; this is not a longer application-reply deadline or cloud SLO.

Local tests cover pre-CAS availability, post-CAS/pre-publication refusal, rejected
records/duplicates, uncertain append, write/freshness/witness/clock panics, backward
and unknown heads, missing/wrong-epoch witness, sticky latch poison, boot invalidation,
deadline/expiry during preparation, revocation races and bounded/shared retention.
All ten read families, continuation pages, View and Operation match writer/cold
bytes before and after AEAD replay. Direct Runtime tests show independent witness
calls, no backend reads for the independent path, read service during a slow writer,
parallel read I/O and an owner revoke while an agent read is in flight. These are
real journal/API/runtime code with synthetic local dependencies, not AWS evidence.
Exact complete-run results and source/publication receipt live in TRACKER.

The original C2 handoff was C3, not hardware: separate the synchronous connection's subscription
poll from command delivery, retaining one cipher owner and unreleased candidates
until the connection's final authorization/generation/time gate. Bound queued jobs,
external prepare/I/O/completion and slow consumers; do not queue a response that
already used this synchronous C2 release permit. C4 then qualifies actual clients,
dependency delays, sustained unrelated writes and peak live memory. C5 requires a
fresh exact-source manifest. Storage durability, wire format and financial policy
remain unchanged; the full-history append cost remains a documented limitation.

## C3: connection scheduling and observation I/O — 2026-10-07

`transport::PreparedReply` keeps independent Read/View/Operation candidates
unreleased until the carrier consumes them. Both TLS and web commands perform
their carrier time check before Runtime's final signed-time/auth/generation gate;
web periodic jobs return the same opaque candidate. The Runtime checks the exact
owning boot and connection, not just a matching configuration. No worker owns
Noise state or a socket, and no released read waits in an output queue.

Each web connection has one scoped, joined preparation worker, one job slot and
one completion slot. A coalesced one-slot wake signal carries no authority or
private data. The connection handles at most one already-arrived command before
draining completion and considering another poll. It alone encrypts/writes both
command replies and updates. Only one periodic job can be outstanding; global
four-ticket, eight-connection, frame, application and session limits are unchanged.
Close drops queued work and joins finite in-flight I/O; it does not detach a worker
or claim immediate cancellation/erasure of remote work and all retained copies.
The 500-ms poll interval begins after completion/delivery, including normal skips.

Native observation scheduling now reserves an exact one-use request under the
writer, performs ordinary venue/RPC I/O under a separate supervisor-port owner,
then rejoins the same writer with fresh time/authority. Pacifica completion checks
the original accepted reservation and governed gateway; original receive timestamps
remain evidence while its transaction uses the current logical cut. A late 429
still records the shared cooldown. Unknown exchange spends its original permit
without resend. Original-deposit collection similarly fetches transaction/code/
account evidence without a journal guard; existing recognition rechecks it and
credits once after rejoining. There is no new wire or economic finality source.

When all scheduler gates are false, tick checks the fresh signed lease and sticky
boot/publication fence, without loading cloud history merely to discover no work.
It no longer proactively detects witness/STS loss on an otherwise inactive tick;
every actual read/command performs its own fresh check before private output.
Backend durability/CAS work remains under its one writer. Trading and fund-moving
activation remain rejected by the measured manifest; their existing controller
composition is not claimed as a newly qualified unlocked financial-I/O path.

Local regressions cover queued generation/expiry/revoke, cross-connection/boot
candidate rejection, global ticket exhaustion/release, inactive ticks without
writer/cloud I/O, commands during stalled native I/O, late read completion and
duplicate deposit evidence. A real Rust Noise/socket test stalls a periodic job,
delivers a same-socket command before releasing it, checks cipher/update ordering,
and joins close during I/O. That scheduling test uses a synthetic Handler; actual
Node/Chrome joined to the real Runtime remains C4, not inferred from it.

An unplanned carrier detail was a cumulative five-second frame wait in the first
blocking completion-loop prototype. Bounded wake notification fixes the schedule;
no transport deadline was raised. Strict Clippy also required boxing the owned
deposit-evidence variant; this is private in-memory ownership, not a stored schema.
The exact final checks and local commit receipt belong in TRACKER. Next is the
fixed C4 joined matrix, not a new AWS session or financial activation.

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
