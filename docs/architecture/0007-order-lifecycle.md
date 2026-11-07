# ADR 0007 — Bound intents, qualified completion and shared commitments

2026-09-30. Implements the order-controller portion of X01–X04 in the existing
journal. This is offline implementation, not native execution qualification.

## One transaction boundary

`State` owns the existing ledger, multi-resource holds and exact attempts, plus
order lifecycle metadata. Metadata is not an asset or another position book.
Authorized acceptance, shared holds, allocation binding, exact attempted action
and possible exposure commit through the same CAS journal. Nothing is delivered
before durable acceptance. A lost commit reply poisons the handle; replay and an
identical transaction retry return no second exposure. No order-ID reuse, changed
instruction under the same ID, automatic resend, or arbitrary cancel-all path.

An immutable intent binds deployment/private account/request, market and units,
signed size, minimum/maximum price, per-lot fee ceiling, GTC/ALO/IOC semantics,
private reduce-only intent, economic policy, grant epoch and dispatch expiry.
`Approval` is the trusted authentication port's result binding its complete
domain-separated canonical digest and owner/epoch. Constructing this Rust type
does **not** authenticate a customer; it is not exposed as an API credential.
P18 must verify signatures/grants before constructing controls. The trusted
authority controller alone advances monotone epochs. This prevents later local
exposure with a revoked epoch, not an already escaped venue credential or order.

Prepared order/cancel bytes are canonical **abstract actions**, not fabricated
Pacifica signatures. P14 must bind its exact native encoding and signing attempt
to them before exposure. Generic P05 actions cannot replace a recorded order's
prepared action. An attributed cancel is distinct from the original place attempt;
it can run during evidence containment without authorizing new financial exposure.
One place and one cancel attempt per request are supported here. Further native
retries need P14's qualified reconciliation, not a timeout-based loop.

## Evidence is not a control

Economic observations post before the all-or-none control subproposal. Valid
external facts survive failed controls. Status observations retain raw evidence
in encrypted transactions, but ACK/cancel ACK/unknown post no fills or fees.
Expiry and unknown results retain commitments. Terminal release requires a
qualified final quantity, exact complete execution-identity set, and source cut;
every identified fill must have posted and its cut must be covered. A terminal
certificate may arrive before its execution records, but cannot release early.

Duplicate semantics exclude receive timestamps and raw transport formatting;
conflicting normalized status or execution identities remain evidence and contain
dependent actions. Financial projection time is monotone journal acceptance time;
the original input receive time is separately retained. An earlier receive time
does not discard a delayed fill after a later internal allocation binding.

An out-of-bound actual fill still posts native exposure/fees to explicit suspense,
not to an unauthorized customer or an invented house trade. Its original and
classified payloads are retained so retries cannot change ownership. A late fill
contradicting a terminal certificate faults the lifecycle and re-encumbers its
hold. If that fill is independently attributable and within the authorized bounds,
it still belongs to its customer; the certificate's error does not erase their
execution. P11 adds funded handling for excess private-close quantity; P07 contains
it and never silently reverses the private position.

## Capacity and causality boundaries

All operation classes use the same `Reservation` API; buy/sell commitments are
not netted. Partial fills reduce remaining order quantity, **not automatically
the collateral hold**. P09 must evaluate open positions and remaining independent
fill outcomes before resizing/admitting risk. The P05 flat-settled capacity guard
remains deliberately conservative here. P08/P09 jointly enable open-position
withdrawals; this phase does not pretend a pending-only hold replaces margin.

The adapter must deliver economic events in qualified causal order across related
orders, reversals and funding boundaries. P13 owns gaps, buffering and native
causal qualification. `source_cut` is not an invented global native sequence and
end-of-pagination is not a completion proof. The 24 permutation test here concerns
three same-side additive partial fills plus a terminal certificate, with duplicates
and real durable reloads. It does not prove arbitrary reversals commute.

Terminal proof, authenticated approval and canonical source ordering remain trusted
ports; unsupported production capabilities stay disabled under G01/G03. No live
venue calls, customer keys, funding or deployed recovery behavior are introduced.

## Encoding, tests and provenance

Financial engine revision **3** and journal canonical wire revision **2** reject
older semantics explicitly. SQLite schema 2 and opaque AEAD/frame formats do not
change. Every new lifecycle/authority/classification field contributes to the
state commitment and reproduces through replay. No implicit history migration.

`crates/journal/tests/orders.rs` covers bound authentication and atomic rollback,
one-shot exposure/lost durable replies, changed IDs, ACK/unknown/expiry, complete
partial histories, all 24 additive delivery permutations with replay, adverse fills,
terminal contradictions, scoped cleanup, epoch revocation, conflicting originals,
and all four independent buy/sell fill/no-fill outcomes. Existing corruption,
concurrency and process-kill suites continue to exercise the same storage boundary.

Promoted W01/W03 cases, not experimental code or live authority:

| Local reference | SHA-256 |
| --- | --- |
| `work/specs/financial-model/execution-lifecycle.cjs` | `dd675f733cfb30746a1a43918c774510e9787ca142257e2d4721405f62fb6bbb` |
| `work/specs/financial-model/execution-lifecycle-checks.cjs` | `24515a1b652d693aea46f5caceba7465a634a5a2331fd75b580adb90839705ac` |

Those studies also contain historical ADL candidates. They are not approval for
profitable-user priority: P12 must implement the approved RF1/RF2 baseline.
