#!/usr/bin/env bash
# Stage A local stack: Surfpool mainnet fork + local ER + QFS (S5).
set -euo pipefail

if command -v surfpool >/dev/null 2>&1; then
  echo "stack-a: starting Surfpool fork (RPC :8899). Point ER/QFS at localhost."
  echo "  ephemeral-validator --lifecycle ephemeral --remotes http://127.0.0.1:8899 ..."
  echo "  query-filtering-service --listen-addr 127.0.0.1:6699 --ephemeral-url http://127.0.0.1:7799"
  exec surfpool start --rpc-url https://api.mainnet-beta.solana.com
fi

echo "stack-a: surfpool is not installed (S5)."
echo "  curl -sL https://run.surfpool.run/ | bash"
echo "  surfpool start --rpc-url https://api.mainnet-beta.solana.com"
exit 1
