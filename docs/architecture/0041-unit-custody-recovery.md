# ADR 0041 — Unit custody, owner discovery and shared recovery cutover

Date: 2026-10-16. P35 composes [ADR 0030](0030-venue-directed-escrow.md) and
[ADR 0040](0040-unit-native-execution.md). No new program, asset, coverage
promise, bridge or production authority is introduced.

## Existing money, distinct routes

The existing Pacifica funding controller and private recovery package machinery
accept the sealed financial Controller. Legacy use retains its checks; selected
use additionally binds exact native environment/account, funding wallet, custody
config and payout vault. Unit mode requires exact-batch escrow and canonical
config/token-vault PDAs, not common-vault or weak demo-credit composition. The
empty Config remains immutable; accepted registrations supply beneficiaries
without changing keys, routes, precision or lifetime paid baselines.

An escrow request is permanently routed to one unit. Full sweep and owner refund
remain exclusive. Handoff recognizes the claim once at the broker with retained
restrictions. Its distinct Broker → Venue instruction inherits only the original
full-batch consent. Arrival does not create a second deposit. Partial/uncertain
credit and unsupported terminal evidence retain pending inventory/holds. Another
unit's account/source/backing cannot discharge them. Payout acceptance requires
the signed API decision; demo flat-allocation/credit cannot activate this mode.

Typed trusted Runtime workers use actual time and the one writer for
preparation/completion. Exact signed-wire persistence precedes submission;
chain/native I/O runs outside the lock. Original fees, counters, finality and
qualified-credit checks remain. No public API exposes internal Approval, raw
transfers, balance setters or operator recovery actions.

## Purpose-separated owner discovery

`api::units::onboarding` is opt-in on the same confidential carrier.
`UnitOnboardingClient` signs `CINDER-UNIT-ONBOARD` plus UnitId, the independently
pinned complete custody contract and existing domain/policy/wallet/session/action
fields. V1 signatures are invalid; no URL-selected/default unit exists. Canonical
classic-SPL recipients and custody config/route/payout PDAs are checked. Complete
custody policy contributes to carrier release binding separately from the
existing API-history contract.

Enrollment atomically adds empty books in all units at one shared epoch and
retains the original proof for startup verification. Public bootstrap mode
refuses unverifiable trusted-fixture Enroll history. Reopen retrieves the same
identity/epoch without resetting grants, claims or counters. Deposit notices are
selected-unit locators, never credit. Wrong contract/unit/session/owner/epoch or
conflicting originals refuse. A complete recovery-key inventory prohibits new
enrollment. SDK checks unit reply and wallet-derived identity with no retry.
Shipping v1 bootstrap remains separate.

## Global cutover, separately backed claims

Trusted operator `BeginRecovery` requires every unit in registry order, exact
reviewed versions, bounded recovery authority and independently qualified fencing
evidence commitment. Existing per-component Begin checks run all-or-none. A
missing/stale unit cannot partially freeze the deployment. Local Begin/freeze
cannot substitute; expiry never reopens ordinary operation.

Existing settled-cut logic still requires flat/settled books, actual final
reconciliation, resolved facts/holds/movements, returned assets and actual
payout-vault backing for positive claims. Fresh shared head and each unit's own
final check are mandatory. A's assets cannot fund B. Original config/customer
normal/recovery counters are checked, not reset. Existing encrypted package,
owner-key registry and dual-readback machinery are reused. Key registration
refuses any recovery, even expired. Package preparation is an exclusive operator
workflow, not replica I/O under a serving Runtime lock. Later accepted facts
invalidate prepared roots; separate qualified chain/operator approval still
governs publication/activation. No heartbeat or immediate-outage payout exists.

Unit replay revision **5** starts empty histories; no migration from 1–4 or new
counter baselines for existing liabilities. Synthetic tests cover real bootstrap
signatures, canonical recipients, restart/revoke, exclusive sweep/refund,
once-only handoff, full allocation, missing-credit holds, global recovery,
shared-head/final-check binding and selected Pacifica adapter/counters. Legacy
partial movement/payout/package tests exercise the same reused implementation.

**BULK funding/return rails remain disabled.** Order/account APIs are not an
invented bridge. Pacifica's deferred missing-credit investigation remains
deferred. No financial/devnet/RPC/AWS or changed-image run is authorized here.
P36 joins offline workflows and records a separate qualification proposal.
