#!/usr/bin/env bash
# S5 collateral path on a fresh local validator (no Phoenix programs required).
# Venue boot (Surfpool + Rise) is opt-in: CINDER_S5=1 ./scripts/test-s5.sh
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "${root}"
export PATH="$HOME/.cargo/bin:$HOME/.local/share/solana/install/active_release/bin:$PATH"
export NVM_DIR="${NVM_DIR:-$HOME/.nvm}"
# shellcheck disable=SC1091
[ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh"

./scripts/check-toolchain.sh
export ANCHOR_WALLET="${ANCHOR_WALLET:-$HOME/.config/solana/id.json}"

# Isolated from S1 (same Config PDA). Mocha glob via CINDER_MOCHA.
NO_DNA=1 CINDER_MOCHA=s5-collateral.ts anchor test --validator legacy

if [[ "${CINDER_S5:-}" == "1" ]]; then
  yarn ts-node scripts/venue-boot.ts
  yarn run ts-mocha -p ./tsconfig.json -t 180000 tests/s5-venue.ts
fi
