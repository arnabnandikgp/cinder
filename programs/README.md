# Cinder custody program

This isolated Anchor **1.2.0** workspace contains `cinder-vault`. The
[custody decision](../docs/architecture/0015-solana-vault.md) specifies its
accounts, instructions, authority assumptions and financial/privacy boundaries.
It does not deploy or control a native venue account.

From the repository root, with Node 24.21.0, Rust 1.97.1/rustfmt/Clippy,
Anchor CLI 1.2.0, Agave/SBF 3.1.10/platform-tools 1.52, Surfpool 1.5.0,
OpenSSL development headers/pkg-config and **wasm-bindgen CLI 0.2.129**:

The default vault suite includes the joined HTTP/WebSocket recovery tests. It
therefore also builds the private service and browser WASM core; these are
required even without `--recovery-only` (which only narrows the tests run).
Install the exact wasm-bindgen CLI separately, for example with
`cargo install wasm-bindgen-cli --version 0.2.129 --locked --root /absolute/path/to/cinder-test-tools`.
The [vault CI workflow](../.github/workflows/vault.yml) instead provisions a
checksum-verified release binary and all dependencies before running offline.

```sh
cargo fetch --locked
cargo fetch --manifest-path tools/web-channel/Cargo.toml --locked
cargo fetch --manifest-path programs/Cargo.toml --locked
npm ci --prefix tools/web-channel --ignore-scripts --no-audit --no-fund
npm ci --prefix clients/vault --ignore-scripts --no-audit --no-fund
rustup target add wasm32-unknown-unknown --toolchain 1.97.1
# Omit this override if the exact CLI is already on PATH:
export CINDER_WASM_BINDGEN=/absolute/path/to/cinder-test-tools/bin/wasm-bindgen
"${CINDER_WASM_BINDGEN:-wasm-bindgen}" --version # must print wasm-bindgen 0.2.129
# One-time platform-tools hydration if not already installed:
NO_DNA=1 cargo build-sbf --manifest-path programs/cinder-vault/Cargo.toml --tools-version v1.52 --arch v0 -- --locked
node scripts/check-vault.mjs
```

If the global Anchor differs, set `CINDER_ANCHOR_TOOL` to a separately installed
1.2.0 binary. The check will not install/replace global tools. It owns a fresh
offline Surfpool process on ports 18899/18900 and refuses occupied ports.
All identities are disposable in-memory local fixtures; no wallet file is loaded.
The normal financial workspace checks remain `node scripts/check.mjs`.

Do not deploy the fixed local identity or unused generated keypair. External
deployment requires G03/G05 approval. P17 extends the same program with
[immutable final recovery claims](../docs/architecture/0017-recovery-claims.md),
separate operator activation and the existing shared payout counters. P21's
joined offline tests exercise ledger reconciliation and private claim delivery,
and use fake evidence for native fencing. Live native-fencing and venue/hardware
qualification remain P23.
