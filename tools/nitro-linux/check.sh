#!/bin/sh
# Isolated Rust check; live NSM, vsock and SDK/Node qualification are separate.
set -eu
mkdir -p /tmp/cargo
cp /check/cargo-config.toml /tmp/cargo/config.toml
rustc --version
openssl version
pkg-config --modversion openssl
test -z "$(find /sys/class/net -mindepth 1 -maxdepth 1 ! -name lo -print -quit)"
test ! -e /dev/nsm
test ! -e /source/work
test ! -e /source/.git
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo build --workspace --all-targets --all-features --locked --offline
cargo test --workspace --all-features --locked --offline
cargo test --workspace --all-features --release --locked --offline
