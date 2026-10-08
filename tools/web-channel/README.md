# Shared web channel and offline qualification

Cinder's bounded HTTP/WebSocket transport uses one Rust Noise core in
`crates/web-channel`. This isolated tool workspace compiles those SAME source
files for native qualification and browser/WASM; it does not maintain a second
handshake. See [ADR 0021](../../docs/architecture/0021-confidential-web-api.md).
The service/SDK and bounded private reads/updates are implemented offline.
Fresh changed-image Nitro qualification and customer release approval remain open.

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

The default runner includes the [retained-history pack candidate](../../docs/architecture/0026-retained-history-packs.md).
CI selects `--standard-only` for existing channel/C4 checks and `--packed-only`
for a separate cached growth job, each retaining the fifteen-minute job limit.
The pack job builds the same pinned native/WASM/verifier prerequisites, then
runs actual Node/Chrome HTTP/WebSocket growth and original-ID replay. Candidate
storage is selected only in local-fixture mode, never in shipping Nitro.

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

Nine shared command/read groups per transport run in Node AND actual Chrome against a separate
Rust service process and the existing protected journal: AWS-only/wrong-release
refusal; all eight signed commands and ownership; scoped agents/revocation;
fresh-boot reconnect; corrupt request/lost committed reply/restart reconciliation;
request/backpressure bounds; finite directional records; witness-loss containment.
Six additional socket groups cover initial/correlated snapshots, other-ingress
commits/pagination, agent privacy/revocation, idle expiry, dropped delivery/fresh
reconciliation, real slow-consumer overflow and witness loss. Both transports
run all eight original commands and ten reads.
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

### C4 joined Runtime matrix

`node tools/web-channel/check-runtime.mjs` (also invoked by `check.mjs`) runs the
SAME actual Node/Chrome suite through the opaque relay into `Runtime`, the API and
AEAD/replicated file journal—not `FixtureHandler`. After normal WASM/tool hydration,
it needs the feature-gated `cinder-service-fixture` binary; `--node-only` is a
focused iteration mode, never full browser qualification. `/usr/bin/time` supplies
whole-process peak RSS (macOS `-l`, Linux `-v`); missing memory evidence fails.
The runner copies and hashes one disposable executable for every cell; an
independent build cannot replace the qualified artifact midway through a run.

The fixed grid is initial accepted history 1/8/32/64, writer AND independent-read
witness delays 0/50/400 ms, and one/two private watches: 24 cells per client.
Each cell joins two simultaneous reads, one same-socket owner grant and inactive
controller tick. History pads are 16 KiB encrypted non-economic evidence; no
customer credit or economic risk is seeded. Counters report whole-run replica
GET/PUT bytes, writer/fresh read calls, peak independent read I/O, clock calls and
acceptance. Latency and peak RSS include the real Runtime, writer candidate,
retained accepted history/index/view generations and carrier buffers for THIS
declared workload. The 128-MiB local fixture ceiling is not production capacity,
maximum-sized records/pages, NSM cost, independent AWS-witness qualification or an
enclave SLO. Dependencies and root/time authority remain explicitly synthetic.

Fixed fault groups cover agent revoke during I/O, queued expiry, known
post-CAS/pre-publication races, sustained unrelated commits, exact encrypted reopen
without mutation resend, continuous ingress/expiry, slow-consumer queue overflow,
and witness/head/epoch/credential-port/lease/boot refusal. Credential failure here
is a synthetic port error; exact STS margin/expiry checks remain cloud unit tests
and the hardware gate. Journal tests separately retain four distinct generations,
share the original records and test poison/panic/uncertain acceptance. Together
these are bounded implementation tests, not a formal proof or audit.

Controls are bounded private stdin only. The public test harness observes framing
for notification timing; it cannot decrypt replies. Every run owns disposable
keys/files/children and checks captured carrier, logs and ciphertext for private
markers. Quiet-window recovery after unrelated writes is expected; a continuously
changing global generation does not guarantee read progress. No timeout, cache,
account-only freshness, financial gate or storage-repair contract is relaxed.

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
