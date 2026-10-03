# Private Cinder client

Thin semantic SDK for P18: private snapshots/operations, GTC/ALO/bounded IOC,
attributed cancellation, owner payouts, scoped grants and private leverage.
Amounts are bigint lots/ticks/quote atoms; account/market grids come from the
approved deployment policy. It is not a Pacifica URL-only proxy or native signer.

Inject wallet/agent `signMessage` and a **trusted confidential channel**. P19 adds
the Node automation profile below; the semantic client still supports a separately
qualified channel port. A caller-created object with matching fields is not proof
of confidentiality. There is no plaintext fallback. P21A adds the bounded web
profile below; WebSocket remains the next slice. See [ADR 0018](../../docs/architecture/0018-private-api.md).

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
keys/public wire vectors, not configured wallets. The Node TLS path has no runtime
npm dependencies.

## Confidential browser/Node HTTP

`src/browser.ts` is the public web bundle entry. `connectWebChannel` fixes AWS-only
trust and implements the same `ConfidentialChannel` consumed by `PrivateClient`.
Load the generated `channel.js`/`channel_bg.wasm` from your trusted client
distribution and pass its initialized module as `core`, plus an independently
selected release policy and `baseUrl`. There is no public quote-verifier/root/
approval option. Do not load the core or policy from the attestation response.

```ts
import init, * as core from './channel.js';
import { connectWebChannel, PrivateClient } from './sdk.js';
await init();
const channel = await connectWebChannel({
  baseUrl: configuredHttpsOrigin, policy: independentlyApprovedRelease, core,
});
// Inject the same walletSigner/account/domain configuration shown above.
```

The web bundle includes pinned PKI.js/ASN1.js verification and the WASM core uses
pinned upstream Snow. No remote verification service decides trust. Full offline
build and actual Chrome/Node process checks: `node tools/web-channel/check.mjs`
after [tool preparation](../../tools/web-channel/README.md). It generates ignored
`pkg/sdk.js`, `pkg/channel.js` and `pkg/channel_bg.wasm`. Fixture trust is separate
test code, not a public SDK option.

No package release, terminal/onboarding, live endpoint or fresh Nitro qualification
is claimed. See [ADR 0021](../../docs/architecture/0021-confidential-web-api.md).
