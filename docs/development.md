# Developing the TEE workspace

Start with [TRACKER](implementation/TRACKER.md), then the owning
[phase](implementation/PLAN.md). [ADR 0001](architecture/0001-workspace.md) explains
package boundaries and the deliberately small P01 scope.

## Toolchains

- Rust **1.97.1**, rustfmt and Clippy: `rust-toolchain.toml` controls this workspace.
- Node **24.21.0**: `.node-version` controls scripts/CI. Use an existing version
  manager or the official release binaries; do not change global tools implicitly.
- No Node package manager, registry crates, API keys, RPC endpoint, wallet,
  Solana CLI, Anchor, database, Docker or AWS account is needed for current checks.

One-time installation of toolchains/CI actions/images needs network access. Once
installed, source build/lint/tests are offline; Cargo runs with `--locked --offline`.
The lockfile contains only the three local packages. This is not a promise that
future dependencies arrive without a separately controlled hydration step.

```sh
rustup toolchain install 1.97.1 --profile minimal --component rustfmt --component clippy
node --version
node scripts/check.mjs
```

The runner fails on a wrong Node/Rust version. It validates docs and package edges,
runs script regressions, formatting, Clippy, build, and debug/release Rust tests.
The same command runs in CI. For focused property-harness work:

```sh
node scripts/check.mjs --properties
cargo fmt --all
```

No check sends venue requests. Current Rust tests use a toy counter and explicit
fault doubles, not a funded account, real process crash or implemented financial
engine. Do not count debug/release executions as independent mathematical proofs.
The original 233 research groups remain separate evidence, not production coverage.

## Files and packages

Kernel changes belong in `crates/kernel`; keep network, time, signing, persistence
and venue SDKs outside it. Port contracts live in `crates/ports`; offline doubles
in `crates/test-support`. Tests may inspect fake accepted/committed records, but
controllers must reconcile through actual evidence rather than access that oracle.
Add future packages only with documented ownership and an updated dependency guard.

Ordinary tests must use tracked sanitized fixtures. Never mount ignored `work/`,
wallet directories, cloud credentials or unrelated `stays/` into test containers.
Production durable journal, financial IDs/math, application API and native clients
are intentionally still future phases.

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
