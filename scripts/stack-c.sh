#!/usr/bin/env bash
# Stage C local stack: mb-stack (base :8899, ER :7799, QFS :6699).
# Privacy tests must hit QFS :6699, never raw ER :7799.
set -euo pipefail

if command -v mb-stack >/dev/null 2>&1; then
  exec mb-stack --reset
fi

echo "stack-c: mb-stack is not installed (S2)."
echo "  npm i -g @magicblock-labs/ephemeral-validator@latest"
echo "  mb-stack --reset"
echo "  # 8899 base, 7799 ER, 6699 QFS"
echo "  # local ER identity: mAGicPQYBMvcYveUZA5F5UNNwyHvfYh5xkLS2Fr1mev"
exit 1
