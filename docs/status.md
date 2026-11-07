# Implementation status and qualification limits

Repository bookkeeping reviewed 2026-11-07; financial and hardware evidence
remains unchanged through 2026-10-23. This describes this backend tree, not a
production release. Current execution priority is in the
[tracker](implementation/TRACKER.md); detailed contracts remain in the
[acceptance criteria](implementation/PLAN.md).

## Repository branches

`main` is the canonical TEE product trunk. The pre-TEE Phoenix/PER `main` at
`8f9c5140976f95081470ffd8370509a467e4836e` is preserved, with its history, on
`archive/phoenix-per-poc-20261107`. The old `product/tee-v1` branch remains a
pre-promotion snapshot at `7266fd6d86d81586b70ca532a93103e6e51ac02b`.
This promotion changes the development entrypoint, not financial qualification
or release approval. Historical branch/source references below remain original.
Promotion [PR #91](https://github.com/arnabnandikgp/cinder/pull/91) merged at
`5037a403c45960efb62aa70edd9fc67ab9401138` after eight passing PR checks.

The local product workspace is consolidated into the normal `cinder` checkout.
The redundant `cinder-tee` worktree is removed; its ignored research and local
files are preserved in the canonical checkout. Old Phoenix-local artifacts and
the private migration receipt remain in ignored `work/` namespaces, not Git or
CI dependencies. Historical absolute paths identify their original locations;
their contents and qualification meaning have not been rewritten.

Implementation PRs #83, #84, #86, #87 and #88 merged into `product/tee-v1` at
`fd7014bd9dd1d394c735d6da567318282fdc46c7` after eight passing checks on each
final head and review closeout. P36's bounded acceptance/receipt scope and
P37–P39 implementation milestones are closed. **P40 financial qualification,
P23 live financial acceptance and P24 release review remain open.** Documentation
consolidation [PR #89](https://github.com/arnabnandikgp/cinder/pull/89) merged at
`58c9da1c7e2f690460c8195927cd256c5fe9cda7`; all eight offline checks passed on
that merged tree. Bookkeeping [PR #90](https://github.com/arnabnandikgp/cinder/pull/90)
merged at `7266fd6d86d81586b70ca532a93103e6e51ac02b`, after its minor review
correction, reproducible CI source repair and all eight passing checks. Neither
bookkeeping nor trunk promotion changes qualification.

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

The implemented stack and documentation consolidation are merged. The current
scope is local workspace consolidation only; the following sequence remains proposed,
not started:

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
