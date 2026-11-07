# P20 local Linux checks

Explicit Linux qualification, not another default pre-PR check. Use the Apple
container skill and current CLI help before running this recipe. This toolchain
has no NSM device and is not an EIF. Hardware/Nitro/KMS/witness and native venue
qualification are separate [P20 gates](../../docs/architecture/0020-nitro-runtime.md).
The Node/SDK runner is still tested separately on the pinned host toolchain.

Export an exact committed product revision, never mount the live worktree, home,
wallets or cloud directory. Install the Rust dependencies beforehand through
the approved locked public fetch; vendoring below then works offline. Check disk
space first: builder, image export and Rust outputs can temporarily consume
several GiB. Use a source-free image build context; only setup downloads public
toolchain/Debian packages. The base digest and observed package versions are
pinned, but an immutable Debian repository snapshot is not selected here.

From the product worktree, with an approved local container session:

```sh
p20_linux_dir=$(mktemp -d /private/tmp/cinder-nitro-linux.XXXXXX)
p20_source_commit=$(git rev-parse HEAD)
mkdir "$p20_linux_dir/source" "$p20_linux_dir/image"
git archive --format=tar "$p20_source_commit" | tar -xf - -C "$p20_linux_dir/source"
cp tools/nitro-linux/Containerfile "$p20_linux_dir/image/Containerfile"
cargo vendor --locked --offline --versioned-dirs "$p20_linux_dir/vendor"
container system start
# If the default kernel is absent, use the CLI's recommended kernel explicitly.
container build --platform linux/arm64 --cpus 4 --memory 4g --dns 1.1.1.1 \
  --progress plain -t cinder-nitro-linux:check "$p20_linux_dir/image"
container run --rm --network none --cpus 4 --memory 4g \
  --mount "type=bind,source=$p20_linux_dir/source,target=/source,readonly" \
  --mount "type=bind,source=$p20_linux_dir/vendor,target=/vendor,readonly" \
  --mount "type=bind,source=$p20_linux_dir/source/tools/nitro-linux,target=/check,readonly" \
  --workdir /source \
  -e CARGO_TARGET_DIR=/tmp/cinder-target -e CARGO_HOME=/tmp/cargo \
  -e CARGO_INCREMENTAL=0 -e CARGO_NET_OFFLINE=true -e CARGO_BUILD_JOBS=4 \
  -e CARGO_PROFILE_DEV_DEBUG=0 -e CARGO_PROFILE_TEST_DEBUG=0 \
  cinder-nitro-linux:check sh /check/check.sh
```

Record exact source revision/tree, image/ARM64 digests, kernel, library/toolchain
versions, flags and results. Debug symbols/incremental compilation are omitted
for space; debug overflow/assertion semantics remain enabled. No environment
inheritance, interactive cloud session, port publication, network interface or
credentials are necessary. Tests verify the ordinary Linux NSM refusal and safe
owned-socket operations; a Unix pair is not a Nitro vsock device. Do not re-label
these results as hardware evidence.

The check container removes itself and its temporary build outputs on exit.
Remove only the task's named image/builder/temp export if cleanup is needed;
never prune shared images/containers or delete source/research. Do not stop/delete
a builder another task owns. Downloaded public dependencies and generated build
cache can be recreated; they are not project records.
