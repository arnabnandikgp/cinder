# Opaque web relay

Node 24.21.0's built-in HTTP/net server handles public HTTP/WebSocket paths, bounded framing and
random session routing only. It has no customer signer, plaintext dispatcher,
venue credentials, private persistence or cryptographic provider.

```sh
node services/web-relay/main.mjs 127.0.0.1:8080 127.0.0.1:8081 https://client.example
```

The upstream is a fixed loopback opaque Rust relay to the enclave's AF_VSOCK port,
never a caller-controlled URL. The enclave explicitly selects `cinder-enclave
--web`; the original no-flag TLS entry remains supported. Exactly one enclave
runtime/financial journal is constructed. This slice does not deploy or qualify
that changed image on AWS; P23 owns fresh hardware/native workflow qualification.

Production ingress must supply HTTPS; loopback HTTP is only the offline fixture
profile. Optional CORS permits one configured exact origin, without cookies or
credentials. TLS alone does not substitute for the SDK's fresh Nitro verification
and Noise channel. The protocol, failure/reconciliation rules and limits are in
[ADR 0021](../../docs/architecture/0021-confidential-web-api.md) and the
[read/socket contract](../../docs/architecture/private-read-contract.md).
`/v1/ws` is a strict binary carrier attached to an already confirmed handle;
all commands/subscriptions remain encrypted and authorized inside the service.
No text/subprotocol/compression/private headers or arbitrary upstream is supported.

Offline regressions: `node --test services/web-relay/*.test.mjs`. Actual
Node/Chrome signed SDK and protected-journal process workflows run through
`node tools/web-channel/check.mjs`. No RPC, venue endpoint, configured wallet,
cloud credentials or customer funds are used.
