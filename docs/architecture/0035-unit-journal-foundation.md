# ADR 0035 — New-history execution-unit journal foundation

Date: 2026-10-16. Implements the first offline slice of the approved
[unit boundary](0034-execution-unit-boundary.md), not multi-venue activation.

## One history, existing economics

`journal::units::Journal` is an explicit alternative new-history entrypoint.
It uses the same opaque Backend, Protection, frame chain, CAS and freshness
contracts as the single-unit journal. There is one global head and accepted
transaction sequence, never one independently persisted journal per venue.
Its immutable registry contains one or two units; each component reuses the
existing `model::State` and exact kernel transition rules. No new financial
equations, capital transfers or native authority follow from a descriptor.

Genesis binds nonzero unit IDs, the domain, native environment/account, distinct
funding wallets, custody configs/payout vaults, key-role/policy commitments and
the complete kernel config (sources, units, markets and policies). Distinct
source physical accounts prevent namespace aliases from duplicating backing.
These are configuration consistency checks, not proof of native isolation or
correct chain addresses. New genesis starts with no customers or balances.
Legacy format/engine and shipping Runtime remain unchanged. Neither format can
open the other, and create never overwrites a nonempty store. No active-state
migration or snapshot import is provided.

## Shared ownership and bounded foundation controls

Enrollment atomically installs the same wallet-derived customer in every unit,
with an explicit recipient for each unit's asset and authority epoch one. A
global epoch advance updates every component in the same append; failed or
duplicate mutations cannot partially enroll or revoke. Owner authentication
remains the calling controller's responsibility; this is not a public API.

Unit transactions bind the unit and the exact shared head. Observations reuse
existing source/envelope/duplicate/conflict rules. A foreign or unnormalized
input is retained and contained in the selected unit, not silently routed by a
symbol. Unit-local evidence/financial problems do not change another component.
Any shared storage/freshness uncertainty fences the whole handle.

The foundation allows only generic reserve/release/prepare, deposit notices and
qualified raw resolution. It pins `(customer, request)` to the original unit and
canonical reservation digest, including released operations. Prepare must match
that route and the global current authority epoch. This reservation commitment
is **not** the future signed order's economic-intent digest. Future order/funds
controllers must extend/version the contract before any exposure is permitted.
All other controls (including per-unit registration/epoch changes, exposure,
recovery and financial admission controllers) are refused as a control result;
valid observations in the same transaction still apply and remain durable.
There is no exposure delivery or signer in this entrypoint.

## Durability and next boundaries

Replay independently checks expected genesis, global transaction identities,
canonical bytes, inner/outer header equality, receipt and the complete committed
state (registry, customers, epochs, routes and every component). Exact retries
return their original receipt only after freshness validation. Unknown append
outcomes disable every state view/mutation until explicit reload; reload rebuilds
one state from authenticated history, not independent projections.

This phase does not attach P23's read-publication or shipping service to the new
entrypoint. Versioned unit-bound API signatures/grants, economic-intent routing,
full controllers, worker scheduling, global recovery fencing and joined Runtime
acceptance remain the next composition slice. No v1 default routing, live venue,
RPC/AWS qualification, cross-margin, frontend or Pacifica credit investigation.
