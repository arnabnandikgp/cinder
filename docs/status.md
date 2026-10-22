# Implementation status and qualification limits

Evidence reviewed through 2026-10-23. This describes this backend tree, not a
production release. Current execution priority is in the
[tracker](implementation/TRACKER.md); detailed contracts remain in the
[acceptance criteria](implementation/PLAN.md).

## Implemented surface

| Area | Present in the code | Qualification boundary |
| --- | --- | --- |
| Financial core | Exact accounting, attribution, funding/fees, margin/risk, protection, liquidation and ADL rules | Native inputs, numerical policy and production capital/coverage require qualification |
| Custody/recovery | Anchor escrow batches/refunds/sweeps, constrained payouts, shared payment counters, final claims and independent package delivery | Local SBF/SDK workflows are not a live venue-funded recovery |
| Private API/SDK | Owner/agent authentication, operation lookup, encrypted HTTP/WebSocket, private reads and bounded subscriptions | Browser/Node and exact-image Nitro evidence have separate scopes |
| Multi-venue composition | Unit-bound API, financial state, native workers and custody/recovery under one accepted journal | No cross-venue collateral substitution or automatic native capability equivalence |
| Pacifica | Execution/observation/funding components and joined offline workflows | Missing-credit/live financial investigation deferred; no complete live round trip claimed |
| BULK | Exact wire/financial observations, agent execution, bounded reads, fill attribution and measured unit assembly | Funding/returns unqualified; latest assembled close blocked by active-account precision/funding |

## Latest measured result

The [October 21 receipt](archive/qualification/unit-paper-hardware-20261021.md)
records two separately scoped runs. The corrected image, frozen at `e72484f`,
passed seven-role key release, one-time native-backed paper initialization,
an actual encrypted SDK opening and customer/unit fill attribution. Same-image
restart preserved existing cash/position; independent witness fencing refused
both units. These are genuine passes within that scope.

Cinder's close refused active-account observations: six exact original account
responses fail the eight-decimal decoder; later reports also contain nonzero
funding outside the paper policy's qualified scope. No precision rounding, native
balance import, unresolved-event deletion or admission bypass was used.

A separately approved local **test teardown**, after fencing/stopping the enclave,
closed only the known paper position and revoked the agent. It is not a passing
Cinder close or final reconciled cash bridge. The run's cloud resources were
independently removed and private evidence retained. Both one-host approvals are
consumed; the receipt authorizes no new live activity.

## Remaining work

The user requested review/merge of the implemented stack and documentation
consolidation first, then a stop before this sequence:

1. Promote sanitized active-account response fixtures that reproduce the exact
   precision/funding failures.
2. Distinguish reported native valuation precision from exact settlement money;
   preserve original values instead of rounding them into ledger postings.
3. Establish the funding settlement/allocation evidence and any explicitly
   reviewed paper-policy change. Do not infer payments from balance changes.
4. Verify the joined opening → fill → funding → close → reconciliation → restart
   lifecycle locally, then propose fresh bounded hardware acceptance if needed.

The remaining original hardware acceptance includes the Cinder close/final cash
bridge and complete current-image Node/Chrome administration/receipt/restart
checks. The above local correction is not implemented by the cleanup PR.

Deferred, not silently dropped: Pacifica credit investigation, live native
funding/return/payout and venue-backed recovery qualification, production
financial calibration, key/recovery governance and independent assurance.
Frontend integration and another venue are not part of this workstream.

## Release gates

G01 native semantics, G02 financial policy, G03 security governance, G04 user
verification/assurance and G05 live-run authority remain distinct. Their exact
requirements are in the [financial contract](implementation/BASELINE.md).
There is no customer-funds/mainnet authorization, audit, universal solvency proof,
unconditional exit guarantee or verified lowest-fee promise implied by this tree.

Merge records are the linked GitHub PRs in the tracker. Phase completion still
requires its acceptance evidence; merging experimental disabled code does not
mark P23/P36/P40 financial qualification or P24 production review complete.
