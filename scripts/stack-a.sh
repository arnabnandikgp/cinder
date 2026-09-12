#!/usr/bin/env bash
# Stage A: Surfpool mainnet fork on :8899. ER/QFS still pointed at localhost.
set -euo pipefail

export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"

if ! command -v surfpool >/dev/null 2>&1; then
  echo "stack-a: installing surfpool..."
  curl -sL https://run.surfpool.run/ | bash
  export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"
fi

if ! command -v surfpool >/dev/null 2>&1; then
  echo "error: surfpool not on PATH after install" >&2
  echo "  curl -sL https://run.surfpool.run/ | bash" >&2
  exit 1
fi

echo "stack-a: surfpool start --rpc-url ${SURFPOOL_RPC_URL:-https://api.mainnet-beta.solana.com}"
echo "  RPC :8899  (point local ER/QFS remotes here)"
echo "  Do not send-register-ixs to Phoenix mainnet; send built ixs to this fork."
exec surfpool start --rpc-url "${SURFPOOL_RPC_URL:-https://api.mainnet-beta.solana.com}"
