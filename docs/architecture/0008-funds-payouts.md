# ADR 0008 — Partial funds, shared collateral and beneficiary discharge

2026-09-30. Implements P08 in the existing financial ledger and CAS journal.
This is offline accounting/controller implementation, not custody qualification.

## One movement and one customer entitlement

An immutable mandate binds request/attempt, physical source, internal destination
or exact beneficiary, net amount, fee ceiling and fee owner. Authorization moves
no money. Each qualified debit, arrival, return or impairment has its own economic
identity. For cumulative gross debit `D` and gross settled/returned/impaired `S`:

```
transit = max(D - S, 0)
unpaired = max(S - D, 0)
assets = vault + native marked assets + transit - unpaired
```

`unpaired` is a timing contra-balance, never collectible debt or new capital.
An arrival seen before its debit cannot inflate backing. Internal arrival credits
its actual custody location, not another customer deposit. Transfer fees and
impairment belong to house; an external payout can instead have an explicitly
authorized customer fee payer and ceiling. That is a policy input, not a selected
commercial fee schedule. No loss is silently spread across customers.

Only confirmed payment to the authorized beneficiary reduces that customer's
claim and increments their cumulative ordinary paid counter. A source debit is
not a withdrawal completion. The counter excludes fees and excess/wrong-recipient
payments. Those excess payments reduce house equity, not customer collectible
debt. Actual fee overruns, wrong routes and unauthorized partial outcomes remain
accounted and contained; they are not rejected merely for violating admission.
Unrepresentable evidence remains retained as an explicit unresolved kernel result.

## Queue, commitments and finality

The private request digest binds ownership, amount, beneficiary, fee payer, partial
consent, policy, grant epoch and expiry. `Approval` is a trusted authentication
port result, not a signature verifier. P18 must authenticate it. The same journal
atomically accepts the FIFO operation, shared commitments, bounded prepared
action, and possible exposure before returning any dispatch capability.

Payouts reserve the customer's full requested claim and authorized fees at queue
entry. Source liquidity is required at preparation, allowing a backed but illiquid
request to wait. Internal transfers reserve their physical source immediately.
Payout FIFO is per current pool/asset, not per source; unresolved prepared sends
serialize on the same source. Intentional partial dispatch requires consent.
Only one attempt per operation is supported here; later retries require separately
qualified lifecycle semantics. Returns do not reset spent capabilities.

Actual receipts consume only corresponding remaining commitments. Source debit
consumes source hold, beneficiary payment consumes customer hold, actual fees
consume fee hold. No other order's reservation is resized. Final release requires
equal debit/settlement totals, the exact applied receipt set, qualified coverage
in **each source's own causal units**, and no-later-execution evidence. A terminal
certificate may precede its receipts but cannot release early. Contradictions
re-encumber and contain the operation. Neither timeout nor an empty response is
proof that a native capability can no longer execute.

Unexposed cancellation is only a clean local abandonment: a faulted operation,
terminal proof, or any real bound receipt prevents it from releasing the hold or
marking the operation terminal. Contradictory pre-exposure receipts still post
their actual effects; refusal, duplicate delivery and replay preserve containment.

The terminal evidence reference and nonzero authority epoch are **not cryptographic
authentication or native fencing by themselves**. P16 must qualify the retained
evidence before constructing this trusted input. Abstract prepared bytes are not
signed Solana transactions or native venue transfers. P15/P16 provide those rails.
Freeze refuses ordinary exposure while still ingesting actual financial facts and
allowing scoped order cleanup. Recovery mode/authorization is not implemented here.

## Marked collateral prerequisite

An immutable-version policy and qualified fresh mark set install against an exact
ledger version. Every later capacity check uses current ledger/holds and rechecks
freshness under the journal CAS. With positive funding receivables excluded:

```
free = cash + min(unsettled funding, 0) + unrealized PnL - initial margin
initial margin = sum ceil(abs(position notional) * market margin bps / 10_000)
```

Customer, house and native dimensions stay separate. Native free capacity is also
capped by native cash. House capacity is capped by backing surplus less nominal
in-transit assets: a transfer receivable is not available first-loss cash. Physical
vault liquidity is another constraint. Open positions therefore do not themselves
prohibit withdrawals, but committed collateral does. Different assets/venues are
not implicitly fungible. Tests use synthetic margins, not calibrated live limits.

Until P09 supplies the outcome evaluator, active pending orders conservatively
refuse this collateral path. No zero-risk assumption for opposing orders, no
user-selected leverage engine, stress-capital claim or production admission is
smuggled into this prerequisite. P09 must replace that refusal with bounded
reachable-outcome evaluation and retain these same reservations and payout rules.

## Verification and provenance

Canonical journal wire revision **3** / financial engine **4** explicitly reject
older semantics. SQLite schema and encrypted-frame formats are unchanged. All
movements, queue records, paid totals, proof cuts, flags, collateral policy and
remaining commitments contribute to replay and state commitments.

The tracked 17-group funds suite covers all 24 four-leg delivery orders with
prefix reconciliation/reload/duplicates; partial arrival/return/impairment; wrong
recipient, overpayment and fee overruns; FIFO/consent; open-position margin and
double holds; illiquid queue/stale marks; fill-versus-exposure; false/missing/early
terminal evidence; freeze and late receipts; changed authorization; and lost
durable replies. The same encrypted/crash and journal checks remain applicable.

Promoted local studies (not live credentials or a second reference ledger):

| Reference | SHA-256 |
| --- | --- |
| `work/specs/financial-model/funds-lifecycle.cjs` | `70a6ead6f288638e27d745d0762dd128cf59686768564679ee15503a4ad9931f` |
| `work/specs/financial-model/funds-lifecycle-checks.cjs` | `476aa072856c02066c6a23118e7066548bf7f0b12fb7ac2fdd4cf47f57d613e6` |
| `work/specs/financial-model/funds-and-capital.md` | `a6e723036a928a9d12dc70ad301390068f2d3939deef210fd961626466655011` |

The flat study's house-fee convention is generalized only to an explicit capped
customer authorization. Final coverage/pricing and native finality remain gates.
