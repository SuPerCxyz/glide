#!/bin/bash
# scripts/test-network.sh — Network anomaly tests for Glide server.
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
PASS=0; FAIL=0

check() {
    if [ "$2" = "0" ]; then
        echo "  ✅ $1"; PASS=$((PASS+1))
    else
        echo "  ❌ $1"; FAIL=$((FAIL+1))
    fi
}

echo "=== Glide Network Tests ==="
echo "Server: $SERVER"
echo ""

# Phase 1: Server Reachability
echo "--- Server Reachability ---"
curl -sf "$SERVER/api/v1/health" > /dev/null 2>&1; check "Health endpoint reachable" $?
curl -4 -sf "$SERVER/api/v1/health" > /dev/null 2>&1; check "IPv4 connection" $?
curl -sf "http://localhost:9999/api/v1/health" > /dev/null 2>&1; RESULT=$?
[ "$RESULT" -ne 0 ]; check "Wrong port rejected" $?

# Phase 2: Port binding
echo ""
echo "--- Port Binding ---"
ss -lntup 2>/dev/null | grep -q "$SERVER_PORT"; check "Port $SERVER_PORT listening" $?
ss -lntup 2>/dev/null | grep "$SERVER_PORT" | grep -q "127.0.0.1"; check "Bound to managed loopback" $?
ss -lntup 2>/dev/null | grep "$SERVER_PORT" | grep -q "glide-server"; check "Glide server owns port" $?

# Phase 3: Registration
echo ""
echo "--- Registration ---"
curl -sf -X POST "$SERVER/api/v1/devices/register" \
  -H "Content-Type: application/json" \
  -d '{"device_id":"net-1","name":"NetTest","platform":"linux","registration_token":"reg123"}' > /dev/null 2>&1
check "Valid registration" $?

curl -sf -X POST "$SERVER/api/v1/devices/register" \
  -H "Content-Type: application/json" \
  -d '{"device_id":"net-2","name":"NetTest","registration_token":"wrong"}' > /dev/null 2>&1
RESULT=$?; [ "$RESULT" -ne 0 ]; check "Bad token rejected" $?

# Phase 4: API
echo ""
echo "--- API Endpoints ---"
curl -sf "$SERVER/api/v1/devices" > /dev/null 2>&1; check "GET /devices" $?
curl -sf "$SERVER/api/v1/clipboard/history" > /dev/null 2>&1; check "GET /clipboard/history" $?

# Phase 5: WebSocket
echo ""
echo "--- WebSocket ---"
python3 -c "
import asyncio, websockets, json
async def t():
    ws = await websockets.connect('${WS_SERVER}/ws/sync?device_id=ws-net')
    await ws.send(json.dumps({'event_type':'DeviceJoined','data':{'device_id':'ws-net','name':'Test'}}))
    await ws.close()
asyncio.run(t())
" > /dev/null 2>&1; check "WebSocket connect/disconnect" $?

# Summary
echo ""
echo "========================================"
echo "Network tests: $PASS passed, $FAIL failed"
echo "========================================"
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
