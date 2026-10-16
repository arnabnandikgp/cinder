# ADR 0038 — Unit-scoped financial admission

Date: 2026-10-16. Bounded offline continuation of [P31](0037-unit-api-history.md)
and the approved [unit boundary](0034-execution-unit-boundary.md).

## Atomic decision over the original intent

An opt-in controller authenticates the current channel/signature/owner epoch,
then deduplicates by customer/request and original unit/economics. It joins the
original signed record (new or previously recorded), original financial deadline
and epoch, original agent's still-valid grant and actual admitted-order budget,
trusted unit-specific hold computation and existing joined financial controls in
one shared-head append. A fresh retry signature cannot renew the original intent.
An owner retry of an agent request does not bypass that agent's limits.

The protected history retains an immutable decision for each decided request,
including a refused control result. A refusal cannot later become an acceptance
under the same request. Duplicate decisions return only after current auth and
freshness checks, without consuming another quota, hold or cancel attempt.
Pre-admission validation errors create no decision; their original recorded
economics/epoch/deadline still cannot change. Unknown append outcomes fence all
units until explicit reload and authenticated lookup; no automatic resend.

Order acceptance, unexposed cancellation (or bounded cancel preparation of an
already-exposed original), owner payout acceptance and leverage selection reuse
the existing financial engine. Cancels derive target unit/customer/market and
attempt ownership from accepted history, not caller hints. A recorded-only order
is not cancellable as an admitted order. Only accepted orders consume lifetime
grant budget, including after cancellation. READ remains unit-wide.

The journal admits one matching financial control per decision and retains real
facts even when controls fail. It cannot accept a financial control through the
foundation path. Trusted per-unit collateral/risk-policy installation is allowed
for offline composition; it is neither a public endpoint nor numerical policy
approval. There is no fallback to flat-cash admission when joined risk is absent.
Assets, holds, capital and restrictions remain component-local; the shared head
prevents stale concurrent decisions spending them twice. A fault in shared
storage/freshness fences all components.

## Version and limits

Unit replay revision 3 starts only new empty histories. Revisions 1/2 are refused;
no active-state migration or snapshot import. Shipping single-unit codecs,
financial equations, Runtime and v1 SDK remain unchanged. Native exposure/signing,
workers, API listeners, current lifecycle views and recovery composition are
separate gates, not implied by this controller's durable acceptance.

P31 review follow-up: before runtime/read wiring, measure repeated-request latency
at `wire::MAX_ITEMS`. If required, use head-keyed in-enclave derived indexes/cache
and incremental append validation, preserving freshness/current-head rechecks,
replay validation and shared fencing. Never introduce a mutable grant database.
This offline slice does not speculate about production caching or throughput.

## Acceptance

Test accepted/refused order, payout and leverage decisions, cancel ownership,
agent quota, recorded-only conversion, original expiry/revocation, retry after
response loss, CAS/unknown outcomes, encryption/replay and wrong-unit facts.
Exercise actual existing risk/hold controls with explicit synthetic policy;
fixtures are not native qualification, calibrated capital policy or an audit.
No live venue/RPC/AWS, frontend or deferred Pacifica-credit work is authorized.
