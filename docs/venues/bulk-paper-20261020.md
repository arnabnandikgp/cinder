# BULK disposable paper lifecycle — 2026-10-20

This is native **paper-interface evidence**, not an activated Cinder deployment,
custody round trip or mainnet qualification. The user approved proposal A and
routine no-real-funds continuation. No existing wallet, transfer, Solana RPC,
AWS, frontend or customer/mainnet funds were used.

## Source and execution receipt

One fresh owner and two agents were generated locally. Cinder's `wire::Action`
encoded limit/cancel messages and computed original order IDs. Node's standard
Ed25519 implementation signed those bytes. Setup used documented faucet/agent
layouts, signature domain 2 and fixed `https://exchange-api.bulk.trade` only.
No SDK/native addon was installed. Disposable local keys are not a production
custody design or permission to move confidential keys out of the enclave.

Private evidence: `work/venues/bulk/paper-20261020.QnlKbJ`, ignored and optional
for checkout/CI. Action/nonce/cap consumption precedes signing; private signed
wires precede transmission; raw responses precede interpretation. All original
response hashes/lengths were independently checked. The run is complete: do not
resume it with a changed source/codec.

- Recorded start/result: 12:56:26.817–13:04:07.219 UTC.
- Source/binary commitment: `883a3a4cda2a967bd7b228dce1a0ab2fd99d5ac8e034c6984204176ce341d5c6`.
- Manifest file SHA-256: `09c17dec18a01fd4be5cb4b701b1dde16981fc9730caffdd86145364adbed59b`.
- Result file SHA-256: `d829df0cbd3abd7e3f3cc199dd1404b78e0e908acf4d1d15ceab53ee54001d4b`.
- **12 signed + 14 account + 3 public requests; four order attempts; $6.40
  cumulative limit-price order notional**, including cleanup. Caps remained
  14/24/12 requests, five attempts, $20 notional and 30 minutes.
- No unknown transport result, replacement original or blind resend. All HTTP
  responses were 200 except fee-tier 404. Unauthorized actions returned HTTP 200
  with semantic failure, not successful execution.

## Observed results

| Check | Result | Limit |
| --- | --- | --- |
| Paper funding | Faucet returned 1,000 USD; account independently reported 1,000 | Not an SPL deposit, bridge receipt or real capital |
| Two agents | Both registrations visible; both removed; each revoked agent's later signed cancel rejected as `unauthorized signer` | Funds/admin powers, subaccounts and propagation latency not exhaustively tested |
| Order identity | All four native limit statuses match Cinder's computed IDs | Not a Cinder SDK/enclave financial-admission run |
| Actual open/close | Full maker fills: buy 0.00002 BTC at 100,000, sell at 80,000 | Functional test, not execution-price/latency benchmark |
| Bounded IOC | Close IOC at 70,000 cancelled with zero fill; original position remained | No IOC-fill or partial-fill coverage |
| Cleanup | Separate closing ALO filled; account reported flat | Known cleanup action, not resend of uncertain execution |
| Cancellation | New acknowledged ALO cancelled by original ID; history confirms cancelled; no position resulted | Cancel ACK is not global financial completeness |
| Final state | Zero positions, orders and agents; margin 999.6, realized PnL -0.4; fees/funding zero | Residual faucet balance is disposable, not returned real funds |
| Book reads | Both sampled snapshots empty; later native fills occurred | Empty sampled depth does not guarantee no future execution |
| Account fee tier | Documented `feeTier` query returned empty HTTP 404 | Unavailable, not zero fees or proof of fee aggregation/savings |

Native fills' zero effective fees and exact arithmetic
`0.00002 * (80000 - 100000) = -0.4 USD` agree with final 999.6 paper margin.
This does not qualify nonzero fee precision, funding, ADL or capital sufficiency.

## Source-backed compatibility correction

Both fill rows omit `tradeId`, retaining native `slot`/`sequence`. Cinder initially
refused the unchanged bytes; the observed-shape regression reproduced that failure.
The [official SDK's HistoryFill deserializer](https://github.com/Bulk-trade/bulk-client/blob/5012611bd0fd36e12523cd15bac763f3a600b1e9/crates/api-rust/src/msgs/history.rs)
explicitly uses the required native pair when the field is **absent**, rejecting
explicit null. This also exists in the retained SDK pin. Exact upstream file
independently fetched October 20, SHA-256:
`6387851fde317239633a86cdda827219f8cfd28d4d0816ac1a0c760197bd9b55`.

Cinder now follows that alternate representation. Explicit IDs must remain
canonical and match the required pair. Missing/null/invalid/out-of-range native
fields, wrong accounts and isolation still fail. Both spellings share the same
scoped dedup key; conflicting economics remain conflicts. No timestamp, order
hash, guessed sequence or rewritten response supplies identity. Invalid records
remain archived without posting. History coverage was `unknown`, not promoted.

Offline re-decoding of unchanged native bytes after rebuild successfully linked
both fills to their original orders, net zero quantity and -0.4 USD diagnostic
PnL. No additional network request. Decode receipt SHA-256:
`8a5efddac2f0217d09be8fbbd7d0eaae1899e41f5f52545684f1b752ba9b436d`.

| Original evidence | SHA-256 |
| --- | --- |
| Two-fill response 24, 1,051 bytes | `79bc1ec35f52b84bc54623e23302132aa7a4f3222cd4be28774224a34aa5599c` |
| Order history 25, 1,380 bytes | `fe07a0d721f0f212c64138aa3f6b4e4cf1038c1ebd6b85169dc9ea986a6eca9e` |
| Final account 29, 1,538 bytes | `7308caea52c9f988dce2b0cd7bc686cace56aa4584e97890ad28dd2b488bf91a` |

The tracked sanitized fixture retains native numbers/omissions and replaces
account/order identities. CI uses offline fixtures, never the live runner.

## Remaining boundary and next action

This supplies bounded native agent/raw-signing/order/cancel/open-close evidence,
not `Gateway::Level::Qualified`, funding readiness, history completeness or
shipping API v2 activation. Nonzero fees/rebates, partial/IOC fills, funding/ADL
and complete forced-event causality remain separate evidence targets. Fee-tier
404 remains unavailable, not retried against another host or treated as success.

Next: prepare the non-fixture multi-unit role/egress/storage configuration using
these constraints; qualify its changed Nitro image separately before key release.
BULK supported test funding/return rails and operation-linked credit remain
unestablished; controllers stay disabled. Pacifica missing credit stays deferred.
See [scope/questions](bulk-native-qualification.md) and
[qualification sequence](../implementation/UNIT-QUALIFICATION-PROPOSAL.md).
