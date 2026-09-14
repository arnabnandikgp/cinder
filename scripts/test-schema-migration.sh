#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "${root}"

wallet="${ANCHOR_WALLET:-${HOME}/.config/solana/id.json}"
for bin in anchor mb-test-validator ephemeral-validator query-filtering-service; do
  if ! command -v "${bin}" >/dev/null 2>&1; then
    echo "error: ${bin} is required" >&2
    exit 1
  fi
done

for port in 8899 7799 6699 6700; do
  if lsof -nP -iTCP:"${port}" -sTCP:LISTEN >/dev/null 2>&1; then
    echo "error: port ${port} is already in use" >&2
    exit 1
  fi
done

anchor build

storage="$(mktemp -d "${TMPDIR:-/tmp}/cinder-schema-migration.XXXXXX")"
base_ledger="${storage}/base-ledger"
er_ledger="${storage}/er-ledger"
fixture="${storage}/fixture"

cleanup() {
  for pid in "${qfs_pid:-}" "${er_pid:-}" "${base_pid:-}"; do
    [ -n "${pid}" ] && pkill -TERM -P "${pid}" 2>/dev/null || true
  done
  sleep 0.2
  for pid in "${qfs_pid:-}" "${er_pid:-}" "${base_pid:-}"; do
    [ -n "${pid}" ] && pkill -KILL -P "${pid}" 2>/dev/null || true
    [ -n "${pid}" ] && kill -KILL "${pid}" 2>/dev/null || true
  done
}
trap cleanup EXIT INT TERM

ANCHOR_WALLET="${wallet}" ./node_modules/.bin/ts-node --transpile-only \
  scripts/create-schema-fixture.ts "${fixture}"

mb-test-validator --quiet \
  --ledger "${base_ledger}" \
  --rpc-port 8899 \
  --reset \
  --account-dir "${fixture}/accounts" \
  --bpf-program h3Bw2xjj69JssRkaxr8Jxh6TtamvrjSxASXbfLeWyPg target/deploy/cinder_ledger.so \
  --bpf-program 9zhBFVgk13gnYT6iVuKPGfQiAvVfr6cYQq2bY2QUzXmg target/deploy/cinder_vault.so \
  >"${storage}/base.log" 2>&1 &
base_pid=$!
./scripts/wait-rpc.sh http://127.0.0.1:8899
# The RPC socket opens before the validator finishes stabilizing fees. Starting
# ER during that short window is flaky because its remote handshake can fail.
sleep 3

ephemeral-validator --no-tui --listen 127.0.0.1:7799 \
  --lifecycle ephemeral \
  --remotes http://127.0.0.1:8899 \
  --remotes ws://127.0.0.1:8900 \
  --storage "${er_ledger}" \
  --reset >"${storage}/er.log" 2>&1 &
er_pid=$!
./scripts/wait-rpc.sh http://127.0.0.1:7799 30

query-filtering-service \
  --listen-addr 127.0.0.1:6699 \
  --listen-addr-ws 127.0.0.1:6700 \
  --ephemeral-url http://127.0.0.1:7799 \
  --ephemeral-url-ws ws://127.0.0.1:7800 \
  >"${storage}/qfs.log" 2>&1 &
qfs_pid=$!
./scripts/wait-rpc.sh http://127.0.0.1:6699

ANCHOR_PROVIDER_URL=http://127.0.0.1:8899 \
ANCHOR_WALLET="${wallet}" \
EPHEMERAL_PROVIDER_ENDPOINT=http://127.0.0.1:7799 \
TEE_PROVIDER_ENDPOINT=http://127.0.0.1:6699 \
TEE_WS_ENDPOINT=ws://127.0.0.1:6700 \
CINDER_MIGRATION_MANIFEST="${fixture}/manifest.json" \
  ./node_modules/.bin/ts-mocha -p ./tsconfig.json -t 180000 tests/schema-migration.test.ts
