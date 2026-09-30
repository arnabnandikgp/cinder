# Developing the TEE workspace

Start with [TRACKER](implementation/TRACKER.md), then the owning
[phase](implementation/PLAN.md). [ADR 0001](architecture/0001-workspace.md) explains
package boundaries and the deliberately small P01 scope.

## Toolchains

- Rust **1.97.1**, rustfmt and Clippy: `rust-toolchain.toml` controls this workspace.
- Node **24.21.0**: `.node-version` controls scripts/CI. Use an existing version
  manager or the official release binaries; do not change global tools implicitly.
- A C compiler builds the pinned bundled SQLite dependency; no database server
  is needed. No Node package manager, API keys, RPC endpoint, wallet, Solana CLI,
  Anchor, Docker or AWS account is needed for current checks.

One-time installation of toolchains/CI actions and `cargo fetch --locked` needs
network access. After hydration, source build/lint/tests are offline; Cargo runs
with `--locked --offline`. P05 adds pinned SQLite/hash dependencies outside the
kernel. The dependency policy checks exact versions/features/build exceptions and
the full lockfile digest; see [ADR 0005](architecture/0005-durable-journal.md).

```sh
rustup toolchain install 1.97.1 --profile minimal --component rustfmt --component clippy
node --version
cargo fetch --locked
node scripts/check.mjs
```

The runner fails on a wrong Node/Rust version. It validates docs and package edges,
runs script regressions, formatting, Clippy, build, and debug/release Rust tests.
The same command runs in CI. For focused property-harness work:

```sh
node scripts/check.mjs --properties
cargo fmt --all
```

No check sends venue requests. Rust tests cover exact financial primitives,
joined ledger/evidence, canonical identities and local durable crash/replay paths,
as well as the original toy harness. P05 includes ten actual killed child processes
and SQLite disk-full tests; it does not qualify power loss or encrypted Nitro
persistence. Do not count debug/release executions as independent mathematical proofs.
The original 233 research groups remain separate evidence, not production coverage.

## Files and packages

Kernel changes belong in `crates/kernel`; keep network, time, signing, persistence
and venue SDKs outside it. Port contracts live in `crates/ports`; offline doubles
in `crates/test-support`. Tests may inspect fake accepted/committed records, but
controllers must reconcile through actual evidence rather than access that oracle.
The local journal is `crates/journal`; its tests create and remove only uniquely
named disposable temporary directories. `test-hooks` is a test-only feature used
by the all-features runner, not a deployment feature. Its fixture protection is
deliberately insecure; there is no production cipher or plaintext fallback.
Add future packages only with documented ownership and an updated dependency guard.

Ordinary tests must use tracked sanitized fixtures. Never mount ignored `work/`,
wallet directories, cloud credentials or unrelated `stays/` into test containers.
The exact types and codecs are described in [ADR 0002](architecture/0002-financial-primitives.md).
Encrypted durable deployment, the application API and native clients remain
future phases. `--properties` selects workspace tests prefixed `property_`.

## Routine PR verification

Run the pinned checks on macOS before opening a routine PR; hosted Linux CI
provides the normal Linux verification. Do not duplicate every PR run in a local
container. After a CI failure, inspect the failed job and logs, make a justified
fix, verify it locally, and push. Allow two such fix-and-push attempts before
falling back to local Linux reproduction if the failure persists. This is room
for evidence-based iteration, not a requirement to guess twice or weaken tests.

This policy does not waive explicitly planned Linux, SBF or actual Nitro
qualification. Those tests run in their owning phases. P01's completed local
Linux run established the harness baseline; it is not a recurring pre-PR gate.

## Local Linux reproduction on Apple silicon

Use the repository's Apple `container` skill, not Docker Desktop. Native Linux
checks are relevant to the eventual enclave runtime; they do not qualify Nitro.
Create an export containing only the intended staged/tracked files. Mount that
export read-only, never the entire research worktree or host home directory.
Use the official `rust:1.97.1-slim-bookworm` image and a checksum-verified Node
24.21.0 Linux ARM64 binary. Record the pulled image digest and kernel/platform in
the tracker. Pin that digest for reproducing a recorded run; tags may move.

After installing necessary compiler components, run the same check command with
network disabled, the source mounted read-only and `CARGO_TARGET_DIR` pointed at
temporary container storage. Do not confuse ARM64 parity with x86-64 CI execution;
the hosted Linux x86-64 job remains separate evidence. If container tooling is
unavailable, record it explicitly and never claim local Linux verification.

The historical P01 run below predates registry dependencies. For P05 or later,
also hydrate the pinned Cargo cache before disabling networking; the old recipe
alone no longer bootstraps a fresh offline environment.
The P01 run used the official base directly rather than a production container
image. In a disposable **tool-only** directory, the verified Node distribution
was `node-v24.21.0-linux-arm64/`; compiler archives were
`rust-components/rustfmt.tar.xz` and `rust-components/clippy.tar.xz`. Both came from
`https://static.rust-lang.org/dist/2026-07-16/` with names
`rustfmt-1.97.1-aarch64-unknown-linux-gnu.tar.xz` and
`clippy-1.97.1-aarch64-unknown-linux-gnu.tar.xz`. Verify their adjacent `.sha256`
files; expected hashes are also the rustup cache filenames below. After exporting
only reviewed staged files, set `p01_source` and `p01_tools` to those disposable
directories and reproduce the offline check:

```sh
container run --rm --network none --cpus 2 --memory 2G \
  --mount "type=bind,source=$p01_source,target=/workspace,readonly" \
  --mount "type=bind,source=$p01_tools,target=/tools,readonly" \
  -w /workspace -e CARGO_TARGET_DIR=/tmp/cinder-target \
  docker.io/library/rust@sha256:2775a09d208ff0d7c1f50490c45b62db929e87ba1dcbc3f2132ac71a704bcdd3 \
  sh -c 'set -eu
    cp /tools/rust-components/rustfmt.tar.xz /usr/local/rustup/downloads/3dbde15d30794924195ae446f3d2ceb542a131306d22ae7912c7634d414622a8
    cp /tools/rust-components/clippy.tar.xz /usr/local/rustup/downloads/d8bac7b0ba5ca9bb868ccb9e367a1d52f4837f3ebf4892eaf64cda37ce362bb5
    rustup component add --toolchain 1.97.1 rustfmt clippy
    /tools/node-v24.21.0-linux-arm64/bin/node scripts/check.mjs'
```

Do not substitute a home/repository root for either mount. The test has no need
for other host files; compiler installation and build outputs are discarded with
the container. This is native ARM64 harness evidence, not an enclave image recipe.

## Checks and review

`Plan and handoff consistency` and `Offline Rust workspace` are the intended TEE
checks. Hosting a workflow does not enforce branch protection; inspect repository
rules before asserting a merge gate is configured. Vercel's legacy website preview
is not a financial-runtime check. Keep its result visible and request scoped
integration settings instead of fabricating a website on this branch.

Before push: run the pinned checks, inspect the explicit staged diff for private
material, update the phase handoff and verify a clean export without `work/`.
Do not deploy or run live tests based on P01 test results.
