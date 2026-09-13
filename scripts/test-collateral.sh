#!/usr/bin/env bash
# Collateral tests use a fresh local validator and do not require Phoenix.
# Set CINDER_VENUE=1 to also run the Surfpool/Rise integration test.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "${root}"
export PATH="$HOME/.cargo/bin:$HOME/.local/share/solana/install/active_release/bin:$PATH"
export NVM_DIR="${NVM_DIR:-$HOME/.nvm}"
# shellcheck disable=SC1091
[ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh"

./scripts/check-toolchain.sh
export ANCHOR_WALLET="${ANCHOR_WALLET:-$HOME/.config/solana/id.json}"

# Isolated from the ledger suite because both create the same Config PDA.
NO_DNA=1 CINDER_MOCHA=collateral.test.ts anchor test --validator legacy

if [[ "${CINDER_VENUE:-}" == "1" ]]; then
  yarn ts-node scripts/venue-boot.ts
  yarn run ts-mocha -p ./tsconfig.json -t 180000 tests/venue.test.ts
fi
