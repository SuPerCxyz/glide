#!/usr/bin/env bash
set -euo pipefail

managed_server_pid=""
managed_data_dir=""

pick_test_port() {
  python3 - <<'PY'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
}

wait_for_health() {
  local server="$1"
  for _ in $(seq 1 80); do
    if curl -sf "$server/api/v1/health" >/dev/null 2>&1; then
      return 0
    fi
    sleep 0.25
  done
  return 1
}

start_managed_server() {
  if [ -n "${GLIDE_SERVER:-}" ] && [ "${GLIDE_TEST_MANAGED_SERVER:-0}" != "1" ]; then
    export GLIDE_WS_SERVER="${GLIDE_WS_SERVER:-${GLIDE_SERVER/http:/ws:}}"
    export GLIDE_WS_SERVER="${GLIDE_WS_SERVER/https:/wss:}"
    return 0
  fi

  cargo build --package glide-server >/dev/null

  local port="${GLIDE_TEST_PORT:-$(pick_test_port)}"
  managed_data_dir="$(mktemp -d)"
  export GLIDE_SERVER="http://127.0.0.1:${port}"
  export GLIDE_WS_SERVER="ws://127.0.0.1:${port}"
  export GLIDE_REGISTRATION_TOKEN="${GLIDE_REGISTRATION_TOKEN:-reg123}"
  export GLIDE_LISTEN_ADDR="127.0.0.1:${port}"
  export GLIDE_DATA_DIR="$managed_data_dir"
  export RUST_LOG="${RUST_LOG:-info}"

  target/debug/glide-server >"$managed_data_dir/server.log" 2>&1 &
  managed_server_pid="$!"

  if ! wait_for_health "$GLIDE_SERVER"; then
    echo "Managed Glide server failed to start on $GLIDE_LISTEN_ADDR" >&2
    cat "$managed_data_dir/server.log" >&2 || true
    return 1
  fi
}

stop_managed_server() {
  if [ -n "${managed_server_pid:-}" ]; then
    kill "$managed_server_pid" >/dev/null 2>&1 || true
    wait "$managed_server_pid" >/dev/null 2>&1 || true
  fi
  if [ -n "${managed_data_dir:-}" ]; then
    rm -rf "$managed_data_dir"
  fi
}

trap stop_managed_server EXIT
