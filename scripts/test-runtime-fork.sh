#!/usr/bin/env bash
# Opt-in native execution + real PER/QFS proof. No public-chain transactions.
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ "${CINDER_R5_FORK:-}" != 1 ]]; then
  echo 'Set CINDER_R5_FORK=1 to authorize disposable localhost fork tests.' >&2
  exit 1
fi
for port in 8989 8990 7799 7800 6699 6700; do
  if lsof -nP -iTCP:"$port" -sTCP:LISTEN >/dev/null 2>&1; then
    echo "Port $port is in use; stop the existing local stack first." >&2
    exit 1
  fi
done
for bin in surfpool ephemeral-validator query-filtering-service mb-test-validator; do
  command -v "$bin" >/dev/null || { echo "Missing $bin" >&2; exit 1; }
done
storage="$(mktemp -d "${TMPDIR:-/tmp}/cinder-r5-runtime.XXXXXX")"
cleanup() {
  for pid in "${qfs_pid:-}" "${er_pid:-}" "${fork_pid:-}"; do
    # The npm ER/QFS launchers forward SIGINT to their native child, but not
    # SIGTERM. Wait for that forwarded shutdown before releasing local ports.
    if [[ -n "$pid" ]]; then kill -INT "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true; fi
  done
  echo "Local test logs/storage: $storage"
}
trap cleanup EXIT INT TERM
if [[ "${CINDER_R5_SKIP_PROGRAM_BUILD:-}" != 1 ]]; then
  NO_DNA=1 anchor build --program-name cinder_vault --ignore-keys
  NO_DNA=1 anchor build --program-name cinder_ledger --ignore-keys
fi
CARGO_INCREMENTAL=0 cargo build --locked -p cinder-operator
export PROVIDER_ENDPOINT=http://127.0.0.1:8989
NO_DNA=1 surfpool start --host 127.0.0.1 --port 8989 --ws-port 8990 --rpc-url "${SURFPOOL_RPC_URL:-https://api.mainnet-beta.solana.com}" --no-deploy --no-tui --no-studio --ci --db :memory: --airdrop-amount 0 --skip-signature-verification >"$storage/fork.log" 2>&1 &
fork_pid=$!
./scripts/wait-rpc.sh "$PROVIDER_ENDPOINT"
export CINDER_MB_TEST_DUMPS="${CINDER_MB_TEST_DUMPS:-$(node -e 'const p=require("path"),f=require("fs");process.stdout.write(p.join(p.dirname(f.realpathSync(process.argv[1])),"bin/local-dumps"))' "$(command -v mb-test-validator)")}"
node -r ts-node/register/transpile-only scripts/load-magicblock-fork.ts
RUST_LOG=warn ephemeral-validator --no-tui --listen 127.0.0.1:7799 --lifecycle ephemeral --remotes "$PROVIDER_ENDPOINT" --remotes ws://127.0.0.1:8990 --storage "$storage/er" >"$storage/er.log" 2>&1 &
er_pid=$!
./scripts/wait-rpc.sh http://127.0.0.1:7799
query-filtering-service --listen-addr 127.0.0.1:6699 --listen-addr-ws 127.0.0.1:6700 --ephemeral-url http://127.0.0.1:7799 --ephemeral-url-ws ws://127.0.0.1:7800 >"$storage/qfs.log" 2>&1 &
qfs_pid=$!
./scripts/wait-rpc.sh http://127.0.0.1:6699
CINDER_R5_PRIVATE="${CINDER_R5_PRIVATE:-1}" node node_modules/mocha/bin/mocha.js -r ts-node/register/transpile-only tests/native-clock.test.ts tests/native-confirmation.test.ts tests/runtime-funding-fork.test.ts
