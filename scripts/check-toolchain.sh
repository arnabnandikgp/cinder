#!/usr/bin/env bash
# Assert the MagicBlock private-counter / PER-quickstart pin.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
mkdir -p "${root}/target/deploy"
cp "${root}/keys/cinder_vault-keypair.json" "${root}/target/deploy/"
cp "${root}/keys/cinder_ledger-keypair.json" "${root}/target/deploy/"

expected_anchor="1.0.2"
actual_anchor="$(anchor --version | awk '{print $NF}')"
if [[ "${actual_anchor}" != "${expected_anchor}" ]]; then
  echo "error: anchor --version is ${actual_anchor}, expected ${expected_anchor}" >&2
  echo "hint: avm install 1.0.2 && avm use 1.0.2" >&2
  exit 1
fi

rustc_ver="$(rustc --version | awk '{print $2}')"
if [[ "${rustc_ver}" != 1.89.* ]]; then
  echo "warn: rustc is ${rustc_ver}, expected 1.89.x (see rust-toolchain.toml)" >&2
fi

solana_ver="$(solana --version 2>/dev/null | awk '{print $2}' || true)"
if [[ -n "${solana_ver}" && "${solana_ver}" != 3.1.* ]]; then
  echo "warn: solana-cli is ${solana_ver}, spec pin is 3.1.9" >&2
fi

echo "anchor ${actual_anchor} ok"
echo "rustc  ${rustc_ver}"
echo "solana ${solana_ver:-unknown}"
