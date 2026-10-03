# P21A: confidential web API contract and transport proposal

Date: 2026-10-03. Product scope and bounded Noise NK qualification approved.
Status: implementation contract; new browser channel not yet implemented or qualified.
This is the implementation contract, not a published or implemented HTTP API.
Progress belongs in the tracked implementation tracker. No deployment is enabled.

## Boundary and method matrix

The parent routes opaque sessions. The enclave authenticates customers, decrypts
requests, authorizes each action and serves projections of the existing journal.
HTTP and WebSocket call the same handler. Neither is a new financial engine.
The current Node/TLS profile remains supported; web support does not weaken it.

| Method family | Caller | Result and authoritative source |
| --- | --- | --- |
| view / account | Owner or current READ agent | Exact settled cash, unsettled funding, holds, authority epoch and separately qualified valuation/margin; customer book, reservations and existing risk model |
| positions | Owner or current READ agent | Signed lots, signed remaining basis, selected leverage, qualified mark/PnL and freshness; own book and accepted configuration/evidence |
| operation / order / openOrders / orderHistory | Owner or current READ agent | Own request economics, durable outcome, actual fills and cancel/terminal status; API records plus order controller, not the pooled venue's list |
| fills | Owner or current READ agent | Attributed actual executions and fees, with private IDs and causal/time evidence; no ACK-to-fill conversion or suspense charged to the customer |
| funding | Owner or current READ agent | Recognition and cash settlement distinguished, accepted units and evidence coverage; no double earning or hypothetical absent-position funding |
| movements | Owner or current READ agent | Own qualified deposits, customer-paid fees and payouts, requested versus actually paid; pooled treasury transfers do not become fictitious per-user transfers |
| grants | Owner | Own current grants, expiry, permissions and accepted-order budget use; accepted API history and current epoch |
| myGrant | Current READ agent | Its own permissions/budget only; not a list of other keys or a new permission for non-READ agents |
| markets | Owner or current READ agent | Configured market/precision IDs, exact lot/tick conversions, supported order modes and policy caps; not public price/chart/orderbook data |
| order | Owner or scoped TRADE agent | One GTC/ALO/bounded IOC; retain price/fee/quantity/deadline/reduce-only limits and current risk admission |
| cancel | Owner or scoped CANCEL agent | Owner's order or that agent's original admitted order; independent durable cancel attempt, no inference that an ACK released fills/holds |
| payout | Owner only | Registered recipient, explicit net amount/fee/partial consent; beneficiary finality, not a source debit or HTTP success |
| grant / revoke / leverage | Owner only | Existing bounded READ/TRADE/CANCEL grant, epoch-wide revoke and capped private leverage selection; no native credentials or administrative delegation |

READ retains P18's own-account scope; the grant's trading-market restriction does
not silently redefine existing account-view permissions. Revocation checks precede
queries/retries and every stream emission. An owner may see its grants; an agent
cannot inspect another agent's key or budget. `myGrant` is a deliberately narrow
new READ query, not delegation or a financial permission. Self-service account creation,
selective revoke, triggers, modify/batch, spot and arbitrary transfers stay excluded.

## Selected local-qualification direction

The user approved qualifying **Noise_NK_25519_ChaChaPoly_SHA256** as a new web-channel
profile, using one pinned Rust implementation compiled for native and browser/WASM.
Do not handwrite ciphers or implement a second JavaScript handshake. This authorizes
bounded local qualification, not approval of an unaudited dependency for customers.

Browser WebSocket/WebCrypto APIs do not expose P19's TLS exporter. A parent's
HTTPS certificate and an owner signature therefore cannot replace its exact
connection binding. Reviewed application-layer encryption makes ordinary HTTPS/WSS
useful as delivery, without trusting it with private plaintext.

| Alternative | Disposition |
| --- | --- |
| Parent plaintext HTTPS/WSS plus wallet signatures | Reject: authenticity does not stop the parent reading orders or balances |
| TLS 1.3 inside a browser/WASM byte tunnel | Valid alternative worth reconsidering if a well-supported implementation meets the verifier/exporter profile; requires a new browser TLS/provider build, not wrapping `fetch` |
| HPKE to a boot key | Do not choose a one-shot encryption shortcut: HPKE alone is not a bidirectional authenticated session, nor forward-secret against later recipient-key compromise |
| Standard attested Noise NK | Recommended local qualification: known responder key comes from independent attestation, then a standard two-message handshake supplies the session |
| Keep P19 Node-only TLS | Safe fallback if the web-channel qualification fails, but does not satisfy approved browser support; report that gap rather than weakening privacy |

The earlier local experiment used Snow 0.10.0 and published-vector tests. It did
not test browsers or an attested application channel. Snow's maintainer explicitly
states it has not received a formal audit. Those results are provenance, not
acceptance or a selected production dependency. Confirm/pin the implementation,
features, dependency graph, entropy, zeroization and browser build in a follow-up
dependency decision before adding it to the shipping workspace.

### Proposed handshake safeguards

1. Client creates a fresh random challenge; no account, wallet, order or signature
   is sent. Enclave creates a boot-only responder key using qualified entropy.
2. A fresh NSM document binds that exact key, challenge, release/domain/manifest,
   profile, boot/session identity and expiry. The web quote has a distinct purpose
   from TLS, clock and KMS recipient quotes; an old TLS quote cannot authorize it.
3. The independent client validates bounded CBOR/COSE, AWS root/path/signatures,
   exact nondebug measurements, selected policy, freshness and key/context before
   accepting the responder. Policy/verifier code cannot come from the relay.
4. Both Noise handshake messages have empty application payloads. Complete the
   handshake and bidirectional key confirmation before private authentication;
   library support for early payloads is not permission to send identity early.
5. Domain/profile/challenge/quote context is committed in the canonical prologue;
   the finalized transcript supplies the P18 session binding. Signed requests also
   bind account, epoch, policy, expiry and immutable financial operation identity.
6. Each direction has one serialized cipher state. Replays, truncation, reflection,
   reordered records, expired/fenced sessions and authentication failures close
   the channel. Never restore counters or keys across restarts, resume a cached
   approval or retry an uncertain ciphertext. Reconnect freshly and query the
   original economic ID; an HTTP receipt is not a native receipt.

The follow-up implementation must publish exact byte encodings, purpose strings,
prologue/binding vectors and chosen verifier dependencies before this profile can
be marked reviewed/implemented. A remote Node verification service is not a browser
verifier: it would replace independent validation with trust in that service.
The current OpenSSL verifier cannot simply be compiled unchanged for browser WASM.
Its complete strict chain/time/strength/root policy needs equivalent browser
validation and differential negative fixtures; no weaker parser/path fallback.

## Proposed HTTP and WebSocket envelope

Use one private method dispatcher rather than account/market/operation IDs in URLs.
Proposed routes are `/v1/attestation`, `/v1/session`, `/v1/exchange` and `/v1/ws`.
Route names are not yet live. Attestation/handshake data is public and bounded;
the exchange and WebSocket application bodies are ciphertext.

The random outer session handle routes state but grants no account authority.
Do not put private IDs, signatures, tokens, error details or methods in HTTP
headers, query strings, cookies, URL paths, access logs or WebSocket subprotocols.
No bearer token may substitute for the enclave-verified owner/agent signature.
Outer errors only describe delivery/framing availability. Detailed authorization,
financial conflict and operation outcomes are encrypted replies.

Application method/version, response correlation, chunk identity and record type
are authenticated inside the channel. The public carrier cannot reinterpret a
query as a mutation, splice chunks or redirect a record into a different session.
Encode all financial quantities as exact integers; SDK objects use bigint and
documentation JSON examples use canonical decimal strings. Existing eight-command
signing/digest vectors stay unchanged; richer reads have explicitly versioned,
signed schemas. Do not guess new wire tags or silently change retained intent IDs.

One HTTP exchange is in flight per session. Ambiguous delivery or a missing reply
closes that session; callers reattest/reconcile rather than retransmit ciphertext.
Concurrent or reordered delivery must reject without another financial action.
WebSocket may interleave commands and server updates only through serialized
directional record queues; correlation IDs are not economic idempotency keys.

Enclave session state is ephemeral and bounded, not a separate replay/financial
database. Use the existing journal for accepted intents and exact retries.
Review server-framework/dependency choices separately from the crypto profile.
Parent routing is fixed to the selected enclave, not a caller-selected host/URL.

## Read provenance, bounds and stream semantics

Each result names its private snapshot/cursor, applicable authority/policy/unit
versions, qualified observation time and completeness/freshness. Missing/stale
evidence gives unavailable fields with reason, not zero. An accepted ledger is
authoritative about what was booked, not proof that an external account has not
changed since the last complete observation.

Separate accounting equity/unrealized PnL from conservative risk equity, initial
and maintenance margin, pending requirements and genuinely free collateral.
Positive unsettled funding may enter accounting equity but is excluded where the
existing conservative risk policy requires it. Immediately payable cash is not
free collateral. Do not publish pooled cash, house capital, pool flags, native IDs
or a withdrawable promise based on a historical snapshot.

Revalidate evidence age at the current qualified read clock. The existing
`risk_report()` uses the journal's logical cut; calling it on an old cut without
a read-time freshness check could display expired evidence as current. Add a
read-only qualified-time path through the same model, not a clock-only journal
mutation or a second margin formula. Freeze/unknown funding/causal gaps restrict
available derived fields; actual customer cash and retained facts remain visible
where trustworthy. Errors do not erase adverse history.

Paginate orders, positions and economic history with at most 64 items per page.
Bind opaque cursors to account, query/filter, authorized snapshot and revision;
never expose a global journal index or changes caused only by another customer.
Read and count only permitted data before choosing page boundaries. Old cursors
have an explicit stale/unavailable result, not a fabricated empty history.
History is complete only for the declared retained range. Do not truncate or
delete accepted economic/replay evidence to satisfy a display-page bound.

History timestamps distinguish qualified event time, observed time and committed
cut; do not fabricate venue time from API arrival. Customer event identity/order
comes from replayable attributed journal evidence, including operator-forced
events, not merely this user's API mutations. Treasury moves have pool ownership;
show user allocations only when the journal actually establishes them.

Private streams start with an authorized bounded snapshot and account-local
revision, then committed updates/events. A revision changes only when the
authorized projection changes; do not leak unrelated journal activity via gaps,
empty events or global counters. Bind subscriptions to signer, account, epoch,
permissions, expiry and current writer/freeze state. Recheck on every emission.
One bounded stream queue per connection; overflow terminates it with resync
required, never silent drops. Reconnect obtains a fresh snapshot/current query;
it does not replay a trade. Read capabilities stay read-only.

Initial budgets: eight active sessions, five-second handshake/exchange budget,
at most 128 client records and a 120-second attested session lifetime. Reattest
long-lived integrations; revalidate shorter request/grant expiry on every use.
Keep private commands at 1 KiB and complete replies at 1 MiB. Noise records have
their own smaller bound: split replies into authenticated bounded chunks with
one partial assembly per direction and an absolute timeout; no interleaved chunks,
unbounded allocation or partial result delivery. Proposed stream queues are capped
at 32 complete updates and 1 MiB total, whichever comes first. Exact framing and
caps must be tested before implementation acceptance, not inferred from a library.

## PR-sized implementation steps and acceptance

P21A remains in progress through all these steps; a contract-only PR cannot close it.

1. Contract + isolated core qualification. Record approved methods/permissions,
   retained signing semantics, candidate transport, dependency policy and local
   native/browser evidence. Existing-command SDK helpers join HTTP integration.
2. Transport qualification. Lock dependencies; run standard known-answer vectors,
   actual browser/WASM and native builds, strict quote/verifier equivalence,
   no-early-identity, key/transcript substitution, replay/reflection/order/fencing,
   bounds and private-record fragmentation tests. Stop if any property fails.
3. HTTP + shared service. Real offline browser/Node SDK → opaque relay → enclave
   fixture → current API/journal. Exercise one-shot delivery, lost replies,
   ambiguous commitment and process restart with unchanged operation IDs.
4. Authorized read projections + WebSocket. Add bounded schemas/paging/events,
   exact allocation/time/freshness, grant/self-view limits, revocation, snapshot
   race, overflow/resync and cross-account leakage tests. No native query forwarding.
5. Joined adversarial acceptance. Both transports exercise order/query/cancel/
   owner payout, fail closed on hostile parent/session/storage, preserve ordinary
   freeze and later recovery epoch semantics, and run offline on fresh CI checkout.

Mintlify follows implemented handlers and checked examples. P21 recovery,
P22 joined workflows and P23 changed-image/live qualification remain required;
this work adds no live trading, automatic recovery activation or public host.

## Qualification evidence boundary

The first PR-sized slice qualifies a shared native/browser Noise core in a separate
`tools/web-channel/` workspace. It has no financial, venue, wallet, AWS or shipping
service dependency. Synthetic responder-key trust is explicit; it is not fresh
Nitro attestation and cannot close P21A's transport criterion. The later independent
browser verifier, complete quote/transcript profile, session lifetime/fencing,
HTTP handlers, private projections and WebSocket subscriptions remain required.

Source review found no zeroizing Drop implementations for Snow 0.10.0's default
DH/cipher key arrays or chaining state. This does not invalidate a known-answer
vector, but prevents a claim of qualified secret erasure/forward secrecy under
process-memory reuse. No wrapper can promise to erase opaque library internals.
Record this before shipping: choose/review a hardened implementation or a narrowly
reviewed dependency change; do not silently fork cryptographic code or assert
that dropping a Rust/WASM object wiped its state.

## Primary sources and evidence limits

Checked 2026-10-03:

- [WHATWG WebSockets interface](https://websockets.spec.whatwg.org/#the-websocket-interface)
  and [W3C WebCrypto](https://www.w3.org/TR/webcrypto/): browser delivery/crypto APIs,
  not P19's TLS exporter/socket-binding interface.
- [Noise revision 34](https://noiseprotocol.org/noise.html): NK handshake, prologue,
  transport states and message limits. This does not certify Cinder's profile.
- [Snow 0.10.0 API](https://docs.rs/snow/0.10.0/snow/) and
  [maintainer README](https://github.com/mcginty/snow): candidate features and
  explicit lack of formal audit; no production library selection implied.
- [RFC 9180](https://www.rfc-editor.org/rfc/rfc9180.html#section-9.7): HPKE's
  security limits; a session needs additional protocol design.
- [AWS validation profile](https://docs.aws.amazon.com/enclaves/latest/user/verify-root.html):
  certificate/COSE/measurement validation and application-bound attestation.

The approved local follow-up adds [the isolated qualification tool](../../tools/web-channel/README.md)
with candidate Snow 0.10.0, a shared known-answer test and native/browser hostile
wire checks. Those results are bounded tests, not a formal proof or fresh Nitro
attestation. No AWS resource, live venue, wallet or shipping graph was used/changed.
Existing experiments are provenance, not new acceptance evidence.
