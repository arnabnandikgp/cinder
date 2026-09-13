#!/usr/bin/env bash
# Local privacy stack: base :8899, ER :7799, QFS :6699.
# Privacy tests must hit QFS :6699, never raw ER :7799.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "${root}"

for port in 8899 7799 6699; do
  if lsof -nP -iTCP:"${port}" -sTCP:LISTEN >/dev/null 2>&1; then
    echo "error: port ${port} is already in use; stop the existing local stack first" >&2
    exit 1
  fi
done

# Isolate both validators from their default ledgers so each demo run is fresh
# without deleting another local project's state.
stack_storage="$(mktemp -d "${TMPDIR:-/tmp}/cinder-stack.XXXXXX")"
base_ledger="${stack_storage}/base-ledger"
er_storage="${stack_storage}/er-ledger"

for bin in mb-test-validator ephemeral-validator query-filtering-service; do
  if command -v "$bin" >/dev/null 2>&1; then
    continue
  fi
  echo "stack-local: installing @magicblock-labs/ephemeral-validator..."
  npm i -g @magicblock-labs/ephemeral-validator@0.14.10
  break
done

for bin in mb-test-validator ephemeral-validator query-filtering-service; do
  if ! command -v "$bin" >/dev/null 2>&1; then
    echo "error: $bin is not on PATH after install" >&2
    exit 1
  fi
done

cleanup() {
  for pid in "${qfs_pid:-}" "${er_pid:-}" "${base_pid:-}"; do
    [ -n "$pid" ] && kill "$pid" 2>/dev/null || true
  done
  wait "${qfs_pid:-}" "${er_pid:-}" "${base_pid:-}" 2>/dev/null || true
}
trap cleanup EXIT INT TERM

echo "stack-local: resetting base and ER ledgers"
echo "  base :8899  ER :7799  QFS :6699"
echo "  local ER identity: mAGicPQYBMvcYveUZA5F5UNNwyHvfYh5xkLS2Fr1mev"
echo "  local storage: ${stack_storage}"

mb-test-validator --ledger "${base_ledger}" --rpc-port 8899 --reset &
base_pid=$!
./scripts/wait-rpc.sh http://127.0.0.1:8899

ephemeral-validator --no-tui --listen 127.0.0.1:7799 \
  --lifecycle ephemeral \
  --remotes http://127.0.0.1:8899 \
  --remotes ws://127.0.0.1:8900 \
  --storage "${er_storage}" \
  --reset &
er_pid=$!
./scripts/wait-rpc.sh http://127.0.0.1:7799

query-filtering-service \
  --listen-addr 127.0.0.1:6699 \
  --listen-addr-ws 127.0.0.1:6700 \
  --ephemeral-url http://127.0.0.1:7799 \
  --ephemeral-url-ws ws://127.0.0.1:7800 &
qfs_pid=$!
./scripts/wait-rpc.sh http://127.0.0.1:6699

echo "stack-local: ready (Ctrl-C stops all nodes)"

monitor_stack() {
  while :; do
    for service in "base:${base_pid}" "ER:${er_pid}" "QFS:${qfs_pid}"; do
      name="${service%%:*}"
      pid="${service#*:}"
      state="$(ps -o stat= -p "$pid" 2>/dev/null || true)"
      if [[ -z "$state" || "$state" == *Z* ]]; then
        echo "stack-local: ${name} exited; stopping remaining services" >&2
        return 1
      fi
    done
    sleep 1
  done
}

monitor_stack
