#!/usr/bin/env bash
# Record cluster: mb-stack (base :8899, ER :7799, QFS :6699) + mocked Phoenix residual.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "${root}"

export PATH="$HOME/.cargo/bin:$HOME/.local/share/solana/install/active_release/bin:$PATH"
export NVM_DIR="${NVM_DIR:-$HOME/.nvm}"
# shellcheck disable=SC1091
[ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh"

export PROVIDER_ENDPOINT="${PROVIDER_ENDPOINT:-http://127.0.0.1:8899}"
export EPHEMERAL_PROVIDER_ENDPOINT="${EPHEMERAL_PROVIDER_ENDPOINT:-http://127.0.0.1:7799}"
export TEE_PROVIDER_ENDPOINT="${TEE_PROVIDER_ENDPOINT:-http://127.0.0.1:6699}"
export TEE_WS_ENDPOINT="${TEE_WS_ENDPOINT:-ws://127.0.0.1:6700}"
export ANCHOR_PROVIDER_URL="$PROVIDER_ENDPOINT"
export ANCHOR_WALLET="${ANCHOR_WALLET:-$HOME/.config/solana/id.json}"

if ! ./scripts/wait-rpc.sh "${TEE_PROVIDER_ENDPOINT}" 5; then
  echo "error: QFS not up. Other terminal: ./scripts/stack-c.sh" >&2
  exit 1
fi

./scripts/check-toolchain.sh
anchor build
anchor deploy --provider.cluster localnet
yarn ts-node --transpile-only scripts/cinder-demo.ts
