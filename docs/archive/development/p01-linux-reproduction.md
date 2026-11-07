# Historical P01 Linux reproduction

Preserved from `docs/development.md` at `58c9da1` during the 2026-11-07
bookkeeping pass. This records the original harness recipe, not a new run,
current tool installation instructions, or permission to launch a container.
See the [current development guide](../../development.md) for maintained setup
and verification requirements. No container was run during this archival move.

The historical P01 run predates registry dependencies. For P05 or later,
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
