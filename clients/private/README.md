# Private Cinder client

Thin semantic SDK for P18: private snapshots/operations, GTC/ALO/bounded IOC,
attributed cancellation, owner payouts, scoped grants and private leverage.
Amounts are bigint lots/ticks/quote atoms; account/market grids come from the
approved deployment policy. It is not a Pacifica URL-only proxy or native signer.

Inject a wallet/agent `signMessage` and a **trusted confidential channel**. There
is no shipped HTTP/WebSocket/attestation implementation or plaintext fallback.
P19 supplies the qualified channel. A caller-created object with matching fields
does not establish confidentiality. See [ADR 0018](../../docs/architecture/0018-private-api.md).

```ts
const client = new PrivateClient(qualifiedChannel, walletSigner, {
  domain: approvedDomain, account: privateAccountId, policy: approvedPolicy,
});
const result = await client.request({
  id: stableRequestId, epoch: currentAccountEpoch, expiresAt: authExpiry,
  command: { kind: 'order', market: approvedMarketId, lots: 2n,
    minimum: 90n, maximum: 110n, fee: 1n, tif: 'IOC',
    reduceOnly: false, goodUntil: financialExpiry },
});
```

Those numbers illustrate encoding, not Pacifica grids/fees or a production policy.
Keep the operation ID and exact command stable. On timeout query that original ID;
do not create a fresh order or native nonce. Reconnect uses a fresh channel-bound
signature. Current epoch/grant permissions always govern access.

`accepted` is durable admission, `acknowledged` is not a fill, `partial` reports
actual progress, and `unknown` requires reconciliation. Payout `net` excludes its
additional fee cap; `paid` requires beneficiary receipts. `feeCap` is not charged
order fees. `cash`, `funding`, explicit `held` and position basis are distinct; none
alone is withdrawable equity. Mutations return protected typed errors on known
failure; an unknown transport outcome throws a redacted reconciliation warning.

No automatic retry, public account dump, alternate payout recipient, agent payout,
raw cancel-all, batch/modify/admin endpoint, website or network subscription ships.
Grants currently permit one market/key, bounded accepted-order counts and scoped
permissions. Revocation invalidates all account grants, not an escaped venue order.

After installing pinned Node 24.21.0: `npm ci --ignore-scripts`, then
`node ../../scripts/check-private-client.mjs`. Tests are offline and use synthetic
keys/public wire vectors, not configured wallets. Runtime npm dependencies: none.
