#!/usr/bin/env bash
# S2 PER/QFS tests. Requires mb-stack (base :8899, ER :7799, QFS :6699).
# Does not start a second validator — deploy onto the stack, then mocha s2.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "${root}"

export PATH="$HOME/.cargo/bin:$HOME/.local/share/solana/install/active_release/bin:$PATH"
export NVM_DIR="${NVM_DIR:-$HOME/.nvm}"
# shellcheck disable=SC1091
[ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh"

./scripts/check-toolchain.sh

qfs_up() {
  curl -sf --max-time 2 -X POST http://127.0.0.1:6699 \
    -H 'content-type: application/json' \
    -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' >/dev/null 2>&1
}

if ! qfs_up; then
  echo "error: QFS :6699 is not up. Start the stack first:" >&2
  echo "  ./scripts/stack-c.sh" >&2
  exit 1
fi

export PROVIDER_ENDPOINT="${PROVIDER_ENDPOINT:-http://127.0.0.1:8899}"
export WS_ENDPOINT="${WS_ENDPOINT:-ws://127.0.0.1:8900}"
export EPHEMERAL_PROVIDER_ENDPOINT="${EPHEMERAL_PROVIDER_ENDPOINT:-http://127.0.0.1:7799}"
export EPHEMERAL_WS_ENDPOINT="${EPHEMERAL_WS_ENDPOINT:-ws://127.0.0.1:7800}"
export TEE_PROVIDER_ENDPOINT="${TEE_PROVIDER_ENDPOINT:-http://127.0.0.1:6699}"
export TEE_WS_ENDPOINT="${TEE_WS_ENDPOINT:-ws://127.0.0.1:6700}"
export ANCHOR_PROVIDER_URL="$PROVIDER_ENDPOINT"
export ANCHOR_WALLET="${ANCHOR_WALLET:-$HOME/.config/solana/id.json}"
export VALIDATOR="${VALIDATOR:-mAGicPQYBMvcYveUZA5F5UNNwyHvfYh5xkLS2Fr1mev}"

anchor build
anchor deploy --provider.cluster localnet
yarn run ts-mocha -p ./tsconfig.json -t 120000 tests/s2-privacy.ts
