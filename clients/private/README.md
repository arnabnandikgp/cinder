# Private Cinder client

Thin semantic SDK for P18: private snapshots/operations, GTC/ALO/bounded IOC,
attributed cancellation, owner payouts, scoped grants and private leverage.
Amounts are bigint lots/ticks/quote atoms; account/market grids come from the
approved deployment policy. It is not a Pacifica URL-only proxy or native signer.

Inject wallet/agent `signMessage` and a **trusted confidential channel**. P19 adds
the Node automation profile below; the semantic client still supports a separately
qualified channel port. A caller-created object with matching fields is not proof
of confidentiality. No plaintext fallback or browser HTTP/WebSocket transport is
provided. See [ADR 0018](../../docs/architecture/0018-private-api.md).

```ts
import { AttestedNodeChannel, nativeQuoteVerifier } from './src/node-channel.ts';
const qualifiedChannel = await AttestedNodeChannel.connect({
  host: configuredHost, port: configuredPort, policy: independentlyApprovedRelease,
  verifier: nativeQuoteVerifier(independentlyInstalledVerifierAbsolutePath),
});
```

Release policy binds network, deployment, manifest and PCR0/1/2. The independent
`cinder-verify-quote` executable accepts only the pinned AWS root; obtain/build it
from a reviewed client distribution, never a relay-provided download/path. The SDK
checks this actual socket's certificate key and TLS exporter before private data.
The verifier port is trusted code; do not substitute a remote approval boolean.
Node's native TLS APIs are intentional here; browser `fetch` cannot provide this
same-socket exporter verification. See [ADR 0019](../../docs/architecture/0019-attested-service.md).

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

After installing pinned Node 24.21.0 and building `cinder-service` with
`--all-features --locked --offline`: `npm ci --ignore-scripts`, then
`node ../../scripts/check-private-client.mjs`. Tests are offline and use synthetic
keys/public wire vectors, not configured wallets. Runtime npm dependencies: none.
