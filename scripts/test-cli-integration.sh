#!/bin/bash
# scripts/test-cli-integration.sh — CLI copy/paste integration tests with server
set +e
PASS=0; FAIL=0
SERVER="http://localhost:8080"
check() { if [ "$2" = "0" ]; then echo "  ✅ $1"; PASS=$((PASS+1)); else echo "  ❌ $1"; FAIL=$((FAIL+1)); fi }

echo "=== CLI Integration Tests ==="

# Ensure server is running
curl -sf "$SERVER/api/v1/health" > /dev/null 2>&1
check "Server running" $?

# Test 1: Register a device via API and verify
echo ""
echo "--- Device Registration ---"
curl -sf -X POST "$SERVER/api/v1/devices/register" \
  -H "Content-Type: application/json" \
  -d '{"device_id":"cli-test-1","name":"CLI-Test","platform":"linux","registration_token":"reg123"}' > /dev/null 2>&1
check "Register CLI test device" $?

# Test 2: WebSocket clipboard sync (simulates what CLI does)
echo ""
echo "--- CLI Clipboard Sync ---"
python3 -c "
import asyncio, json, websockets, urllib.request

SERVER = '$SERVER'

async def test():
    # Sender connects
    ws_s = await websockets.connect(f'ws://localhost:8080/ws/sync?device_id=cli-sender')
    await ws_s.send(json.dumps({'event_type':'DeviceJoined','data':{'device_id':'cli-sender','name':'CLI-Sender'}}))

    # Receiver connects
    ws_r = await websockets.connect(f'ws://localhost:8080/ws/sync?device_id=cli-receiver')
    await ws_r.send(json.dumps({'event_type':'DeviceJoined','data':{'device_id':'cli-receiver','name':'CLI-Receiver'}}))
    await asyncio.sleep(0.3)

    tests = [
        ('English text', 'Hello from CLI!'),
        ('Chinese', '你好世界 CLI 测试'),
        ('Emoji', '🚀🔥✅🎉'),
        ('Multiline', 'line1\nline2\nline3'),
        ('Empty', ''),
        ('Large 500KB', 'x' * 500000),
        ('Special chars', '<script>alert(1)</script> & \"quotes\"'),
    ]

    passed = failed = 0
    for label, text in tests:
        item_id = f'cli-{label}'
        event = {'event_type':'ClipboardCaptured','data':{'item':{
            'item_id':item_id,'source_device_id':'cli-sender','source_session_type':'Persistent',
            'kind':'Text','representations':[{'mime_type':'text/plain','content':{'Text':text}}],
            'size':len(text.encode('utf-8')),'created_at':0,'payload_refs':[],'checksum':'abc','delivery_policy':'Broadcast'
        }}}
        await ws_s.send(json.dumps(event))
        found = False
        for _ in range(15):
            try:
                msg = await asyncio.wait_for(ws_r.recv(), timeout=3)
                d = json.loads(msg)
                if d.get('event_type') == 'ClipboardCaptured' and d['data']['item']['item_id'] == item_id:
                    actual = d['data']['item']['representations'][0]['content']['Text']
                    if actual == text:
                        print(f'  ✅ CLI sync: {label}')
                        passed += 1
                    else:
                        print(f'  ❌ CLI sync: {label} (mismatch)')
                        failed += 1
                    found = True
                    break
            except asyncio.TimeoutError:
                break
        if not found:
            print(f'  ❌ CLI sync: {label} (timeout)')
            failed += 1

    # Bidirectional: receiver sends back
    await ws_r.send(json.dumps({'event_type':'ClipboardCaptured','data':{'item':{
        'item_id':'cli-reverse','source_device_id':'cli-receiver','source_session_type':'Persistent',
        'kind':'Text','representations':[{'mime_type':'text/plain','content':{'Text':'reply from receiver'}}],
        'size':19,'created_at':0,'payload_refs':[],'checksum':'abc','delivery_policy':'Broadcast'
    }}}))
    found = False
    for _ in range(10):
        try:
            msg = await asyncio.wait_for(ws_s.recv(), timeout=2)
            d = json.loads(msg)
            if d.get('event_type') == 'ClipboardCaptured' and d['data']['item']['item_id'] == 'cli-reverse':
                print('  ✅ CLI sync: reverse (receiver→sender)')
                passed += 1
                found = True
                break
        except asyncio.TimeoutError:
            break
    if not found:
        print('  ❌ CLI sync: reverse (timeout)')
        failed += 1

    # Loop prevention
    await ws_s.send(json.dumps({'event_type':'ClipboardCaptured','data':{'item':{
        'item_id':'cli-loop','source_device_id':'cli-sender','source_session_type':'Persistent',
        'kind':'Text','representations':[{'mime_type':'text/plain','content':{'Text':'echo'}}],
        'size':4,'created_at':0,'payload_refs':[],'checksum':'abc','delivery_policy':'Broadcast'
    }}}))
    got_echo = False
    for _ in range(5):
        try:
            msg = await asyncio.wait_for(ws_s.recv(), timeout=1)
            d = json.loads(msg)
            if d.get('event_type') == 'ClipboardCaptured':
                got_echo = True
                break
        except asyncio.TimeoutError:
            break
    if not got_echo:
        print('  ✅ Loop prevention')
        passed += 1
    else:
        print('  ❌ Loop prevention (got echo)')
        failed += 1

    await ws_s.close()
    await ws_r.close()
    print(f'\n  Results: {passed} passed, {failed} failed')

asyncio.run(test())
" 2>&1

echo ""
echo "========================================"
echo "CLI integration tests: $PASS passed, $FAIL failed"
echo "========================================"
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
