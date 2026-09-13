#!/usr/bin/env bash
# Ledger tests use the legacy validator because Anchor 1.0.2 defaults to
# Surfpool.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "${root}"
./scripts/check-toolchain.sh
exec anchor test --validator legacy "$@"
