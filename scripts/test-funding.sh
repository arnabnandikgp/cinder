#!/usr/bin/env bash
# Fork smoke for Rise funding wiring. Requires Surfpool (`stack-fork.sh`) and
# CINDER_FUNDING=1.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "${root}"
export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$HOME/.local/share/solana/install/active_release/bin:$PATH"
export NVM_DIR="${NVM_DIR:-$HOME/.nvm}"
# shellcheck disable=SC1091
[ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh"

export CINDER_FUNDING="${CINDER_FUNDING:-1}"
export PROVIDER_ENDPOINT="${PROVIDER_ENDPOINT:-http://127.0.0.1:8899}"
export ANCHOR_WALLET="${ANCHOR_WALLET:-$HOME/.config/solana/id.json}"

yarn run ts-mocha -p ./tsconfig.json -t 180000 tests/funding-fork.test.ts
