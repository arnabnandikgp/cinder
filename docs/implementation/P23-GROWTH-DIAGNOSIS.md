# P23 history-growth diagnosis

Updated 2026-10-07. Scope: offline diagnosis, not a storage migration or a new
AWS authorization. Parent source: `eac1f57c16367d97ad5f32931880c979de522227`
(application `d2bf4b69968def0fd10ab8b70a056c1aea71f2f3`). The read-only C5
invocation is closed. Its 64-record qualification remains unpassed.

## Finding and evidence limits

The replicated append path re-reads the entire accepted history, then checks
both replicas of every historical frame before accepting the next frame. On
healthy storage this costs **3N + 2 sequential GETs and two PUTs per append**,
where N is the number of accepted frames, including genesis. Aggregate I/O for
successive appends is quadratic. Each shipping cloud call establishes its own
TLS connection and sends `Connection: close`.

This is a confirmed structural cost, not evidence that every unavailable SDK
request has this cause. C5 did not capture per-call/stage latency for the failed
growth request. It established an unavailable reply at head 50 followed by an
original-ID COMPLETE receipt at head 51. The original grant was not resent;
encrypted restart preserved all 49 API operations and account epoch 3.

The SDK's generic exception covers transport/decryption/receipt-validation
failure. An encoded API error is returned as an error response instead. Thus the
exception alone cannot identify an SDK timer, relay timer, failed cloud call or
bad response. Do not turn the local result below into a measured AWS root cause.

## Reproduced mechanism

`runtime::contention_tests::replica_latency_can_outlive_reply_budget_while_original_grant_is_durable`
uses the real Runtime, signed API requests, encrypted Journal and Replicated
backend, with synthetic replica/witness ports and a fixed qualified fixture
clock. No AWS credentials, network, economic actions or live authority are used.

1. Genesis/API initialization and 49 unique READ-agent grants reach head 50.
   This fixture omits the shipping funding-bind commit; it matches the tested
   history length, not C5's exact initialization/operation mix.
2. Only replica GET/PUT latency is set to 100 ms; witness latency is zero.
3. The next grant makes 155 GETs, two PUTs, seven witness reads and one CAS.
   The first debug run took **16,281 ms**. Its replica-delay lower bound alone
   is 15,700 ms, exceeding the current 15-second application reply budget.
4. The test models a missing late reply by performing only original-ID lookup,
   never reissuing the grant. The COMPLETE receipt matches, head 51 stays
   unchanged, and explicit encrypted replay preserves all 51 transactions. A
   fresh API instance rebuilds the same receipt from replay, without its old
   record cache. The financial ledger is unchanged; no orders/attempts exist.

This is a Runtime-level counterexample to a bounded reply guarantee under the
current I/O design. It is **not** an actual SDK timeout, HTTP/WS result, fresh
process restart, AWS latency calibration or a successful hardware cut-64 test.

The expanded existing count regression checks these exact healthy paths:

| Existing head | Accepted frames N | Replica GETs | Replica PUTs | Replica-delay lower bound at 100 ms/call |
| --- | --- | --- | --- | --- |
| 0 | 1 | 5 | 2 | 0.7 s |
| 8 | 9 | 29 | 2 | 3.1 s |
| 24 | 25 | 77 | 2 | 7.9 s |
| 32 | 33 | 101 | 2 | 10.3 s |
| 50 | 51 | 155 | 2 | 15.7 s |
| 64 | 65 | 197 | 2 | 19.9 s |

These are synthetic sensitivity values, not recommended capacity limits.
Backend-only append also makes five witness reads and one CAS; the joined API
path adds freshness checks. A head sequence is not a user/order count or the
frame count. The 64 sample is not the configured maximum: storage bounds remain
4,096 frames / 64 MiB of opaque history.

## Where the work occurs

| Stage | Owning code | Healthy cost / purpose |
| --- | --- | --- |
| Envelope/authority/current state | `crates/api/src/lib.rs`, `crates/service/src/runtime.rs` | Validate ownership/epoch and current journal before mutation; not a reply-latency guarantee |
| Full accepted-chain load | `Replicated::load`, `crates/journal/src/replicated.rs` | N GETs, cumulative bounds, exact backward hash/sequence chain and fresh anchors |
| New frame replication | `Replicated::copy_both` | Two PUTs plus two exact GET readbacks |
| Historical repair | `Replicated::ensure_both` for every frame | 2N GETs; missing/corrupt copy requires PUT + exact GET or refusal |
| Durable acceptance | `Witness::accept`, post-CAS anchor | Exact expected head/epoch CAS; uncertain post-CAS error requires explicit reconciliation |
| Accepted publication / response | `Journal::commit`, API receipt, carrier | Publish only accepted state; a lost response does not undo acceptance |

Independent private reads avoid historical replica GETs, but rely on a fresh
witness and accepted publication. That C1–C4 remediation is still useful. It does
not make the writer's historical replication work constant. CPU/AEAD/NSM and
transport overhead can add latency; none was isolated in C5's failing request.

## Review gate: do not bypass the storage contract

[ADR 0025](../architecture/0025-concurrent-private-reads.md) and
[C1–C5](P23-REMEDIATION.md) deliberately retain full-history repair before new
acceptance. Reusing an earlier GET to skip a later copy check changes the
observation/repair boundary: a copy lost in between would no longer be repaired
by that append. Cached ciphertext or a healthy old witness does not establish
that both copies still exist. Simply deleting the duplicate reads is therefore
not treated as a proven safe fix here.

| Approach | Assessment |
| --- | --- |
| Longer timeout, automatic retry, a special head-64 case | Reject. Does not remove growth; retries can duplicate accepted actions. |
| Skip old checks or rely on an in-memory snapshot | Requires a different reviewed durability/repair contract; not an incidental optimization. |
| Connection reuse / bounded parallel I/O | Possible performance work, but retains history-dependent calls/bytes; requires concurrency, finite-credential, failure and budget qualification. Not sufficient evidence of scalable persistence. |
| **Byte-bounded encrypted history packs, retaining all frames** | Recommended next local design/prototype. Reduce object round trips while keeping complete-chain verification, both-copy repair, original receipts and independent witness acceptance. New format/manifest/recovery binding requires review before shipping. |
| Checkpoints, compaction or pruning | Separate later design. Not needed for the first pack prototype; do not remove consumed-event keys, attempts, recovery or receipt provenance. |

The recommended prototype must specify pack sizing and total-byte bounds,
content/hash/stream/head binding, immutable active-pack versions, archive and
migration behavior. It must show exactly when every required copy is verified
or repaired. A pack is not accepted merely because an S3 object exists. Retaining
full history still has byte-growth costs; this proposal is not a production
capacity or durability guarantee.

**No production storage selection, format, timeout, retention policy or financial
gate changed in this diagnosis.** The user subsequently approved the local
retained-history pack design/prototype. [ADR 0026](../architecture/0026-retained-history-packs.md)
defines its candidate format, safety/bounds, explicit archive migration and
actual-client tests. This approval does not select shipping storage or authorize
another AWS run. Review the prototype's results before cloud promotion.

## Fixed follow-up acceptance criteria

After approval, keep one focused storage follow-up; do not enlarge #55:

- Verify the new layout against the same accepted chain/configuration and exact
  original IDs/digests, including unaccepted/orphan packs and interrupted writes.
- Exercise loss/corruption of either historical copy, failed repair, cumulative
  limits, stale/changed witness, post-CAS unknown result and encrypted replay.
- Measure sequential remote round trips, bytes and bounded memory at the existing
  2/8/32/64 initialized cuts with explicit replica **and** witness latency.
  Count guards describe today's baseline, not a desired future performance floor.
- Use the actual Node/Chrome HTTP/WS clients to test grant/revoke, independent
  reads and lost-reply original lookup; no resend or relaxed auth/deadlines.
- Only then propose a fresh exact-source bounded hardware requalification of
  changed storage plus the previously unpassed cut. No reuse of closed C5.

Funding/trading/recovery remain the separate gates in
[the financial continuation map](P23-FINANCIAL-GATES.md). Passing this diagnosis
does not enable them or close P23.
