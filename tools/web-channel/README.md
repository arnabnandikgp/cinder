# Isolated web-channel qualification

This is an experiment with tracked, repeatable tests—not Cinder's shipping API,
enclave service or released client verifier. See [ADR 0021](../../docs/architecture/0021-confidential-web-api.md).
No wallet, AWS, venue, journal or private account is loaded. The native responder
trusts a public synthetic context; the test client deliberately trusts its public
fixture key. The follow-up adds [independent browser attestation qualification](ATTESTATION.md),
actual signed synthetic quotes and a historical public AWS certificate-path fixture.
**This is not fresh browser Nitro attestation.** The AWS-only verifier entry point
never accepts a fixture root; the test-only path is explicitly separate.

One Rust core implements `Noise_NK_25519_ChaChaPoly_SHA256` through exact Snow
0.10.0, compiled natively and to browser/WASM. There is no JavaScript cipher or
alternate handshake implementation. Both handshake payloads are empty; fixed
bidirectional confirmation records precede private data. Errors close the state;
there is no counter reset, resume or ciphertext retry. Bounds: 16 KiB payload,
128 records in each direction including confirmation. These are qualification
limits, not a complete financial/session API protocol.

## Reproduce

Use repository Rust 1.97.1 and Node 24.21.0, Chrome (set `CINDER_TEST_CHROME` if
needed), and exact wasm-bindgen CLI 0.2.129. Prepare tools/dependencies explicitly:

```sh
rustup target add wasm32-unknown-unknown --toolchain 1.97.1
cargo fetch --manifest-path tools/web-channel/Cargo.toml --locked
cargo fetch --locked # Existing native verifier, used only as a test oracle.
npm ci --ignore-scripts --prefix tools/web-channel
# Install checksum-verified official wasm-bindgen 0.2.129 in a task-local directory.
export CINDER_WASM_BINDGEN=/absolute/path/to/wasm-bindgen
node tools/web-channel/check.mjs
```

The hosted [workflow](../../.github/workflows/workspace.yml) records the Linux
download/checksum. The runner itself makes only loopback fixture requests; Cargo
is locked/offline. It checks the isolated dependency policy, native debug/release
vectors and hostile-state tests, native/WASM Clippy, formatting, then shared tests
in Node and actual disposable headless Chrome against the native Rust responder.
Chrome's version is printed in the receipt. Test profiles/children are cleaned up;
no existing browser profile or window is used. Build artifacts stay ignored.

The original seven Node and browser groups cover the published known answer, encrypted
round trip including exact size bound, replay/sticky failure, no early record,
tamper, direction reflection, oversize and explicit platform RNG refusal. The
carrier searches both request and response bytes for a private fixture marker.
This is a useful targeted assertion, **not a proof of metadata privacy**. Native
tests add early-handshake payload refusal, key/domain substitution, all first
handshake truncations/bit flips, transport truncations/bit flips, out-of-order and
other-session records, malformed inputs and directional record exhaustion.
Seven additional groups qualify 40 independently native-checked signed synthetic
cases, the historical AWS certificate path, COSE/root/context substitution,
closed-CBOR/every-prefix refusal, fixed native/WASM/WebCrypto encoding agreement,
input snapshotting, client challenge entropy and verified-key native Noise binding.
No private signing key is exported in the public fixture corpus. Two subprocess
tests ensure the owned Chrome group is reaped before bounded profile cleanup.

## Dependencies and limits

`dependency-policy.json` freezes the manifest, registry checksums, all 43 external
package versions, resolved features and build-script set. Changes need explicit
review; the runner does not regenerate policy. Snow's `std` feature is intentionally
off: it otherwise pulls unneeded ring/Blake2. Only the selected suite and platform
entropy resolver are enabled. WASM uses getrandom 0.3.4's `wasm_js` platform path;
failure is tested to refuse without fallback. The root shipping workspace/lockfile
and its separate dependency guard are unchanged. The policy is a supply-chain
change guard, not an audit of every transitive dependency. SHA2 is also a direct
exact edge for shared binding functions; package/features remain unchanged. The
isolated NPM graph and its 34 registry entries/integrities are separately guarded;
PKI.js/ASN1.js are candidates, esbuild is build-only, lifecycle scripts are disabled.
See [attestation policy/evidence](ATTESTATION.md) and [dependency notices](LICENSES.md).

Snow has no formal audit and its opaque DH/cipher/chaining arrays do not have
qualified zeroizing drops. Rust plaintext scratch uses Zeroizing; opaque keys,
JavaScript copies and browser memory are **not claimed to be wiped**. Before
shipping, select/review hardened internals or a narrowly reviewed dependency
change, and qualify actual enclave entropy. Never interpret dropping this fixture
as a secret-erasure or hardware result. Public known-answer keys are compiled only
for tests/WASM qualification; [THIRD_PARTY](THIRD_PARTY.md) retains their source.

Browser AWS/COSE/X.509 policy and exact prologue/binding now have bounded local
qualification, not a release or fresh NSM web quote. Still required: reviewed
key-erasure hardening, server web quotes/entropy, SDK deadlines/expiry/writer
fencing, bounded reply assembly,
HTTP service integration, richer reads, WebSocket streams and changed-image Nitro
qualification. This standalone tool cannot accept customer funds or close P21A.
