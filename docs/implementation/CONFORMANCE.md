# Initial portable conformance cases

Tracked extraction of approved toy economics from W01/W02/W09 in [EVIDENCE](EVIDENCE.md).
These are expected outcomes, **not tests of a production engine that already exists**.
Owning phases convert them and the broader named suites into executable fixtures,
record units/provenance and run differential checks. Prices below are normalized
linear-perp quote per lot, not native Pacifica decimals or spot payment obligations.

| ID | Setup / events | Required outcome | Owner |
| --- | --- | --- | --- |
| V01 | Final deposit 100; transfer 60 from vault to venue | User entitlement remains 100; physical locations 40/60. During transit, debit is a receivable, not simultaneous cash at both locations. No second customer credit | P03, P08, P16 |
| V02 | Start `q=3,b=100,c=0`; sell 1 at 40, then sell 4 at 35 | First `q=2,b=67,c=7`; then `q=-2,b=-70,c=10`. Full closed-side basis consumed, reversal has new-side basis only | P02–P03 |
| V03 | Alice/Bob deposit 100 each, house 30. Alice long 1 at 100, Bob short 1 at 100. Mark 250, no fees/funding | Native net zero and assets 230. Alice equity 250, Bob -50, house 30. Exact bridge holds; positive claims 250, shortfall 20. Report insolvency, do not invent 50 collectible cash | P03, P09–P10 |
| V04 | Alice long 10, Bob short 10; qualified toy funding `f=-q*1` | Alice accrual -10, Bob +10, native net 0. Settlement moves accrual to cash once; repeated receipt has no effect. Missing rate/boundary evidence is not zero funding | P04 |
| V05 | Claim 100; all 100 physical cash at venue, payout location zero | Backed on this simple valuation, but no immediate payout liquidity. Moving/withdrawing needs actual qualified receipts; timeout is not failure or available cash | P08–P09 |
| V06 | Flat debtor -20; house `h=R=100`, existing coverage `P=20`, `J=O=0` | Before absorption `W=F_free=80`. Absorb 20: debtor 0, house `h=R=80`, `P=0`, collection record 20 with zero asset value. Still `W=F_free=80`; winner payout does not charge fund again | P10 |
| V07 | Alice long 10 at 100, collateral 400; all 10 ADL at 100, fee 2. Four actually restore at 103 with new fee 0.80 | Four retain entry 100; six stay closed. Cash 398.80, house restoration cost 13.60 = price difference 12 + refunded ADL fee 0.80 + new fee 0.80. No funding while absent. Signed-basis handling also tested for shorts | P12 |
| V08 | Alice/Bob claims initially 100/200. Alice receives ordinary payout 20 and incurs settled loss/cost 35 before final recovery cut | Final Alice/Bob claims 45/200, not stale 100/200 and not double-debited 25/200. Only actual available backing can activate payouts | P17, P21 |
| V09 | Economic event E arrives twice, then same ID with different payload; process crashes between durable commit and response | First posts once, duplicate no-op, conflict contained. Restart restores committed state/holds/consumption. Client response loss cannot justify a second signed economic action | P05–P08, P22 |
| V10 | Two covered shocks of 90 occur before a later recovery of 170 | Peak resource need is 180 on this path, not terminal net 10. If only 100 is available, preserve unpaid obligation and restrict action; do not cap modeled loss at available fund | P09–P12, P22 |

Additional mandatory families: winner while net pool loses and reverse; different
entry/funding times; gross/net margin; partial/cancel history races; all 24 transfer
receipt permutations; partial refunds/impairments/overpayments; repeated defaults;
late-close house exceptions; capital shock after restoration reservation; funding
gap; immutable RF2 cuts, all prefix quotas and composed dust bounds; account
splitting/manipulated event order; missing/stale proof kits; unknown signed actions
at cutover; root/counter replay and downstream transfer failure.

RF2 regression must retain the 3,905 small-vector set, 250 seeded uneven vectors,
10,000-lot skew case and splitting counterexample (or an explicitly stronger,
traceable equivalent). Production scalability cannot depend on materializing a
unit job for every atomic lot. A stronger implementation still needs refinement
tests against exact small cases and independent review of its bounds.

Each executable vector records pre-state, ordered evidence, expected postings,
post-state, assumptions and required operating mode. Include shortfall states as
correct expected outcomes. Reference agreement alone does not authenticate a venue
receipt, establish legal collectibility, calibrate capital or verify hardware.
