#!/usr/bin/env bash
# Wait until a Solana-ish JSON-RPC endpoint answers getVersion or getHealth.
set -euo pipefail
url="${1:?usage: wait-rpc.sh <url> [timeout_seconds]}"
timeout_s="${2:-180}"

payload() {
  printf '{"jsonrpc":"2.0","id":1,"method":"%s","params":[]}' "$1"
}

up() {
  local body
  body="$(curl -sf --max-time 2 -X POST "$url" \
    -H 'content-type: application/json' \
    -d "$(payload getVersion)" 2>/dev/null || true)"
  if [[ "$body" == *result* ]]; then
    return 0
  fi
  body="$(curl -sf --max-time 2 -X POST "$url" \
    -H 'content-type: application/json' \
    -d "$(payload getHealth)" 2>/dev/null || true)"
  [[ "$body" == *result* || "$body" == *ok* ]]
}

i=0
while (( i < timeout_s )); do
  if up; then
    echo "wait-rpc: $url is up (${i}s)"
    exit 0
  fi
  i=$((i + 1))
  sleep 1
done
echo "error: $url did not come up in ${timeout_s}s" >&2
exit 1
