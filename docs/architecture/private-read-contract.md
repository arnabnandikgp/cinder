# Private reads and WebSocket delivery, version 1

This is the final P21A offline implementation contract, extending
[P18 signing](0018-private-api.md) and [the attested web profile](0021-confidential-web-api.md).
It adds no financial policy, onboarding, native pooled-account query, recovery
action or live deployment. Changed-image Nitro/native workflows remain P23.

## Signed reads

The original eight tags/bytes remain unchanged. `Read` uses tag 8, read version 1,
family u8, cursor[40], limit u16 BE, within the SAME signed canonical envelope.
Owners and live READ agents query only their bound private account under the
current epoch, policy, expiry and verified channel. Authentication and independent
journal freshness precede projections/cursors. Reads commit nothing or consume
trading budgets. No view is a spend permit or proof of external completeness.

Reply: existing reply prefix/version, tag 3, read version 1, family u8,
revision[32], next[40], evaluatedAt u64, row count u64, then u32-length-prefixed
rows. All integers are BE. Money/basis/payment use signed i128 quote atoms, lots
signed i64, prices u64 ticks; strict Option tags are 0/1 then value. Market unit
is market ID[32] + precision VERSION u32, not a decimal-place count. Requests are
<=1 KiB, replies <=1 MiB, requested page limits 1..64 rows; empty pages are valid.
SDK quantities remain bigint.

| Family / tag | Own-account row / boundary |
| --- | --- |
| account / 0 | Epoch, cash, recognized unsettled funding, explicit held amount. Optional existing-risk-engine equity/initial/maintenance/pending-outcome requirement/other holds/free; optional marked accounting equity. Conservative risk equity, accounting equity and immediately payable cash differ. |
| positions / 1 | Configured market unit, signed lots/basis, optional qualified mark/PnL, optional policy leverage cap/selection. Includes flat configured markets; no new isolated-margin product. |
| operations / 2 | Qualified transaction observation time, original command economics and current durable receipt. READ agents cannot see owner grant commands containing other agent keys. Retained API history, not native history. |
| openOrders / 3 | Order operation rows not complete/rejected. Unknown/cancel-pending orders stay visible; cancel ACK does not prove no further fills. |
| fills / 4 | Private original request ID, actual attributed ordinary-fill unit/lots/ticks/fee and optional trusted observation time. Successfully applied customer-classified inputs only, deduplicated; no suspense, rejected evidence, ACK or requested quantity. |
| funding / 5 | Unit, frozen own eligible lots, optional recognized payment, boundary-settlement flag and optional boundary observation time. Unknown differs from zero. Settlement may precede private allocation evidence; the flag alone is not proof of cash credit. |
| movements / 6 | Qualified transaction observation time, exact own book before/after (cash/funding/all position lots/basis). Includes operator-ingested/forced changes, fees/funding/receipts. Compound transitions are not mislabeled as one deposit. Holds or pool treasury transfers without a customer book change are not cash postings. |
| grants / 7 | Active current-epoch terms and accepted-order budget usage. Owner sees active grants; an agent sees only its own. No expired/revoked grants, other-agent directory or native credentials. |
| markets / 8 | Configured unit/quote precision versions, exact quote-atom-per-lot-times-tick ratio, supported GTC/ALO/IOC mask 7, optional policy cap. Governed kernel metadata, not instrument labels, public feeds or an invented native decimal grid. Native/display mappings still come from approved deployment configuration. |
| updates / 9 | Replacement snapshot: discriminator 2 account row, 0 configured position row, 1 current receipt. Selected economic histories are separately subscribable families. No native IDs/house capital/global head. |

Forced liquidation/ADL/restoration book effects are in movements/current positions;
ordinary fills do not claim a complete source-specific forced-fill allocation
history. New detailed forced-fill rows require a qualified schema. Accepted payout
intent is not money paid: only accepted beneficiary evidence reduces the claim.

Derived risk/valuation use the existing model at evaluatedAt. Missing/unqualified/
stale inputs produce absent fields, never zero or silently refreshed marks.
`committedAt` and `observedAt` name qualified Cinder transaction/observation clocks,
not fabricated venue execution timestamps or block times.

## Paging and provenance

Zero cursor starts a snapshot. Nonzero cursor is SHA-256 revision[32] plus next
permitted-row offset u64. The revision binds all permitted rows, domain/account/
signer/epoch/policy/family, excluding current read time, unrelated accounts and
global journal sequencing. Filter before paging. Changed rows return encrypted
conflict: restart at zero rather than splice versions. Zero next means no more
retained rows, not complete external history. Existing journal limits are 4096
records / 64 MiB; exhaustion contains, never silently prunes evidence.

Money-history indexing is immutable, derived from pre/post authoritative books
on successful commit and verified replay. It adds no durable format, accounting
state or editable balancing ledger. Failed append/duplicates cannot add rows.
Fills/funding project accepted classified ledger evidence. Restart rebuilds the
same index, and live witness checks still precede access to cached private data.

## WebSocket carrier and commands

The SDK obtains fresh independent AWS verification and completes the SAME HTTP
Noise NK handshake/confirmation first. `transport: 'websocket'` then upgrades
`/v1/ws` at the same origin and sends the confirmed public handle[32]. No private
account/method/token/signature enters URL/headers. Parent forbids Authorization/
Cookie, text, subprotocols, extensions and caller-selected upstreams. Browser
Origin must match configured origin; use a cookie-free API origin. Production
requires HTTPS/WSS; HTTP/WS is allowed only at loopback for offline fixtures.

Subsequent masked binary messages <=1046 bytes carry Noise records with
authenticated [version 1, delivery type, sequence u32, signed request]. Type 5
is a socket command, 3 one Read subscription with zero cursor, 1 HTTP. Sequence
starts at 1 after confirmation and increases exactly. HTTP cannot mix after
socket mode. Mutation disguised as subscription refuses before the handler.
Correlation is not an economic ID or authority. All commands reach the same
Session/Handler, reservations, permissions, witnessed journal and durable IDs.

Server binary message: public correlation u32 + ordinary complete encrypted
chunk batch (<=1,050,790 bytes including correlation). Encrypted chunk headers
independently authenticate correlation. One cipher/handler writer, at most one
queued ingress frame, no interleaved chunks or partial plaintext delivery. A
complete final response flushes before orderly close; hostile input aborts.

## Subscription and resync

`PrivateClient.subscribe({id,epoch,expiresAt,query})` signs Read once. Every 500 ms,
independent of command traffic, service rechecks that exact query's current expiry,
signer/READ grant, epoch, mode and witnessed freshness. Initial replacement page
arrives immediately; later pages emit only when the permitted revision changes.
This coalesces state, not every intermediate transition. Selected history retains
intervening changes for explicit paging. Unrelated commits produce no global gaps,
empty updates or counters. Read capabilities never authorize mutations.

Private notification: `CINDER-PRIVATE-UPDATE-1\0`, ordinal u64 starting at 1, encoded
API reply. Correlation is the subscription sequence. Ordinals increase exactly;
counter/correlation/ordinal gaps, malformed replies or encrypted errors terminate
with resync required. No unsigned subscribe, native channel forwarding, resumed
cipher counters or automatic mutation retry. Returning the iterator closes the
connection. One consumer only; another iterator fails closed.

Bounds: eight attested sessions/upgraded sockets; 120-second attested session;
128 authenticated records/direction INCLUDING confirmation, chunks and updates;
five-second handshake/command/initial-subscription and absolute partial-frame
delivery; one subscription and one in-flight SDK command; 32 complete queued
notifications / 1 MiB, whichever comes first. Overflow closes/clears, never silently
drops and continues. Parent assembly caps: 1046 bytes/64 fragments, 16-KiB scratch,
256 control frames. Slow writes have a bounded delivery deadline.

After close/expiry/gap/overflow/restart: freshly attest, sign under current epoch,
fetch zero-cursor replacement, page until complete/conflict, reconcile uncertain
economic IDs. Resubscribe cannot repeat a trade/payout. Scratch cleanup/free is
NOT proof of erasing Snow internals/browser copies; ADR 0021's release limit stays.

## Offline acceptance

`node scripts/check.mjs`: API/journal/history, carrier, service and existing SDK/TLS
regressions. `node tools/web-channel/check.mjs`: shared native/WASM core, independent
verifier, actual Node/Chrome SDK → opaque relay → separate Rust service → protected
witnessed journal. Both transports run all eight original commands/ten read
families; stream cases cover initial/correlated pages, other-ingress commits,
pagination, agent privacy/revoke, idle expiry, missing delivery/fresh reconciliation,
real slow-consumer overflow and witness loss. Rust tests cover deduplicated actual
fills, unknown/zero/settled funding and exact restart projections.

Keys, quote roots, balances and input events are disposable synthetic fixtures.
Process fixtures dispatch intents but perform no actual venue trade/payment.
Carrier/log/disk marker checks are bounded evidence, not mathematical privacy
proof, independent audit or fresh Nitro qualification. P21/P22/P23 remain separate.
