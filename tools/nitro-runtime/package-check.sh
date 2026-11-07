#!/bin/sh
# Run standalone in the isolated ARM64 Linux check container. No AWS.
set -eu
test "$(uname -s)" = Linux
test "$(uname -m)" = aarch64
test -z "$(find /sys/class/net -mindepth 1 -maxdepth 1 ! -name lo -print -quit)"
test ! -e /dev/nsm
test ! -e /source/work
test ! -e /source/.git
test ! -e /tmp/cinder-package
test ! -e /tmp/cinder-bundle
mkdir -p /tmp/cargo
cp tools/nitro-linux/cargo-config.toml /tmp/cargo/config.toml
export CARGO_HOME=/tmp/cargo
export CARGO_INCREMENTAL=0
# A separate deterministic artifact target: never ship an all-feature test build.
export CARGO_TARGET_DIR=/tmp/cinder-package
if test -d /build-cache; then
  # Only dependency intermediates are retained by CI. Always rebuild the
  # application, even if an old application output reaches this mount.
  export CARGO_TARGET_DIR=/build-cache
  cargo clean -p cinder-service --release
fi
export RUSTFLAGS='--remap-path-prefix=/source=/cinder --remap-path-prefix=/vendor=/vendor -C debuginfo=0'
cargo build --locked --offline --release -p cinder-service --bin cinder-enclave \
  --bin cinder-bootstrap --bin cinder-cloud-relay --bin cinder-nitro-relay \
  --bin cinder-nitro-egress --bin cinder-verify-quote --bin cinder-prepare-release
mkdir -p /tmp/cinder-bundle/bin /tmp/cinder-bundle/rootfs/usr/bin \
  /tmp/cinder-bundle/rootfs/lib/aarch64-linux-gnu /tmp/cinder-bundle/rootfs/lib
for name in cinder-enclave cinder-bootstrap cinder-cloud-relay cinder-nitro-relay cinder-nitro-egress cinder-verify-quote cinder-prepare-release; do
  cp "$CARGO_TARGET_DIR/release/$name" /tmp/cinder-bundle/bin/
done
cp "$CARGO_TARGET_DIR/release/cinder-enclave" /tmp/cinder-bundle/rootfs/usr/bin/
for name in libssl.so.3 libcrypto.so.3 libc.so.6 libm.so.6 libgcc_s.so.1; do
  cp "/lib/aarch64-linux-gnu/$name" /tmp/cinder-bundle/rootfs/lib/aarch64-linux-gnu/
done
cp /lib/ld-linux-aarch64.so.1 /tmp/cinder-bundle/rootfs/lib/
ldd "$CARGO_TARGET_DIR/release/cinder-enclave"
sha256sum /tmp/cinder-bundle/bin/* /tmp/cinder-bundle/rootfs/lib/aarch64-linux-gnu/* \
  /tmp/cinder-bundle/rootfs/lib/ld-linux-aarch64.so.1 > /tmp/cinder-bundle/SHA256SUMS
# Same public inputs, a freshly recompiled application crate, byte-identical ELF.
cp "$CARGO_TARGET_DIR/release/cinder-enclave" /tmp/first-enclave
cargo clean -p cinder-service --release
cargo build --locked --offline --release -p cinder-service --bin cinder-enclave
cmp /tmp/first-enclave "$CARGO_TARGET_DIR/release/cinder-enclave"
printf 'Default-feature enclave ELF reproducibility: identical\n'
# An ordinary process cannot silently become a local fixture or boot without NSM.
if /tmp/cinder-bundle/bin/cinder-enclave >/tmp/boot-public.out 2>/tmp/boot-public.err; then
  printf 'Unexpected non-Nitro boot\n' >&2; exit 1
fi
test ! -s /tmp/boot-public.out
test "$(cat /tmp/boot-public.err)" = 'cinder runtime fenced'
