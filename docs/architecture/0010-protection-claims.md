# ADR 0010 — Protection obligations in the authoritative ledger

2026-09-30. P10 adds house-owned protection designation and distinct loss episodes
to the existing ledger. It does not create an insurance token, investor shares,
additional backing assets, or a blanket guarantee of venue outcomes. G02 still
requires approved live coverage and competing-claim priority before activation.

## Recognition, allocation and payment

`Deficit` identifies a qualified, flat, funding-settled negative customer balance.
Recognition creates neither a new customer entitlement nor an asset. Its original
amount, cause and owner remain immutable. Another episode is allowed, but the sum
of remaining episodes cannot exceed the same existing debt. Duplicate causes and
relabelling an already-recognized remainder reject. Collection rights on previously
absorbed debt have **zero backing value** until money actually arrives.

`Remediation` is different: an independently qualified obligation Cinder already
owes. Recognition credits customer cash, debits house cash and designated reserve
once, even if the reserve becomes negative. If that customer already owes an
unabsorbed deficit, the overlapping part offsets it and releases its earmark.
Recognition is a trusted financial input preceding the all-or-none controls;
failed authorization, expired policy or inadequate capital cannot erase the fact.
Qualification of cause/contractual obligation is an authenticated runtime port,
not something inferred from a nonzero evidence hash or a public customer request.

Let `R` be remaining nominal designation and `P` its remaining committed deficit
coverage. A commitment earmarks existing `R`; it changes no cash or external asset.
An approved absorption `x` posts, atomically over the specified claim set:

```
customer cash += x; house cash -= x
R -= x; P -= x; historical absorbed amount += x
```

The existing negative customer balance, designated reserve and house cash must
cover that reclassification. A flat snapshot alone is insufficient: outstanding
orders, funds operations and customer holds must first be resolved. Absorption
does not magically repair an aggregate asset shortfall. Underfunded claims stay
visible; ordinary risk admission and preferential external payouts are restricted.
Actual incoming/outgoing venue facts remain recordable throughout.

The beneficiary's later P08 payout consumes ordinary entitlement and location
cash. It does **not** debit protection a second time. Paid counters continue to
count only actual authorized beneficiary payment, not fees or a nominal award.

## Recovery and repeated episodes

A real incoming receipt first credits actual assets and customer cash through the
existing receipt transition. For a qualified flat debtor, it then:

1. Reduces still-negative, unabsorbed debt and matching claim earmarks.
2. Replenishes the same house resource that absorbed earlier episodes, transferring
   that portion of the receipt from customer cash to house cash/designation.
3. Leaves any excess with the customer.

Historical collection is tracked separately for each episode. Within one debtor
and the same house fund, chronological allocation is deterministic bookkeeping,
not a priority choice between creditors. Recovery never resets cumulative absorbed
capital. A receipt during reopened positions/unsettled funding still records its
actual assets, but leaves a named reconciliation fault rather than inventing a
repayment. Such an anomalous episode remains contained; no generic fault reset or
automatic resume is shipped. Future qualified reconciliation must explain the
ownership changes before enabling that path.

## Risk, coverage authority and extraction limits

P09 eligible free capital is additionally bounded by `R - P`, alongside its existing
house-equity, backing, pending-deficit and shared-hold bounds. Do not subtract `P`
again from backing: the original negative customer balance already affected it.
Recognition, commitment and absorption use the same transitions in prospective
stress replay. Scenarios still cannot inject future deposits or hoped-for recovery.
An inactive protection contract preserves earlier layer behavior; once active,
ordinary admission requires explicit coverage policy and joined risk qualification.

`Control::Protection` has an immutable-version policy with administrator epoch,
qualification expiry, global/per-customer lifetime absorption ceilings and a
commitment to the separately approved coverage/priority terms. An exact decision
binds request, ledger cut, revision, epoch, expiry and all allocation rows to an
administrator-authentication-port digest; the surrounding journal CAS binds all
other lifecycle state. These structs and hashes are **not signature verification**.
They must not be accepted as public API authentication. P18 connects verified
administrator identity to this port. Raw observation input cannot bypass the
controller to designate, commit or absorb capital.

There is no default first-come-first-served, pro-rata or operator discretion rule
silently chosen here. Every competing allocation needs an explicitly authenticated
plan under the deployment's approved G02 terms. Zero ceilings disable discretionary
absorption while retaining actual losses. A policy revision does not reset lifetime
spend; deliberately raising the ceiling requires a new authorized configuration.

The tracked opposing-account counterexample is intentionally **not** a safety
success: customers starting with 200 combined can finish with 220 after a 20 house
absorption, despite exact reconciliation and zero final native exposure. A global
ceiling bounds that extraction across accounts, episodes and recoveries. A per-user
ceiling alone is not Sybil resistance; stronger production prevention/calibration
is not claimed. Owed remediation remains a real liability, not capped away to make
the books appear safe.

Designation reduction cannot outrun recognized commitments/unfunded claims and
must leave joined admission satisfied. It is only a bookkeeping release: there is
**no house capital cash-withdrawal sender**. Do not manufacture one using a customer
payout identity. Capital remains unavailable for external withdrawal until the
later custody path is qualified. No customer capital becomes house designation.

## Evidence and limits

Wire **5**, engine **6** explicitly reject old replay semantics. Canonical private
state commits designation, all original/resolved claim fields, zero-valued collection
history, cumulative absorption, receipt faults and active policy. Reopening the
journal reconstructs the same state without importing research code.

The tracked protection suite covers V06, repeated episodes, partial depletion,
pre/post-absorption recovery, excess receipts, remediation offsets/insolvency,
duplicate causes, exact replay, authentication/cut binding, observation bypass,
global extraction caps, unresolved commitments and risk-constrained designation.
These are bounded deterministic tests, not a proof of adequate live insurance.

| Promoted local reference | SHA-256 |
| --- | --- |
| `work/specs/financial-model/protected-lifecycle.cjs` | `b0edc6a2d184314f03c6993fb802df122bf0105b2ceb32b54224c874ef493969` |
| `work/specs/financial-model/protected-lifecycle-checks.cjs` | `f10dd8fca6cf0a3b3a9449d0536ed6b6f0f6a1b6780d11bc2f8abf759d4e6d2d` |
| `work/specs/financial-model/model.cjs` | `853d4955f0a3b0c13125558105b37566bce374e764e69782d215938aee0ef74f` |
| `work/specs/financial-model/checks.cjs` (relevant protection vectors) | `0c9a0e36e48fee5e90ac3e3efb3865f184b850b6735fdc2246d2c580a8a091e3` |

The reference's single-episode shortcut is not promoted. The review/development
skills guided checked arithmetic, authorization-boundary review and adversarial
counterexamples; they did not authorize deployments, live coverage or new product
economics. No wallet, external fund contribution or venue call is used here.
