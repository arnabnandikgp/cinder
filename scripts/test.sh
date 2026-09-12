#!/usr/bin/env bash
# S0/S1 local tests. Anchor 1.0.2 defaults to Surfpool; use the legacy validator
# until S5 (Surfpool mainnet fork).
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "${root}"
./scripts/check-toolchain.sh
exec anchor test --validator legacy "$@"
