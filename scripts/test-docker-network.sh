#!/bin/bash
# scripts/test-docker-network.sh — Docker network isolation and cross-container tests
set +e
PASS=0; FAIL=0
check() { if [ "$2" = "0" ]; then echo "  ✅ $1"; PASS=$((PASS+1)); else echo "  ❌ $1"; FAIL=$((FAIL+1)); fi }
echo "=== Docker Network Tests ==="

# Test 1: Server running
echo "--- Server Status ---"
docker ps --filter name=glide-server --format "{{.Names}}" | grep -q glide-server
check "Server container running" $?

# Test 2: Cross-container connectivity via host network
echo ""
echo "--- Cross-Container Connectivity ---"
docker run --rm --network host python:3.12-slim python3 -c "
import urllib.request, json
resp = urllib.request.urlopen('http://localhost:8080/api/v1/health', timeout=5)
data = json.loads(resp.read())
assert data.get('status') == 'ok'
" 2>&1
check "Cross-container health check (host network)" $?

# Test 3: Cross-container via bridge gateway
GATEWAY=$(docker network inspect bridge --format '{{range .IPAM.Config}}{{.Gateway}}{{end}}' 2>/dev/null)
if [ -n "$GATEWAY" ]; then
    docker run --rm python:3.12-slim python3 -c "
import urllib.request, json
resp = urllib.request.urlopen('http://${GATEWAY}:8080/api/v1/health', timeout=5)
data = json.loads(resp.read())
assert data.get('status') == 'ok'
" 2>&1
    check "Cross-container health check (bridge gateway)" $?
else
    echo "  ⏭ Bridge gateway not found"
fi

# Test 4: Container-to-container clipboard sync
echo ""
echo "--- Multi-Container Sync ---"
docker run --rm --network host python:3.12-slim sh -c "pip install websockets -q 2>/dev/null && python3 -c \"
import asyncio, json, websockets
async def test():
    ws_b = await websockets.connect('ws://localhost:8080/ws/sync?device_id=docker-b')
    await ws_b.send(json.dumps({'event_type':'DeviceJoined','data':{'device_id':'docker-b','name':'Docker-B'}}))
    ws_a = await websockets.connect('ws://localhost:8080/ws/sync?device_id=docker-a')
    await ws_a.send(json.dumps({'event_type':'DeviceJoined','data':{'device_id':'docker-a','name':'Docker-A'}}))
    await asyncio.sleep(0.5)
    await ws_a.send(json.dumps({'event_type':'ClipboardCaptured','data':{'item':{
        'item_id':'docker-sync','source_device_id':'docker-a','source_session_type':'Persistent',
        'kind':'Text','representations':[{'mime_type':'text/plain','content':{'Text':'cross-container'}}],
        'size':17,'created_at':0,'payload_refs':[],'checksum':'abc','delivery_policy':'Broadcast'
    }}}))
    found = False
    for _ in range(15):
        try:
            msg = await asyncio.wait_for(ws_b.recv(), timeout=2)
            d = json.loads(msg)
            if d.get('event_type') == 'ClipboardCaptured':
                found = True; break
        except: break
    await ws_a.close(); await ws_b.close()
    print('OK' if found else 'FAIL')
asyncio.run(test())
\"" 2>&1 | tail -1 | grep -q "OK"
check "Cross-container clipboard sync" $?

echo ""
echo "========================================"
echo "Docker network tests: $PASS passed, $FAIL failed"
echo "========================================"
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
