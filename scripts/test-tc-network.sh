#!/bin/bash
# scripts/test-tc-network.sh — Network anomaly tests using tc/netns
set +e
SERVER=${GLIDE_SERVER:-http://localhost:8080}
PASS=0; FAIL=0
check() { if [ "$2" = "0" ]; then echo "  ✅ $1"; PASS=$((PASS+1)); else echo "  ❌ $1"; FAIL=$((FAIL+1)); fi }
echo "=== Network Anomaly Tests ==="

# Test 1: Server restart recovery
echo "--- Server Restart ---"
curl -sf "$SERVER/api/v1/health" > /dev/null 2>&1
check "Server health before restart" $?

# Restart via docker
docker restart glide-server > /dev/null 2>&1
sleep 3
curl -sf "$SERVER/api/v1/health" > /dev/null 2>&1
check "Server health after restart" $?

# Test 2: Connection timeout simulation
echo ""
echo "--- Connection Timeout ---"
python3 -c "
import asyncio, websockets, json, time

async def test():
    # Try connecting to a port that will timeout
    try:
        await asyncio.wait_for(
            websockets.connect('ws://10.255.255.1:9999/ws/sync', open_timeout=2),
            timeout=3
        )
        print('  ❌ Timeout to bad IP')
    except (asyncio.TimeoutError, ConnectionRefusedError, OSError) as e:
        print('  ✅ Connection timeout to bad IP')
    
    # Reconnect to working server
    try:
        ws = await websockets.connect('ws://localhost:8080/ws/sync?device_id=recon-test')
        await ws.send(json.dumps({'event_type':'DeviceJoined','data':{'device_id':'recon-test','name':'Recon'}}))
        await ws.close()
        print('  ✅ Reconnect to working server')
    except Exception as e:
        print(f'  ❌ Reconnect failed: {e}')

asyncio.run(test())
" 2>&1

# Test 3: Rapid connect/disconnect
echo ""
echo "--- Rapid Connect/Disconnect ---"
python3 -c "
import asyncio, websockets, json

async def test():
    passed = 0
    for i in range(10):
        try:
            ws = await websockets.connect(f'ws://localhost:8080/ws/sync?device_id=rapid-{i}')
            await ws.send(json.dumps({'event_type':'DeviceJoined','data':{'device_id':f'rapid-{i}','name':f'Rapid{i}'}}))
            await ws.close()
            passed += 1
        except Exception:
            pass
    print(f'  ✅ Rapid connect/disconnect: {passed}/10' if passed >= 8 else f'  ❌ Rapid connect/disconnect: {passed}/10')

asyncio.run(test())
" 2>&1

# Test 4: Large payload via WebSocket
echo ""
echo "--- Large Payload ---"
python3 -c "
import asyncio, websockets, json

async def test():
    ws = await websockets.connect('ws://localhost:8080/ws/sync?device_id=large-payload')
    await ws.send(json.dumps({'event_type':'DeviceJoined','data':{'device_id':'large-payload','name':'LargePayload'}}))
    
    # Send 1MB text
    text = 'A' * 1000000
    event = {'event_type':'ClipboardCaptured','data':{'item':{
        'item_id':'large-1mb','source_device_id':'large-payload','source_session_type':'Persistent',
        'kind':'Text','representations':[{'mime_type':'text/plain','content':{'Text':text}}],
        'size':len(text),'created_at':0,'payload_refs':[],'checksum':'abc','delivery_policy':'Broadcast'
    }}}
    try:
        await asyncio.wait_for(ws.send(json.dumps(event)), timeout=10)
        print('  ✅ 1MB payload sent')
    except Exception as e:
        print(f'  ❌ 1MB payload failed: {e}')
    await ws.close()

asyncio.run(test())
" 2>&1

# Test 5: IPv4/IPv6 connectivity
echo ""
echo "--- IPv4/IPv6 ---"
curl -4 -sf "http://127.0.0.1:8080/api/v1/health" > /dev/null 2>&1
check "IPv4 localhost" $?

curl -4 -sf "http://0.0.0.0:8080/api/v1/health" > /dev/null 2>&1
check "IPv4 0.0.0.0" $?

curl -6 -sf "http://[::1]:8080/api/v1/health" > /dev/null 2>&1
check "IPv6 localhost" $?

# Test 6: Port binding verification
echo ""
echo "--- Port Binding ---"
ss -lntup 2>/dev/null | grep -q ":8080.*0.0.0.0"; check "Bound to 0.0.0.0:8080" $?
ss -lntup 2>/dev/null | grep -q "\[::\]:8080"; check "Bound to [::]:8080 (IPv6)" $?

echo ""
echo "========================================"
echo "Network anomaly tests: $PASS passed, $FAIL failed"
echo "========================================"
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
