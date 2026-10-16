# ADR 0040 — Component-owned native execution and observations

Date: 2026-10-16. P34 extends [ADR 0039](0039-unit-service.md). Offline
implementation does not activate a venue, deploy a listener or qualify an image.

## One sealed controller

`journal::Controller` reuses the legacy financial controller interface. Its
sealed `Source` prevents construction from a host snapshot. `units::Selected`
borrows one existing component of the shared journal, preserves the common head,
and exposes only that component's original transactions/receipts. It neither
creates a child database nor permits retargeting a capability. Successful unique
exposure commits return one delivery; unknown, duplicate and refused commits do
not. An ordinary order requires the same unit's original accepted API decision,
not a generic reservation/preparation. Replay revision **4** starts new empty
histories; revisions 1–3 are refused without migration. The shipping engine and
v1 API are unchanged.

Pacifica execution/reads/observations use this sealed boundary without replacing
their semantics. Unit use additionally binds the exact native account, origin,
configuration and non-master agent. Trusted unit Runtime workers hold the one
writer only for preparation/completion; TLS I/O runs after the borrow ends.

## BULK execution

The opt-in gateway requires an explicit qualified profile and allocated request
budget, an enclave-owned **agent** seed, immutable unit/source/environment,
market grids and exact native-to-kernel notional conversion. Native account and
subaccount parent master seeds are refused. Local restrictions do not narrow an
agent's actual upstream powers; deployment qualification must establish these.
No seed loader, arbitrary signing, master fallback or automatic nonce exists.

The original financially admitted order/cancel, attempt, exact abstract/native
bytes, deterministic native order ID, nonce, key epoch and budget are persisted
before signature delivery. Nonces/capacity survive restart and key rotation.
The raw native signature has **no protocol-enforced expiry**: local dispatch
expiry prevents late local writes but cannot revoke an escaped signature.
Actual upstream revocation remains a separate recovery obligation.

GTC/ALO/bounded IOC and cancel use the existing native codec. Per-user reduction
is not marked pooled-native reduce-only. Cancel derives the original native ID;
its ACK neither releases holds nor proves all fills arrived. Order ACK—including
`filled`—does not post an execution. Unknown/429/error replies retain uncertainty,
with bounded persisted cooldown and no implicit resend.

## Read capacity and financial causality

Unsigned POST account queries use the configured native account and fixed
family, at most 128 rows, explicit first-page slot bounds where supplied, and
only a cursor retained in a successful same-family response. Caller-provided
cursors/URLs/accounts are not accepted. Query capacity commits before egress,
shares the unit's allocated order budget, and preserves cleanup capacity. Unit
budgets must partition the qualified shared-IP allowance; each unit cannot be
given a copy of the whole allowance. Unknown reservation outcomes are not free
capacity. Non-200 bodies cannot normalize facts or supply continuation.

Original bounded response bytes are retained once, with compact references for
normalized inputs. Qualified ordinary fills/fees/rebates use original order
attribution and exact integer grids, ingesting real facts despite frozen
admission or local key revocation. Duplicate/conflicting event identities use
the existing engine's handling. Unknown targets remain unattributed.
Snapshots are diagnostics, never cash setters. Funding/risk histories remain
unresolved until rate/boundary/ADL-cause semantics are qualified. REST pagination
is not a global financial frontier, terminal-order proof or solvency proof.

## Egress and evidence limits

The optional BULK egress supports the **paper-testnet origin only**, fixed
`/api/v1/order` and `/api/v1/account`, pinned TLS root/hostname, qualified clock,
one-shot bounded framing and no redirects/retries. Parent `bulk-testnet` route
copies ciphertext to the fixed host; it has no signature or payload parsing.
Construction is not permission to open sockets or perform a financial run.

Native fixtures verify actual Ed25519 wire signatures, exposure/restart,
wrong-unit/environment/master refusal, ACK versus fill, late/duplicate fills,
fees, reserved cancellation capacity, exact reads, HTTP errors and durable
pagination/budget/expiry. Trusted API-header fixtures are not actual customer
signatures; joined signed private API acceptance belongs to P36. Synthetic
SQLite/protection and policy qualification are not live evidence. The two
approved unsigned public metadata GETs returned origin 502 and add no capability
qualification. Funding/returns and operator-assisted recovery compose in P35;
unsupported native rails stay disabled.
