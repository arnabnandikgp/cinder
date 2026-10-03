# Customer API scope and HyperLink comparison

Reviewed 2026-10-03 against Cinder's current source and HyperLink's public docs.
The user approved all scope recommendations below on 2026-10-03, including the
bounded read extensions and scoped agents. This is not an implemented HTTP API or
a promise of full native venue compatibility. The approved sequence is API
implementation, public docs, then P21 recovery integration. Exact schemas and
browser transport security still need full qualification. [ADR 0021](../architecture/0021-confidential-web-api.md)
records the approved bounded Noise NK/Rust-WASM qualification; it does not approve
shipping candidate cryptography or expose an endpoint. [PLAN](PLAN.md) and [TRACKER](TRACKER.md)
own progress; existing financial/authority rules remain in [BASELINE](BASELINE.md).

## What actually exists

[ADR 0018](../architecture/0018-private-api.md),
[Rust commands](../../crates/api/src/wire.rs),
[Rust replies](../../crates/api/src/lib.rs) and the
[TypeScript client](../../clients/private/src/index.ts) establish eight commands:

| Command | Implemented semantics / important limit |
| --- | --- |
| View | Own cash, recognized unsettled funding, explicit holds, signed positions/basis and retained operation IDs. Not a full portfolio, margin or withdrawability report. |
| Operation | Own durable operation receipt, including acceptance, possible exposure, ACK, actual partial fill/payment, completion or unknown. No complete fill/history response. |
| Order | One bounded GTC/ALO/IOC order; signed lots, price/fee limits, expiry and private-user reduce-only. No batch, trigger or modify path. |
| Cancel | Own original private order ID plus a distinct durable cancel attempt. Cancel ACK is not proof of terminal order state. |
| Payout | Owner-only request to the registered token account, with explicit net amount, extra fee cap and partial consent. No arbitrary destination or agent money movement. |
| Grant | Owner-approved READ/TRADE/CANCEL agent, one market, expiry and lot/fee/order-count limits. No native venue credentials, leverage-setting or payout permission. |
| Revoke | Owner advances the account epoch and revokes all old grants. Selective per-agent revocation is not currently implemented. |
| Leverage | Owner selects private leverage under the configured market cap; this is not a switch to a native isolated-margin account. |

P19/P20 supply the attested Node transport and confidential service. P21A's HTTP
slice adds browser/Node confidential HTTP for these eight commands, tested offline
against the same handler/journal. The final slice adds WebSocket entry/subscriptions
and bounded richer reads; [the exact contract](../architecture/private-read-contract.md)
states time/projection/completeness limits. Current deployment qualification disables native trading/funding/reads;
implemented application semantics do not establish a live brokerage service.
Account/owner/recipient bindings are currently governed configuration, not a
self-service account-creation API. Wallet connection alone does not register a
new customer; public examples must identify preconfigured integration accounts.

## Documented HyperLink surface versus Cinder

The comparison below covers the 32 method names in HyperLink's current
[documentation index](https://docs.hyperlink.xyz/llms.txt). Their names are not
32 distinct routes: actions and private queries use its shared
[POST /exchange](https://docs.hyperlink.xyz/api/exchange-methods).
Documentation is interface evidence, not independent execution/security testing.

| Family | HyperLink documented methods | Cinder / recommended disposition |
| --- | --- | --- |
| Place orders | order | Core single limit/post-only/bounded IOC semantics exist. Keep this phase focused on them; do not represent acceptance as a venue fill. |
| Cancel / identifiers | cancel, cancelByCloid | Stable private request IDs already support customer-directed cancellation and reconciliation. Do not expose pooled native order IDs as customer identity. |
| Replace / batch | modify, batchModify | Missing execution semantics. Later reviewed phase, not a transport alias; cancel/new is not atomic replacement. HyperLink itself documents restricted post-only replacements and independent batch-leg outcomes. |
| Leverage | updateLeverage | Private bounded selection exists; native margin-mode parity does not. |
| Isolated margin | updateIsolatedMargin, topUpIsolatedOnlyMargin | Not part of the current account architecture. A customer-facing margin-isolation product requires a separate financial/risk design. |
| Withdraw | withdraw | Owner-bound payout intent exists through Cinder's Solana custody route, not HyperCore transfers. Retrieval, fees, finality and recipient restrictions must be documented honestly. |
| Agents | approveAgent, extraAgents | Bounded grant and global revoke exist. Recommend an authorized active-grant/budget view; selective revoke, multi-market grants and IP allowlists are separate enhancements. |
| Perp state / asset context | clearinghouseState, activeAssetData | Current View is narrower. Recommend own positions, exact cash/funding/basis, marked PnL/equity and qualified margin/availability context, with freshness and explicit unavailable states. |
| Order views | openOrders, userHistoricalOrders | Current snapshot gives IDs and operation gives aggregate lifecycle. Recommend bounded attributed open-order details and paginated historical operations. |
| Economic history | userFills, userFillsByTime, userFunding, userNonFundingLedgerUpdates | Recommend paginated attributed fills/fees, funding and deposit/transfer/payout history from accepted journal evidence. Cash/funding totals are not a substitute for events. |
| Analytics / limits | portfolio, userRateLimit | Long-range PnL charts/analytics are later work. Expose documented client/session limits first; account budget queries must not leak pooled native usage or other customers' activity. |
| Builder economics | approveBuilderFee, claimRewards, maxBuilderFee, approvedBuilders | No builder-fee or reward product exists. Do not copy optional fee permissions or introduce new claim ownership in a transport milestone. |
| Referrals | setReferrer, registerReferrer, referral | No referral/access-reward product exists. Later commercial work if selected. |
| Spot / transfers | spotClearinghouseState, usdClassTransfer, sendAsset, agentSendAsset | Spot, class transfers and arbitrary/agent transfers are outside Cinder's current perp-only authority contract. Solana deposit instructions are not an API that mints credit or moves money based on a client assertion. |

HyperLink's [order schema](https://docs.hyperlink.xyz/api/exchange-methods/order)
includes trigger orders; Cinder does not yet have a durable TP/SL trigger engine.
This is a meaningful usability/protection gap, not something to hide behind a
market/limit demo. A future trigger design needs price-source/freshness policy,
durable activation, reservations, reduce-only behavior and restart/reconciliation
tests. A client bot is not an always-on server-side stop guarantee.

Its [Pro order documentation](https://docs.hyperlink.xyz/trade/trading/pro-order-types)
also distinguishes front-end execution algorithms from core server methods:
Scale submits a set upfront; the other listed algorithms run in the browser and
can be interrupted when the tab closes. We need not build seven server schedulers
to match a menu. TWAP/DCA/iceberg strategies may later use Cinder's bounded agent
SDK, with their limits and liveness stated explicitly.

## Approved first release of the transport surface

These scope recommendations are approved; they do not select a cryptographic suite
or imply that the listed extensions are already implemented:

1. Preserve the existing six mutations and two reads. Add ergonomic SDK helpers,
   exact signed decimal/unit handling and the same immutable operation identity
   across HTTP and WebSocket. An SDK market-order helper can construct a bounded
   IOC; never introduce an unbounded market order or assume it fully fills.
2. Make the read side usable: own account/positions, order details and history,
   fills/fees, funding, money movements and active agent permissions/budgets.
   Include configured market units, tick/lot precision, supported order modes and
   policy caps so SDK clients can construct valid actions; this is metadata, not
   a public chart/orderbook feed. The owner may inspect its grants; an agent's
   permission/budget view must not become a directory of other agents or customers.
   Specify finite pages, retention, time conventions, stable account-local cursors,
   observation completeness and freshness. Distinguish actual fills, cash postings,
   funding recognition/settlement, reservations and beneficiary payments.
3. Derive valuation/margin displays through the existing financial and risk model,
   not another front-end calculator. Missing/stale evidence is unavailable, not zero.
   Account equity, free collateral and immediately payable cash remain distinct;
   a snapshot is not a spend permit or a guaranteed withdrawal quote. Do not expose
   house capital, other customers, native account IDs or aggregate venue history.
4. Stream committed account/operation updates and the selected event projections;
   define initial snapshot, account-local ordering, finite queues, gap detection,
   resynchronization and current permissions on every path. Request correlation is
   not an economic idempotency key. Subscribe/reconnect must never repeat a trade.
5. Keep public market-data charts/orderbooks outside this private trading milestone.
   HyperLink's [WebSocket API](https://docs.hyperlink.xyz/api/websocket) likewise
   documents private channels, not public market-data feeds. A future terminal can
   use venue market data separately without handing private orders to the parent.

Read-only expansion is still real protocol work: signed query/schema versions,
owner/READ permissions, projection provenance, paging, leakage and restart tests.
Do not simply forward the pooled account's native queries. If the implementation
review finds a read requires an unapproved financial meaning or new collection
mechanism, narrow its fields or defer it explicitly rather than guessing.

## Build sequence and review boundary

API + Mintlify now precede P21, but cannot weaken its eventual recovery contract.
Before coding, finalize a method/permission/response matrix and a transport ADR:
browser-verifiable enclave attestation, fresh session/key binding, encryption,
signature encoding, errors, protocol versioning and finite resource limits.
This comparison itself selects no live route. The later user approval authorizes
local qualification of the Noise NK profile in ADR 0021, not production release.
Existing Node TLS exporter binding cannot simply be read by browser JavaScript.
TLS at an ordinary host plus a signature does not meet the parent-confidentiality
contract; sensitive request/reply decryption stays in the enclave.

Use reviewable steps: contract/attestation review; HTTP + browser/Node SDK;
authorized read projections + WebSocket commands/streams; adversarial process
tests; then Mintlify from the implemented contracts and checked examples.
At scope freeze, split P21A into separately tracked PR-sized milestones if needed;
do not hide a large new history/trigger engine inside a transport-only label.

Keep freeze/writer epoch, current session authority and recovery state transitions
compatible with the authoritative journal. Recovery can later shut down ordinary
mutations and distribute claims without any normal trading endpoint. Never add
a placeholder HTTP action that activates recovery or claims assets autonomously.

Public docs describe actual supported environments and handlers. Recovery pages
must distinguish P17's existing program from P21's not-yet-integrated workflow.
Native venue qualification, end-to-end evidence, production policy and customer
release remain P22/P23/P24 and G01–G04 gates. Documentation publication is not a
production launch or proof of unconditional exits/complete solvency.

## Approved scope decisions

| Decision | Approved recommendation / consequence |
| --- | --- |
| Bare eight-command transport or usable account API? | Add the bounded read projections above. More projection/paging tests, but useful for real integrations and a future terminal; no new trading or custody powers. |
| Triggers, modify and batches now? | Defer to explicitly tracked execution milestones, with TP/SL prioritized for a broader trading release. Initial API is deliberately less feature-rich; never advertise parity. |
| Copy native/HyperLink endpoints? | Preserve familiar strategy concepts with a secure Cinder SDK. Do not claim URL-only compatibility or inherit unrelated spot/builder/referral semantics. |
| Change agent authority to resemble HyperLink? | Keep current scoped grants, owner-only payout/leverage and epoch-wide revoke. Document these restrictions; selective revoke/portfolio grants need a separate authorization change. |

Primary-source links were checked on 2026-10-03. The explorer/web reader could
not load the site, but its public GitBook Markdown URLs and llms.txt were fetched
directly. No authenticated request, live trade or independent HyperLink audit
was performed in this comparison.
