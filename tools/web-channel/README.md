# Shared web channel and offline qualification

Cinder's bounded HTTP transport uses one Rust Noise core in
`crates/web-channel`. This isolated tool workspace compiles those SAME source
files for native qualification and browser/WASM; it does not maintain a second
handshake. See [ADR 0021](../../docs/architecture/0021-confidential-web-api.md).
The HTTP service/SDK is implemented offline. WebSocket/reads, fresh changed-image
Nitro qualification and customer release approval remain open.

The suite is `Noise_NK_25519_ChaChaPoly_SHA256` through pinned upstream Snow 0.10.0.
Both handshake payloads are empty. Independent AWS-root/X.509/COSE verification
binds a caller-owned nonce, exact release policy, boot, handle, responder key and
expiry before the SDK sends private authentication. Bidirectional confirmation
precedes the finalized P18 binding and private records. No custom JavaScript
cipher, fork, plaintext fallback, resumed counters or ciphertext retry exists.
The public SDK entry fixes AWS trust; synthetic roots are separate test code.

## Reproduce

Use Rust 1.97.1, Node 24.21.0, actual Chrome (`CINDER_TEST_CHROME` if needed), and
exact wasm-bindgen CLI 0.2.129. Prepare tools/dependencies explicitly:

```sh
rustup target add wasm32-unknown-unknown --toolchain 1.97.1
cargo fetch --manifest-path tools/web-channel/Cargo.toml --locked
cargo fetch --locked
npm ci --ignore-scripts --prefix tools/web-channel
export CINDER_WASM_BINDGEN=/absolute/path/to/wasm-bindgen
node tools/web-channel/check.mjs
```

The [hosted workflow](../../.github/workflows/workspace.yml) pins the wasm-bindgen
download/checksum. Cargo runs locked/offline after hydration. The suite runs
native debug/release vectors, dependency/process regressions, native/WASM strict
Clippy, formatting, independent quote verification, and actual Node/Chrome tests.
All carrier traffic is loopback. Owned children/browser profiles and uniquely
named temporary encrypted journals are cleaned up; no existing browser profile,
configured wallet, AWS account, RPC or venue endpoint is used.

Generated ignored artifacts: `pkg/channel.js`, `pkg/channel_bg.wasm` and AWS-only
`pkg/sdk.js`. `pkg/http-fixture.js` and `pkg/attestation.js` are TEST bundles, not
client release artifacts. Distribute reviewed application artifacts/policy; never
load cryptographic code or release measurements from an attestation response.

## Evidence and boundaries

The original seven Node/Chrome groups cover the published Cacophony vector,
encrypted native round trip, replay/sticky failure, early record refusal,
tamper/reflection, size bounds and platform RNG failure. Seven further groups
exercise 40 native-checked signed synthetic documents, historical AWS certificate
path validation, every-prefix/closed-CBOR refusal, strict context/key substitution,
input snapshots and native/WASM/WebCrypto binding equality. Historical AWS evidence
is NOT a fresh browser NSM web quote; see [attestation evidence](ATTESTATION.md).

Eight shared HTTP/SDK groups run in Node AND actual Chrome against a separate
Rust service process and the existing protected journal: AWS-only/wrong-release
refusal; all eight signed commands and ownership; scoped agents/revocation;
fresh-boot reconnect; corrupt request/lost committed reply/restart reconciliation;
request/backpressure bounds; finite directional records; witness-loss containment.
The fixture seeds disposable balance/dispatches, not actual fills or payouts.
The test observer checks opaque carrier bytes, parent logs and encrypted replicas
for private markers, fixture signer and storage key. This is targeted evidence,
not a mathematical proof of privacy or live financial behavior.

Root-workspace tests add maximum 1 MiB reply assembly, truncation/trailing/offset/
correlation/tamper rejection, whole-response record budgets and explicit entropy
failure with no platform fallback. Service tests exercise final binding, expiry,
clock failure and stop/fencing. Parent tests cover fixed-target routing, private
header/path refusal, malformed frames, flush/disposal and absolute delivery timeout.
P19 Node TLS and its existing process regressions are unchanged.

## Dependencies and release limits

The isolated Rust policy freezes direct pins, lock checksums, 43 registry packages,
resolved features and build scripts. Its `lib.path` binds the shared core source.
The root shipping guard separately approves `cinder-web-channel` and its pinned
graph; the pure financial kernel remains dependency-free. Only the selected Snow
suite is enabled, not std's extra ring/Blake2 providers. Browser entropy uses pinned
getrandom's `wasm_js`; actual responder entropy uses the explicit NSM resolver.
The NPM policy freezes 34 registry entries/integrities; esbuild is build-only and
install scripts are disabled. These guards are NOT dependency audits.

Snow has no formal audit and its opaque DH/cipher/chaining arrays do not have
qualified zeroizing drops. Rust plaintext scratch uses Zeroizing; Snow internal
keys, JavaScript copies and browser memory are NOT claimed wiped. The user approved
upstream Snow for synthetic offline V1 integration, not a local crypto fork.
Before customer use, resolve/review this security limitation and applicable release
gates; changed-image/hardware/native workflows remain P23. Wrapper `free()` or
browser teardown is not an erasure proof. Public known-answer keys are test/WASM
fixtures, never production session keys. See [licenses](LICENSES.md) and
[vector provenance](THIRD_PARTY.md).
