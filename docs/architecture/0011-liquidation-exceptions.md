# ADR 0011 — Bounded liquidation and late-close exceptions

2026-09-30. P11 extends the existing order lifecycle, reservations and ledger. It
does not deploy a liquidation bot, calibrate market depth, or grant speculative
house trading. Customer liquidation, native forced events and pool distress remain
distinct; RF1/RF2 native-event allocation belongs to P12.

## Admission and authority

An explicitly installed, immutable-version liquidation policy provides per-market
depth, price, fee and additional house-support bounds, administrator epoch and
qualification expiry. Missing markets and expired depth policy cannot authorize
new orders. These are trusted qualification inputs; tests use synthetic numbers,
not current venue guarantees.

An operator liquidation proposal binds the exact financial version, policy,
market, request/attempt, epoch and dispatch expiry to an authentication-port digest.
The journal CAS additionally covers holds/lifecycle. This is not signature
verification and cannot be exposed as a caller-supplied public authentication API.

The customer must currently breach private maintenance. Size is deterministic:
`min(abs(current position), qualified maximum lots)`, in the closing direction,
with bounded IOC semantics. Other unresolved orders for that customer/market must
first complete or be locally abandoned before exposure. A later collateral top-up
that restores maintenance blocks dispatch of a stale prepared liquidation.
This path cannot liquidate a healthy customer merely to repair the pool's net book.

An exceptional house unwind must reduce existing house inventory in a market with
a recorded close exception. It cannot open a new house position as an admitted
action. The request account identifies the incident, not a customer charged for
the unwind. A partial/failed unwind retains residual exposure and obligations.

The existing P07 customer-authorized private reduce-only path automatically gains
funded exception handling when this policy is installed. It retains ordinary
customer authorization/admission. Private reduce-only is **not** a native pooled
reduce-only instruction: the pooled position can increase while that user closes.

## One funded resource envelope

For admitted lots `q`, qualified range `[p_min,p_max]`, quote conversion `v`, private
maximum leverage `L`, fee ceiling `f` and additional stress allowance `a`, reserve
at least the following house resource (each rational requirement rounds upward):

```
house support = ceil(v(q,p_max) / L)
              + ceil(v(q,p_max-p_min)) + q * (2*f + a)
```

This intentionally covers a full-sized exception, not only the expected late
fraction. It is a conservative bounded model, not an unlimited price-gap guarantee.
The same shared house hold competes with existing payouts, coverage and other
operations; it is not a fresh fund for every request. Ordinary closes increase the
existing hold to the derived minimum before preparation. Emergency IOC preparation
creates that hold atomically before any exposure.

The joined P09 pending-order interval still constrains native margin and costs.
House unwind quantities are excluded from the incident customer's private risk
interval, but not from the native interval. Emergency admission can tolerate the
affected user's private initial/maintenance breach; it does not waive native
margin, capital, solvency, concentration, source qualification or liquidity limits.
Thus this bounded controller may refuse an exit during a larger pool crisis or
insufficient funded support. Broader recovery authority is not silently invented.

Every configured stress prefix is checked; liquidation/close transitions also
replay on the same noncommitted scenario ledger. Scenario routes authorize no real
dispatch. Final-proposal checks revalidate authority and policy after **all**
controls, preventing exposure followed by revocation/downgrade in the same commit.
Ordinary risk cannot piggyback on the emergency exception.

## Actual execution and split ownership

The journal retains the original observation and its deterministic classification.
Each native fill posts once; duplication reuses the original classification.
For a qualified close fill, the customer receives at most the minimum of:

- Actual fill magnitude.
- Original still-unexecuted closing intent.
- The customer's currently open quantity in the closing direction.

The remaining signed quantity belongs to the exceptional house book, never a
silent customer reversal. A wrong-side, out-of-price/fee, or contradicted-terminal
execution remains recorded. Unqualified customer attribution is provisionally
house-owned and contained, not a final adjudication erasing an owed user remedy.
Native reported-PnL differences still enter the existing named suspense issue.

The whole native fill value must be exact under the market conversion. Split that
value between customer and house with explicit toward-zero allocation and retained
complementary residue; do not demand that artificial child fills are independently
exact. Fees/rebates are similarly split once, with the remainder assigned to house.
Native quantity/cash/basis still reconcile with all private books after every leg.

Realized house gains/losses change nominal protection designation when active.
Cumulative positive close costs never decrease after a favorable fill. A budget
overrun, excess, or conflicting lifecycle contains ordinary new risk; actual fills
still post if they exceed the reserve or arrive after dispatch expiry. Funds remain
owed even if all modeled bounds were breached. P10 handles separately qualified
deficits/remediation; there is no automatic blanket insurance award.

## Completion, cancellation and abandonment

ACK, cancel ACK, timeout and desired filled quantity are not finality. The existing
exact fill-set/quantity/source-cut terminal witness governs release. Residual
customer/house positions remain real even after the order itself completes.
Scoped cancel remains available during known close containment under the correct
customer/operator epoch. No cancel-all capability is created.

A never-exposed order may be explicitly abandoned only if it has no possible
exposure, execution or status evidence. Abandonment is its own local lifecycle bit,
not a fabricated venue certificate. It releases its hold and prevents later
dispatch; an exposed/unknown action cannot use this route. Contradictory late facts
re-encumber and contain it. This also fixes otherwise stranded prepared P07 orders
after expiry/revocation or a healed liquidation trigger.

Known close exceptions permit bounded house cleanup, not general trading. Ordinary
containment is not automatically cleared just because the house becomes flat.
Conflicting raw evidence, unknown source effects and missing native reconciliation
still block cleanup that depends on them. An explicit reconciled incident-resume
policy is a later operational qualification, not an unrestricted clear-flags switch.

## Verification and provenance

Wire **6**, engine **7** reject prior replay semantics. The tracked close suite
covers 432 long/short partial allocation cases, fractional child-value residue,
partial IOC/restart/timeout, healthy-customer refusal, healed-trigger dispatch,
funded exceptional unwind, uncapped actual costs, global holds, policy revocation,
safe abandonment, scoped cancellation and mixed-proposal bypass attempts.

W01/W03/W09 inputs: the approved F11/F14 baseline, `protected-lifecycle.cjs` and its
checks (hashes in [ADR 0010](0010-protection-claims.md)), plus the customer-maintenance
rule in `risk-policy.cjs` (hash in [ADR 0009](0009-joined-risk.md)). The study's
synthetic close budget and per-lot shortcuts do not become live calibration.
The risk-admission extension is limited to funded bounded exits, not a promise of
execution during insolvency, unavailable liquidity or unqualified venue history.
No network, wallet, deployment or live market is needed for these tests.
