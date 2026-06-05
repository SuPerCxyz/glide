#!/bin/bash
# scripts/test-tc-network.sh — Network anomaly tests using an isolated server.
set +e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/test-lib.sh"
export GLIDE_TEST_MANAGED_SERVER="${GLIDE_TEST_MANAGED_SERVER:-1}"
start_managed_server || exit 1
set +e

SERVER=${GLIDE_SERVER}
WS_SERVER=${GLIDE_WS_SERVER}
SERVER_PORT=${SERVER##*:}
SERVER_PORT=${SERVER_PORT%%/*}
PASS=0
FAIL=0

check() {
  if [ "$2" = "0" ]; then
    echo "  ✅ $1"
    PASS=$((PASS + 1))
  else
    echo "  ❌ $1"
    FAIL=$((FAIL + 1))
  fi
}

echo "=== Network Anomaly Tests ==="
echo "Server: $SERVER"

echo "--- Server Restart ---"
curl -sf "$SERVER/api/v1/health" >/dev/null 2>&1
check "Server health before restart" $?

kill "$managed_server_pid" >/dev/null 2>&1 || true
wait "$managed_server_pid" >/dev/null 2>&1 || true
target/debug/glide-server >>"$managed_data_dir/server.log" 2>&1 &
managed_server_pid="$!"
wait_for_health "$SERVER"
check "Server health after restart" $?

echo ""
echo "--- Connection Timeout ---"
python3 - <<'PY'
import asyncio
import json
import os
import websockets

async def main():
    ws_server = os.environ["GLIDE_WS_SERVER"]
    try:
        await asyncio.wait_for(
            websockets.connect("ws://10.255.255.1:9999/ws/sync", open_timeout=2),
            timeout=3,
        )
        raise SystemExit(1)
    except (asyncio.TimeoutError, ConnectionRefusedError, OSError):
        pass

    ws = await websockets.connect(f"{ws_server}/ws/sync?device_id=recon-test")
    await ws.send(json.dumps({"event_type":"DeviceJoined","data":{"device_id":"recon-test","name":"Recon"}}))
    await ws.close()

asyncio.run(main())
PY
check "Bad IP times out and reconnect works" $?

echo ""
echo "--- Rapid Connect/Disconnect ---"
python3 - <<'PY'
import asyncio
import json
import os
import websockets

async def main():
    ws_server = os.environ["GLIDE_WS_SERVER"]
    passed = 0
    for i in range(10):
        try:
            ws = await websockets.connect(f"{ws_server}/ws/sync?device_id=rapid-{i}")
            await ws.send(json.dumps({"event_type":"DeviceJoined","data":{"device_id":f"rapid-{i}","name":f"Rapid{i}"}}))
            await ws.close()
            passed += 1
        except Exception:
            pass
    if passed < 8:
        raise SystemExit(1)

asyncio.run(main())
PY
check "Rapid connect/disconnect >= 8/10" $?

echo ""
echo "--- Large Payload ---"
python3 - <<'PY'
import asyncio
import json
import os
import websockets

async def main():
    ws_server = os.environ["GLIDE_WS_SERVER"]
    ws = await websockets.connect(f"{ws_server}/ws/sync?device_id=large-payload")
    await ws.send(json.dumps({"event_type":"DeviceJoined","data":{"device_id":"large-payload","name":"LargePayload"}}))
    text = "A" * 1000000
    event = {"event_type":"ClipboardCaptured","data":{"item":{
        "item_id":"large-1mb","source_device_id":"large-payload","source_session_type":"Persistent",
        "kind":"Text","representations":[{"mime_type":"text/plain","content":{"Text":text}}],
        "size":len(text),"created_at":0,"payload_refs":[],"checksum":"abc","delivery_policy":"Broadcast"
    }}}
    await asyncio.wait_for(ws.send(json.dumps(event)), timeout=10)
    await ws.close()

asyncio.run(main())
PY
check "1MB payload sent" $?

echo ""
echo "--- IPv4/Port Binding ---"
curl -4 -sf "$SERVER/api/v1/health" >/dev/null 2>&1
check "IPv4 loopback" $?

ss -lntup 2>/dev/null | grep "$SERVER_PORT" | grep -q "127.0.0.1"
check "Bound to isolated loopback port" $?

echo ""
echo "========================================"
echo "Network anomaly tests: $PASS passed, $FAIL failed"
echo "========================================"
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
