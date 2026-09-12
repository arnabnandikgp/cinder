#!/usr/bin/env bash
# Stage C local stack: mb-stack (base :8899, ER :7799, QFS :6699).
# Privacy tests must hit QFS :6699, never raw ER :7799.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "${root}"

if ! command -v mb-stack >/dev/null 2>&1; then
  echo "stack-c: installing @magicblock-labs/ephemeral-validator (provides mb-stack)..."
  npm i -g @magicblock-labs/ephemeral-validator@latest
fi

if ! command -v mb-stack >/dev/null 2>&1; then
  echo "error: mb-stack still not on PATH after install" >&2
  exit 1
fi

echo "stack-c: mb-stack --reset"
echo "  base :8899  ER :7799  QFS :6699"
echo "  local ER identity: mAGicPQYBMvcYveUZA5F5UNNwyHvfYh5xkLS2Fr1mev"
exec mb-stack --reset
