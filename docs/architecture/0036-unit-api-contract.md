# ADR 0036 — Signed execution-unit API/SDK contract

Date: 2026-10-16. P30 is the next bounded slice after the approved
[unit boundary](0034-execution-unit-boundary.md) and
[journal foundation](0035-unit-journal-foundation.md).

## Explicit protocol, unchanged economics

API v2 signs UnitId inside both the Ed25519 request preimage and stable economic
digest. Keep the existing strategy command bodies, exact integers, request IDs,
financial deadlines and owner/agent distinction. V1 and v2 use distinct magic
versions and never silently upgrade/default/fall back. The current runnable
service and PrivateClient remain v1; this phase adds opt-in contract tooling,
not a second listener or an activated multi-venue service.

V2 request layout is v2 magic, existing economic prefix
`network/deployment/account/request/policy`, nonzero UnitId, existing
`epoch/signer/session/auth-expiry`, command and signature. V2 digest uses its own
intent magic plus the same economic prefix, UnitId and command; session, signer,
epoch and auth expiry remain excluded. Unit changes are different economics.
The 1,024-byte request limit includes the new field. The reply is v2 reply magic,
UnitId and the unchanged bounded v1 reply body. The SDK requires the expected
unit; neither an unwrapped v1 reply nor another unit's reply is accepted.

## Scope verification is not financial admission

The opt-in authorizer validates the signature/channel/deployment/policy, then
checks the shared journal's current freshness and customer epoch. It selects
the immutable configured unit and checks that component's owner/epoch too.
Owner grants are for one unit/market; READ exposes that unit's account views,
including its other markets, not every venue. TRADE/CANCEL retain the current
bounded method rules. Agents cannot grant, revoke, pay out or change leverage.
Global revoke-all remains a shared journal action, never a unit-local epoch.

Agent lookup is a **trusted, in-enclave, immutable accepted-history port**. It
must return the original unit/account/epoch, grant, acceptance time and current
head-bound usage cut; a parent-supplied grant or merely signed-but-unaccepted
proposal is not such evidence. The authorizer checks every scope field, the
fresh head, key/market/method bounds and expiry. Grant acceptance time must
also be no later than that head's logical cut. No adapter for this port is
shipped in this slice: the next durable API/controller join must implement it
from accepted history, not an independent grants database. Owner-only callers
can use the explicit empty-grant implementation.

The returned checked request is only signature/scope preflight. It neither
installs a grant nor revokes an epoch, creates a hold, consumes quota, proves
idempotency/cancel ownership, qualifies native risk, signs or dispatches. Lifetime
order quota is checked after operation dedupe in the later admission controller,
so exhausted quota must not prevent authorization of an exact retry/read.
Any later controller must recheck the captured head/epoch and all financial
conditions atomically before exposure. Limits remain trusted release policy,
not invented calibrated production values.

## Delivery and remaining join

Rust and browser/Node SDK helpers share command vectors and independent
Ed25519/SHA-256 expectations. Signing snapshots private bytes before awaiting a
wallet; returned buffers belong only in the verified confidential channel and
must be erased after use. No network client, retries or native signer is added.

P30 tests current-head fencing, two units with the same owner/market, unit/session/
policy/version tampering, scoped grants, global revocation, stable digests and
wrong-unit replies. Existing v1 fixtures and runnable-service tests stay required.
Durable API records/intent routing, grant installation and quota/cancel checks,
full controllers, read publication, global recovery and joined Runtime remain
the next implementation boundary. No active-history migration, frontend,
venue/RPC/AWS invocation, new financial terms or native capability is implied.
