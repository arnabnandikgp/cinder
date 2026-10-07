# Concurrent private reads: bounded design review

Date: 2026-10-06. **Direction approved for bounded local remediation; implementation
and hardware qualification remain incomplete.**
The user authorized the local concurrency diagnosis and this design review.
Published checkpoint: `6dff3731f365463ad24c1b5fcd2cb82f19d76c86`, PR #55.
No new financial policy, persistence format, dependency, deployment or AWS run.
The user then requested a broader independent architecture review. This draft
was a candidate read slice. The user subsequently approved the broad remediation
direction. [The staged remediation plan](../implementation/P23-REMEDIATION.md)
defines the selected local scope and its gates. Approval does not select a new
production storage/retention contract or waive the design conditions below.

## Problem and evidence

Actual AWS confidential HTTP reads, attestation and selected storage/fencing
faults passed. Full Node/Chrome WebSocket agent/subscription/revocation did not.
The fifteen-second reply fix delivered one Chrome grant at 12,933 ms; it did
not fix initial read/subscription refusals. Hardware logs do not identify the
cause of every close.

The test-only `runtime_contention_tests.rs` reproduces a specific mechanism:
with 400-ms synthetic witness-read latency, a real Runtime read holds
`Runtime.active` for 808 ms and a grant for 2,828 ms. A competing initial read
hits the existing 250-ms lock limit; a periodic poll skips without a reply.
Read-on-read blocking therefore exists even without a write or S3 access.
Healthy append at N=1/9/25 accepted records performs 5/29/77 replica GETs
(`3N+2`). These are bounded local measurements, not calibrated cloud SLOs.

Relevant source boundaries:

- `service::runtime::handle_available`: shared mutex spans remote checks and API.
- `api::Service::handle`: identity/signature, witnessed freshness, current epoch,
  current grant, then projection or durable mutation. Preserve that authority.
- `journal::Journal::{verified_state,commit,reload}`: accepted state/history and
  uncertainty handling. A `state()` alone is not freshness authority.
- `journal::replicated::Replicated::append`: full-history load, two-copy writes,
  repair verification, witness CAS and post-CAS check. Do not remove these here.
- `service::web::Server::application`: a synchronous periodic poll also occupies
  the same connection's command loop. Removing the runtime mutex alone does not
  establish same-socket command responsiveness.

## Alternatives

| Approach | Benefit | Limitation / disposition |
| --- | --- | --- |
| Longer lock/reply deadlines | Lets some slow work finish | Leaves head-of-line blocking and growing history work; not the scheduling fix |
| Fair bounded single command queue | Explicit admission/order; no spinning mutex wait | Every slow read still delays writes/other reads; keep as a possible writer-admission improvement, not the main solution |
| Replace the active mutex with an RwLock | Allows concurrent readers in principle | Backend reads take mutable access; network work still pins guards, and standard RwLock fairness is not guaranteed; reject as a drop-in fix |
| TTL account cache/shared last-good witness result | Faster replies | Can serve after head/epoch changes or revocation; reject |
| Independent immutable accepted read view, fresh check per reply | No remote I/O under the writer lock for reads; one authoritative journal | Requires publication, release gating, bounded retention and race tests; recommended |

[Rust's RwLock documentation](https://doc.rust-lang.org/std/sync/struct.RwLock.html)
states that lock priority is OS-dependent. No new synchronization dependency or
unsafe lock-free pointer implementation is proposed.

## Recommended contract

One writer remains responsible for all financial transitions, signature exposure,
operation identities, reservations, accepted evidence and recovery. Read views
are immutable derivatives of that SAME accepted journal, not a second database,
editable financial state, persisted cache or spend permit.

Each view binds:

```text
domain + stream + writer epoch + exact accepted head(sequence, hash)
+ local publication generation + governed config/API commitment
+ accepted State + API-history/receipt and book-history projections
```

Writer epoch, customer authorization epoch, policy revision and publication
generation are distinct quantities. Advancing one never substitutes for another.
Raw records, native credentials and signable attempts are not exposed through
the read interface. No global sequence/hash is added to private API replies.

### Publication and uncertainty

The journal, not individual API handlers, owns publication. Otherwise scheduler,
funding, recovery or operator-ingested commits could leave a stale view installed.

1. Boot/reload publishes nothing until authenticated replay, configuration/API
   validation and current writer authority succeed. Rebuild projections only from
   accepted records; do not trust a host-supplied decrypted snapshot.
2. A write computes its candidate privately. It does not publish that candidate
   before durable acceptance and all current commit checks succeed.
3. During pre-acceptance storage I/O, the old immutable view may be used only if a
   new independent witness read still matches its exact head and writer epoch.
   Such a read precedes acceptance economically; it is not a stale-cache exception.
4. A successful commit atomically installs its new accepted projection and
   increments the local generation before its API receipt/capability is released.
   Every commit path must do this, including rejected-but-retained controls.
5. Between remote CAS and local publication, a witness reading the new head must
   NOT validate the old view. Refuse/skip; never synthesize the missing projection.
6. Unknown commit outcome, poisoned journal, replay failure, invariant failure or
   writer panic closes read publication and fences the boot. A post-CAS error
   cannot leave the old published view usable because it happens to be in memory.
7. Missing/corrupt replicas and full-history repair retain existing semantics.
   Read freshness still uses the witness; no per-read promise of both replicas'
   present availability is added. Failed new acceptance cannot publish a candidate.

Publication uses a short, bounded metadata/pointer latch, never a lock held over
network I/O, full projection building, Noise encryption or socket writes.
Accepted state/history ownership must be shared or bounded explicitly; simply
cloning the entire journal for every session is not an acceptable implementation.
`Nsm::now()` itself performs device/attestation-verification work under a mutex.
It must not be called while holding this publication latch. The prototype needs
an explicit protocol for obtaining fresh signed time outside the latch, rejecting
delayed release and checking expiry without substituting host/cached wall time.
This is an unresolved implementation condition, not an approved clock redesign.

### Read preparation and release

Only `Read`, `View` and `Operation` enter this path. Commands, even exact mutation
retries, remain on the authoritative writer path. A subscription cannot disguise
a mutation or gain a different authorization rule.

```text
1. Decode bounded request; verify signature and established-channel binding.
2. Acquire a bounded read slot and capture one immutable publication generation.
   Reject if unavailable/fenced. Never acquire Runtime.active for remote I/O.
3. Validate account/domain/policy against the governed contract. Issue one NEW
   authenticated strong read of this exact witness stream using enclave-held,
   finite witness credentials and the measured TLS/SigV4 route.
4. Require witness writer epoch and full head to equal the captured view.
   No last-good value, eventual read, shared proof, implicit reload or read retry.
5. At a fresh time after I/O, evaluate the EXISTING own-account projection and
   owner/agent authorization on the captured state. Derive valuation at that
   explicit evaluation time; absent/stale qualified inputs remain absent.
6. At the short release latch, require the SAME publication generation/head,
   healthy boot/journal, unexpired boot/session/request/grant and current customer
   epoch. Recheck time-dependent validity if projection work crossed a deadline.
   A changed generation discards the result, including an otherwise valid cursor.
7. Authorize release once; the connection's sole cipher writer encrypts/delivers
   the response. No cached result or witness token is reused for the next poll.
```

The pure API projection/authentication logic must be shared with the writer path,
not reimplemented as looser read permissions. Read source types have no commit,
dispatch, signature-exposure, native account directory or recovery authority.

The independent read witness handle exposes only `read`, not CAS. It targets the
same independently provisioned row/epoch and existing finite secret credential;
it is not a second witness or second writer. Do not serialize all read I/O under
one global witness-client mutex, or the original bottleneck moves rather than
disappears. Use bounded independent clients over enclave-only credential material.
This interface restriction does not narrow the AWS credential's actual IAM
authority. No new root/key/IAM authority is assumed or approved by this design.
Preserve the current STS safety margin, TLS/root/endpoint, stream parsing and
finite timing checks, with explicit post-I/O expiry validation.

[DynamoDB strong-read documentation](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/HowItWorks.ReadConsistency.html)
supports a fresh regional table read with `ConsistentRead=true`; it does not lock
the row against a subsequent change. The local generation gate is therefore
essential, and neither gate claims to prevent a later externally authorized epoch
change. No global-table/index/stream consistency assumption is introduced.

### Linearization and revocation

Durable write acceptance remains the existing witness CAS; successful local
completion/publication follows the post-CAS checks. Successful reads linearize
at their own matching strong witness read, conditional on the final release gate.

- A revoke completed before a read begins must make an old agent unauthorized.
- A revoke published while a read is checking/building makes its captured
  generation stale; it cannot obtain a new release permit.
- A read released before an overlapping revoke may already be in flight. Neither
  the old implementation nor this proposal can recall packets or erase data an
  agent previously obtained. Do not promise instantaneous physical delivery revocation.
- Every new subscription emission receives fresh authentication/witness checks.
  A prepared update cannot sit in a queue and later be delivered under a previous
  release decision: perform the generation/time gate at release from that queue.

An external epoch can advance after a strong read without local knowledge. The
read may be linearized before that concurrent change; subsequent checks must
refuse. Eliminating that distributed race would require a different read-lease
or witness protocol, not an extra local mutex or an undocumented claim.

### Failure classification and carrier behavior

| Condition | Required behavior |
| --- | --- |
| Read slots full or local publication changes during read | Bounded unavailable result; established periodic poll may skip without output. No financial retry, no healthy-writer poisoning merely for normal contention |
| Matching-epoch new head during known local acceptance/publication | No old reply. Treat as a read race only when matched to the writer's exact in-flight/accepted transition; otherwise conservatively fence/reconcile |
| Wrong stream/epoch, missing/malformed witness, transport/credential/clock failure | No reply from cached state; fail closed and preserve sticky fencing where required |
| Old agent epoch, missing/expired READ grant | Existing encrypted authorization error; terminate affected subscription, no polling past revocation |
| Uncertain write or poisoned writer | Close publication/fence; original operation reconciliation after approved reopen, never automatic resend |

Ordinary initial reads must return a bounded outcome, not block forever. A
generation race must not be reported as successful subscription setup. Keep
carrier error/resync semantics explicit; do not silently change the SDK to retry
unknown commands or increase the fifteen-second application deadline.

## WebSocket scheduling and bounds

Cross-session lock removal alone is not the full acceptance target. A synchronous
periodic read can still keep an already-arrived same-socket command waiting.
The proposed implementation must qualify these together:

- Keep one connection-owned Noise state/cipher writer and current ingress bounds.
- Prefer queued ingress before STARTING a new periodic poll; never starve expiry
  or subscription checks under continuous valid traffic.
- If a started slow poll still violates command latency, use at most one bounded
  read-preparation job per connection. That job cannot touch Noise or dispatch a
  command. It returns an un-released candidate; the connection performs the final
  generation/auth/expiry gate immediately before encrypted release.
- Cancel/discard obsolete jobs on close/revoke/fence, with finite I/O termination.
  No detached worker, unbounded queue, notification loss or concurrent cipher use.
- Enforce a runtime-wide read-slot limit shared by the selected ingress profile.
  The shipping binary currently chooses TLS OR web; do not add simultaneous
  listeners just to implement this limit. Choose and measure it in the prototype.
  Existing session/page/frame/byte/time limits are not relaxed.

A job surviving socket close must retain only bounded private state and expire;
do not claim drop/zeroization of all State/Arc/browser/provider copies. This
proposal neither fixes nor weakens existing key-erasure limitations.

## Implementation slices and fixed acceptance matrix

1. **Journal/read API:** private immutable projection source, central publication
   and poison invalidation; shared pure authorization/row generation; per-read
   strong witness port. No backend append or economic/wire-format changes.
2. **Runtime/carrier:** bounded read admission, final release gate, distinct normal
   race versus fatal failures, no writer-mutex I/O for reads; qualify same-socket
   poll/command scheduling. Preserve all actual transport/session deadlines.
3. **Joined local regression:** real signed API/Journal/AEAD/Replicated plus actual
   Node/Chrome HTTP/WS. Inject latency/faults; instrument only aggregate timings,
   lock spans and I/O counts, not payloads/identities/keys.
4. **Explicit storage/lifetime decision:** the existing append cost and retained
   history ceiling already rule out an indefinitely growing low-latency service.
   Either qualify a strictly bounded non-customer workload/lifetime under today's
   full-history repair contract, or review a replacement durable segment/checkpoint
   and retention/failure contract. Batching/parallel I/O/TLS reuse can improve
   constants, not remove the ceiling/asymptotic work. This is not solved by reads.
5. **Hardware:** exact-source ARM package, new measured image and reviewed pins,
   then a fresh bounded manifest/session. Old EIF/PCRs are not new-image evidence.

Fixed local gates before another AWS run:

| Gate | Required cases / failure oracle |
| --- | --- |
| Projection equivalence | All ten families, View/Operation, auth errors, receipts, paging/history equal the existing canonical handler at the same state/time; no other-owner or grant-key leakage |
| Read-on-read | Two independent slow witnesses complete without writer-lock contention; no shared last-good witness or retry |
| Read/write | Read during slow pre-CAS grant plus acceptance-before/after-witness races; fresh matching old head allowed only before acceptance, unpublished/new/changed generation refused |
| Revocation/time | Old agent after revoke; revoke during preparation; expiry during I/O/build/queued delivery; owner reconnect at current epoch; publication poisoning/panic |
| Storage/fencing | Witness loss, wrong stream/head/hash/epoch, STS margin/expiry, lease expiry, replica loss/repair, post-CAS uncertainty, reopen restores only exact accepted state |
| HTTP/WS | Actual Node/Chrome initial read, one/two watches, grant/revoke on another session AND same socket, commands arriving during slow polls; original uncertain mutation reconciled without resend |
| Resource/latency | Fixed read/connection/job limits across listeners; bounded full request and memory/retained generations at declared history sizes; continuous traffic and slow consumer; unchanged application deadline |

Memory bounds include writer candidate, current published state, historical
records/indexes, in-flight old generations and response buffers together. Share
immutable accepted records rather than duplicating raw history per generation.
If the declared worst case does not fit the chosen enclave budget, reduce the
qualified workload/admission bounds; never evict accepted evidence or block a
critical writer indefinitely waiting for a stalled reader.

## Review disposition and remaining decision

The independent Astra review supports this direction as a **read-serving slice**,
not a complete architecture correction. Implementation must resolve
fresh NSM time outside the release latch, exact known-writer head metadata, bounds
on retained State/index generations, and a starvation test under unrelated native
commits. Global generation invalidation is conservative but can repeatedly refuse
reads under continuous writes; do not weaken it to account-only freshness quietly.
Also account for repeated API/controller history interpretation, per-call cloud
TLS and background tick I/O in the whole workload budget.

No design interleaving model was implemented/run: work paused for the broader
architecture review before that experiment. The existing two real-runtime
contention diagnostics are distinct evidence. A future abstract model would not
be implementation, AWS, audit, SDK or P23 closure evidence.

2026-10-07 implementation disposition: C1 shares accepted interpretation and pure
authorization; C2 supplies centrally published, individually witnessed bounded
read tickets. C3 retains those tickets through command/periodic preparation and
consumes the release gate on the connection's sole cipher owner. A one-slot wake
signal joins bounded ingress and a scoped periodic worker without blocking command
delivery on poll I/O. Ordinary native observation/RPC requests rejoin the writer
after I/O with exact reserved identity and original evidence time. Inactive tick
checks signed lease/sticky fencing, not proactive cloud-history freshness; actual
operations retain new witness checks. Trading/funding activation remains disabled.
Local targeted races/scheduling tests pass, but actual joined-client, sustained
write and memory qualification remain C4; fresh changed-image hardware remains C5.
These implementation notes do not revise the authority contract or claim those
remaining conditions have been satisfied.

The user approved the read/publication authority split and its release/revocation
semantics as part of the bounded remediation direction. Each implementation step
must satisfy the conditions above; approval is not evidence that it does. Numerical
read/memory budgets are local qualification parameters, not customer-policy
approval. A production segment/checkpoint/retention contract remains a separate
review. All native/chain/financial qualification and G01–G04 gates remain unchanged.
