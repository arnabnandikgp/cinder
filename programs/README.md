# Cinder custody program

This isolated Anchor **1.2.0** workspace contains `cinder-vault`. The
[custody decision](../docs/architecture/0015-solana-vault.md) specifies its
accounts, instructions, authority assumptions and financial/privacy boundaries.
It does not deploy or control a native venue account.

From the repository root, with Node 24.21.0, Rust 1.97.1/rustfmt/Clippy,
Anchor CLI 1.2.0, Agave/SBF 3.1.10/platform-tools 1.52 and Surfpool 1.5.0:

```sh
cargo fetch --manifest-path programs/Cargo.toml --locked
npm ci --prefix clients/vault --ignore-scripts --no-audit --no-fund
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
deployment requires G03/G05 approval; recovery claim activation is P17, not here.
