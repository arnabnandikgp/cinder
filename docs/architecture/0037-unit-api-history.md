# ADR 0037 — Durable unit-scoped API history

Date: 2026-10-16. Implements the next bounded slice after
[P30's signed contract](0036-unit-api-contract.md), under the approved
[unit boundary](0034-execution-unit-boundary.md). No runtime activation.

## One accepted history, two different outcomes

Retain the original signed v2 mutation, unit, customer/request, economic digest,
signer, original epoch/policy, acceptance time and API contract commitment in the
same protected unit journal. A deployment-wide index permanently binds a request
to its unit/economics. Auth refresh is not a new intent. First authenticated
capture pins the API limits/registry commitment; changing it is not an implicit
upgrade. Derived grant/operation indexes are immutable views of this history,
never an independent mutable authorization database.

Owner grants and global revoke-all are administrative completions. Revoke advances
every unit's epoch in the same append as its original signed record. Grant keys
cannot be amended within the same unit/customer/epoch; revoke and grant again.
Same key/market at another unit is a separate grant. READ remains unit-wide.

Financial commands are **recorded-only**, not accepted orders, cancellations,
payouts or leverage changes. No hold, financial posting, quota consumption,
native preparation or exposure follows from capture. These records must not be
encoded as the existing financial API `Accepted` outcome. Future admission must
join the original intent, permissions, order budget, cancel/attempt ownership,
holds, risk and current head/epoch atomically. Only actually admitted orders
consume grant order budget; a recorded-only order does not. No signer or listener
is introduced, and shipping Runtime/PrivateClient remain v1.

## Authentication, replay and ownership

Authenticate the current confidential session, signature, scope, epoch and grant
before returning an existing operation or comparing its economics. Owners can
recover their own agent-created record; another agent cannot take it over.
Changed unit/economics conflicts, including after global revocation. Exact retries
return the original record and never repeat administrative effects. Revocation
requires a newly signed current-epoch retry; stale signatures are not exempt.
An owner may recover an expired original grant: current identity/session/epoch
are authenticated first, but the original grant deadline is not revalidated as
a new proposal before dedupe. A new grant still passes every proposal check.

Accepted grant cuts are rebuilt from original signed administrative records at
the fresh common head. Replay validates signed/header/effect consistency and
bounded grant semantics; malformed protected API records fail closed. Capture
does not accept caller-supplied grant evidence. Cancel capture checks the original
record's unit/owner/market, not a substitute unit hint; native attempt/lifecycle
checks still belong to admission. Read commands are not mutation records.

Foundation reservation commitments remain distinct from signed economic digests.
An existing reservation without an API record cannot be claimed retrospectively
by a new API request. API capture and future foundation reservations cannot move
the same request across units. A recorded-only intent is not ownership evidence
for applying native fills: existing financial routes still govern actual facts.

## Durability and explicit version boundary

Unit journal replay revision becomes 2 because accepted state/administrative
transitions change. Old opt-in revision-1 unit histories fail closed; no active
state migration, snapshot import or old/new mixing is supplied. The shipping
single-unit codec and financial engine revision remain unchanged. New empty
histories are the supported qualification target.

CAS, freshness, encryption, bounded history and uncertain-append fencing remain
the existing shared contracts. An unknown append outcome requires explicit
reload, then fresh authentication and lookup; no automatic resend. No duplicate
administrative effect or private-record reset follows from restart or response
loss. Synthetic offline tests are not native, hardware or production assurance.

Next: the atomic financial-controller join and then joined runtime/read/recovery
wiring. Native BULK qualification, deferred Pacifica credit, frontend integration
and any changed-image AWS run remain separately scoped.
