# Implementation baseline

Approved directions as of 2026-09-30; tracked extraction, not a new product-policy
approval or deployment claim. [Plan](PLAN.md), [tracker](TRACKER.md),
[source/evidence register](EVIDENCE.md). Requirement IDs below are stable.

2026-10-08 bounded demo addendum: [ADR 0029](../architecture/0029-demo-deposit-confirmation.md)
permits original-finalized-signature deposit-history matching plus available
balance corroboration under an explicit testnet venue-bookkeeping assumption.
Its weaker journal marker is not B05 production credit qualification, a native
risk-readiness grant or B07 recovery backing. The original stronger contracts and
unresolved production gates remain; do not treat this approval as a silent
general relaxation of funding finality.

The subsequent separately approved initial-demo setup boundary in ADR 0029
accepts fresh non-atomic venue reads for an idle, exclusively controlled test
account with disabled lending and zero debt. Its explicit policy and retained
provenance can authorize only original bounded ingress, not a second allocation.
`Setup.complete`, native risk readiness and the stronger withdrawal/recovery
contracts remain unchanged; activation and changed-image qualification remain
separate. This is not production setup or a solvency proof.

The subsequently approved local allocation implementation adds one private
configuration grant: one original verified owner deposit, one fixed quote amount,
two distinct request IDs, the current customer authority epoch and an absolute
expiry. Its only route is Vault → Broker → Pacifica testnet. Actual loaded
controllers, chain configuration and demo policy bind its application/KMS context;
durable journal identities prevent renewal or a second allocation. It is governed
internal working-collateral authority, not a customer signature or another credit.
No public funding endpoint, trading, withdrawal, payout, recovery relaxation or
live-run permission follows. Versions 1–5 still reject financial activation.
The subsequent offline v6 composition permits only this private grant's two
retained ingress acceptances and rejects trading, unrelated funds and payouts.
The user subsequently approved a separate common-journal flat-funding rule in
[the continuation map](P23-FINANCIAL-GATES.md#fresh-flat-account-admission-decision--pending-user-review).
Its durable certificate permits only the two fixed cash transfers against the
actual original receipt, with zero positions/funding/debt/other commitments and
exact location capacity. It rechecks before preparation and exposure/signing;
the second leg needs full first-leg completion. No complete native check, mark
or collateral cut is invented. Ordinary admission, withdrawals, payouts, trading
readiness and recovery retain their stronger contracts. Local tests do not grant
changed-image live authority.

## Product and custody contract

| ID | Binding direction | Boundary / unresolved qualification |
| --- | --- | --- |
| B01 | Private, programmatic broker over existing perp liquidity; Pacifica first, Nitro runtime | BULK paused; no Phoenix/PER inheritance, website, token, staking or multi-venue launch in this sequence |
| B02 | Individual customer entitlements over pooled external execution; initial pooled account, justified risk cohorts later | Not LP shares or a separate native account per user. No internal crossing; external account netting is not internal matching. Native isolation, fee aggregation and cohort thresholds need evidence |
| B03 | Actual attributed fills determine customer basis/PnL; selectable leverage under a market cap | No native-account-equivalence, uninterrupted exposure or best/lowest-fee guarantee; numerical caps and pricing remain gates |
| B04 | Solana wallet → program vault → supported, allowlisted broker funding route → venue, and back | Intermediate key-controlled account remains a custody boundary. Program custody of vault funds does not imply PDA authority over HTTP venue actions |
| B05 | Allocate working collateral, not full perp notional; retain useful venue liquidity between orders | No credit before qualified deposit finality; intra-Cinder transfers do not create new customer credit. No assumption that every order waits for a fresh transfer |
| B06 | Private balances, positions, orders and trading history; public deposits and payouts | Not private transfers. Minimize metadata, logs and proof leakage, including during recovery |
| B07 | Operator-assisted recovery: fence, reconcile, settle/return assets, finalize claims, fund vault, authorize activation | Heartbeat failure is an alarm, not payment authority. User claims can bypass the trading API only after authorized activation; not unconditional operator-free exit |
| B08 | Thin secure SDK preserves strategy semantics while exposing necessary Cinder differences | Not literal URL-only native API compatibility. Native signatures do not authorize a customer's use of the pooled account |

## Financial contract

| ID | Binding direction | Implementation consequence |
| --- | --- | --- |
| F01 | One deterministic ledger with customer, house and suspense ownership | Research states are projections, not additive independent books |
| F02 | Exact units, signed remaining basis, explicit precision/rounding | Display average price is not the accounting basis; version all native conversion rules |
| F03 | Reconcile both exposure and equity; per-user attribution matters independently | Aggregate equality cannot hide wrong-owner fills or omitted liabilities |
| F04 | Negative customer equity has zero collectible asset value in the baseline | Report positive-claim shortfalls without netting them against imaginary collectible debts |
| F05 | Ordinary trading/funding/declared costs belong to their customer; Cinder owns its operational-error obligations | No silent customer mutualization, unexplained balancing plug, or unconditional insurance of venue failures |
| F06 | House-funded quote-asset protection; no outside redeemable fund shares initially | Capital designation is not new cash. Coverage recognition, reservation, absorption and payout must not charge the same loss twice |
| F07 | Preserve unpaid claims and restrict preferential payout/new risk when protection is insufficient | No first-come insolvency distribution, automatic healthy-user haircut or erased debt; final priority/coverage terms remain a gate |
| F08 | Later debtor recoveries first address remaining debt, then replenish resources that absorbed that debt; excess is customer credit | Distinguish recovery rights from proven collectibility; prevent double recovery across episodes |
| F09 | Ordinary withdrawals may use genuinely free collateral even with open positions | Outcome-aware margin/capital/location checks; durable FIFO per pool/asset when solvent, total debit reservation and consent for partial payment. FIFO is not insolvency priority |
| F10 | Risk admission considers gross customer exposure, native net exposure, pending outcomes, capital and time/location liquidity separately | Reduce-only is not a theorem of lower pooled risk. New shocks can breach thresholds; ingestion must still record them |
| F11 | Distinguish customer liquidation, venue ADL and pool distress; mitigate via justified risk units and bounded house support | No guaranteed avoidance of healthy-customer reductions; no unapproved transfer of risk between cohorts |
| F12 | RF1: qualified actual restoration preserves original entry economics; house owns replacement-price differences, replacement fees and corresponding ADL-fee refunds | Favorable differences also belong to house. No hypothetical funding while quantity is absent; unrestored quantity settles actual ADL economics |
| F13 | RF2: proportional allocation to affected same-side quantity at a precommitted cut within the risk unit | No operator/profit/API-first preference. Whole-lot bounds, not wallet-splitting immunity; see allocation rule below |
| F14 | Funded and hard-limited exceptional house exposure handles late-close excess, with explicit unwind | Never silently reverse a reduce-only customer. Not permission for speculative house trading or unlimited restoration |

## Minimal mathematical contract

Use integer quantity lots and quote atoms with an explicit per-market multiplier.
The following normalized linear-perp equations absorb that multiplier into price;
they do not select Pacifica wire precision. Customer `i`, market `m`, house `H`,
native account `V`, physical cash location `l` and transfer `j` have separate IDs.

Stored: settled signed cash `c`, signed quantity `q`, signed remaining basis `b`,
recognized unsettled funding `a`, location assets `X`, transfer receivables `T`,
unattributed/third-party obligations `L0`, reservations, consumed event IDs and
policy versions. A native signed cash ledger is not a physical token balance.

At the same valid mark `p`, for complete reconciled evidence:

```text
e_i = c_i + sum_m(q_im*p_m - b_im) + a_i
h   = c_H + sum_m(q_Hm*p_m - b_Hm) + a_H
N   = sum_l X_l + sum_j T_j + sum_m(q_Vm*p_m - b_Vm) + a_V - L0
q_Vm = sum_i q_im + q_Hm
N = sum_i e_i + h
C = sum_i max(e_i, 0); D = sum_i max(-e_i, 0)
W = N - C = h - D; W_cons = W - J
```

`J` is a named conservative deduction not already booked. Unknown evidence makes
reconciliation unknown or bounded, not automatically zero. Do not count native
reported equity in addition to its components. `W >= 0` is neither immediate
payment liquidity nor immunity to later shocks. A correct ledger can be insolvent.

For actual signed fill `x` at execution price `v`, opening/adding gives
`q'=q+x, b'=b+x*v, c'=c`. For a reduction/reversal:

```text
s = sign(q); z = min(abs(q), abs(x))
beta = trunc_toward_zero(b*z/abs(q))
realized = s*z*v - beta; x_open = x + s*z
q' = q+x; b' = b-beta+x_open*v; c' = c+realized
```

Keep division residue in remaining basis. Full close consumes all old basis.
Before fees, the marked equity change equals `x*(p-v)`. Native and customer basis
can differ even when their quantities reconcile. PnL is not a spot notional debit.
Integer refinement for rational conversions: require exact whole-fill value
`T=x*v`, split `k=trunc_toward_zero(T*z/abs(x))`, and use
`realized=-k-beta`, `b'=b-beta+(T-k)`. This retains split residue in the opening
basis without demanding exactness of artificial sublegs. When both legs are
exact it is the same formula above; genuinely inexact whole fills still reject.

Qualified funding `f_i` updates `a_i += f_i`; native funding updates `a_V += f_V`.
Any house residual `f_V-sum_i(f_i)` requires attributed house exposure or a qualified
policy/rounding cause; otherwise contain/reconcile. Settlement transfers accrual to
cash, not a second earning. Net-flat native funding does not establish gross users'
funding without independent qualified rate/boundary inputs. Normalize fee-inclusive
native PnL before posting; do not deduct its fee twice.

Deposit: qualified final `d` adds assets and customer entitlement once (subject to
existing debt recovery). Transfer source debit moves `X_source` to `T_j`; linked
arrival moves `T_j` to `X_destination` less an attributed fee. Reservations create
neither assets nor entitlements. Final payout reduces assets and its owner's claim
once and advances the durable paid counter. Source debit alone is not final payout.

For existing deficits, `R` designates house protection, `P` commits some of it and
`O` encumbers other house capital not already deducted:

```text
F_free = max(0, min(R-P, W_cons-O))
```

Still report untruncated shortfalls. Flat deficit absorption `x` posts
`c_i += x; c_H -= x; R -= x; P -= x; z_i += x`, where `z_i` is a zero-valued
collection record. It allocates an already present loss, not a new asset or another
economic loss. A new owed remediation amount is separately recognized once.
Capital targets derive from transition paths and peak depletion, not a percentage
chosen without calibration. Future fees/fundraising are not current available cash.

### RF2 finite-lot rule

For a qualified economic ADL event, freeze same-side quantities `w_i`, total `Wq`
and actual removed lots `Dq`. Largest remainder allocates `D_i` once:
`floor(Dq*w_i/Wq)` plus residual lots by fractional remainder then canonical ID.
Each allocation differs from its ideal share by less than one lot. Duplicate feed
messages are not new events; distinct executions are not arbitrarily merged.

For incremental restoration, freeze unit jobs for each customer's kth removed lot:

```text
release(i,k)  = floor((k-1)*Dq/D_i)+1
deadline(i,k) = ceil(k*Dq/D_i)
```

Choose available next jobs by earliest deadline then canonical ID. At restored
prefix `r`, awards are monotone and within floor/ceiling of `r*D_i/Dq`. Full
restoration returns exactly `D_i`. Composed residual reduction is within **two**
lots of original-position proportions, not one. This is a hand-derived and bounded
tested reference, not a machine proof or scalable production scheduler.

Fee refunds and signed basis use cumulative pieces
`trunc(x*(n+k)/D_i)-trunc(x*n/D_i)`; full restoration returns exact original totals.
A changed private close intent voids remaining restoration for that customer;
do not opportunistically reweight or reopen them. Production must join this rule
to actual causality, late fills, resource bounds and exception accounting.

## Execution, privacy and recovery requirements

| ID | Requirement |
| --- | --- |
| X01 | Event envelope binds network/deployment, venue/account, source namespace/economic ID, operation/attempt, market/asset precision version, causal execution cut and observation time, payload hash, authority and policy versions |
| X02 | Raw evidence remains append-only. Same ID/same payload is idempotent; changed payload is a conflict. Atomically commit postings, consumed keys, holds and version |
| X03 | Durable intent precedes capability/signature exposure. Unknown accepted actions keep holds. Retry only with evidence; database leases do not revoke native credentials or escaped requests |
| X04 | Order ACK, fill, cancel ACK and complete settlement are different states. Record real adverse events even during freeze; fail closed on unbounded new actions, not on facts |
| X05 | Normal risk, restricted, reconciling/frozen, prepared recovery, active recovery and unresolved shortfall are distinct states with explicit permitted actions |
| S01 | Separate owner/Cinder agent, pooled trading signer, funds/recovery authority and storage/transport keys. Grants bind domain, methods, expiry, limits and authority epoch |
| S02 | Authenticated attestation binds approved release measurement, freshness and session key. Private transport terminates in the enclave; outbound venue TLS is authenticated there too |
| S03 | Parent storage contains authenticated ciphertext only. Durable journal/snapshots need replication, rollback protection and a trusted fresh head; multiple local directories are not independent witnesses |
| S04 | KMS release revocation does not revoke a key already loaded in an enclave. Fence old workers and all outstanding venue capabilities before cutover |
| S05 | A hostile frontend can steal data before encryption; client release distribution and verification belong in the threat model |
| R01 | Final recovery claims follow settled/reconciled liabilities, existing payouts, pending actions and returned assets. Stale snapshots are not payable entitlements |
| R02 | Recovery root binds program/network, asset, pool, epoch, claim identity/recipient, amount, cutoff and paid-counter basis, with privacy salt |
| R03 | Activation is operator-authorized only after fencing, final reconciliation and segregated available backing. Root replacement cannot reset paid claims; late deposits use suspense/refund paths |
| R04 | Encrypted claim packages (leaf/salt/path/domain/counter data) remain obtainable outside the trading API. Claimants can verify/submit after activation; public claims reveal payment details, not full private history |
| R05 | Attested code/channel, own-claim inclusion, complete liabilities/backing and executable exit are separate assurances. Inclusion alone proves neither completeness nor solvency |

## Gates still requiring evidence or substantive review

| Gate | Must be resolved before | Not implied by existing approval |
| --- | --- | --- |
| G01 — native semantics | Enabling an adapter capability | Fee/funding precision, causal completeness, real partial/forced events, native authority/isolation, fee aggregation or mainnet parity |
| G02 — financial policy | Authorizing live customer risk | Numerical leverage/gross/depth limits, horizons/buffers, coverage eligibility/priority, reserve access and replenishment, fee terms, cohort thresholds |
| G03 — security governance | Releasing production keys or upgrade authority | Authority topology, recovery approvals, release measurements, rollback witness, key rotation and compromise response |
| G04 — D14 assurance | Advertising backing/exit verification or accepting customer funds | Production completeness/assets proof, independent review and what the user verifier actually establishes; no automatic ZK requirement |
| G05 — external authority | Each live test/deployment | Environment, accounts, assets, limits, cleanup and funding authority; AWS costs and mainnet permission separately |

Approved policy must not be reopened merely to start coding. An unqualified
capability stays disabled; a synthetic policy can exercise a test, never silently
authorize real money. Substantive changes require a decision record and user review.
