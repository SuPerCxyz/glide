#!/bin/bash
# scripts/test-clipboard-cli.sh — Linux CLI clipboard tests
set +e
PASS=0; FAIL=0
check() { if [ "$2" = "0" ]; then echo "  ✅ $1"; PASS=$((PASS+1)); else echo "  ❌ $1"; FAIL=$((FAIL+1)); fi }
echo "=== Linux CLI Clipboard Tests ==="
python3 -c "
import json, urllib.request, asyncio, websockets
SERVER='http://localhost:8080'
async def test():
    ws = await websockets.connect('ws://localhost:8080/ws/sync?device_id=cli-headless')
    await ws.send(json.dumps({'event_type':'DeviceJoined','data':{'device_id':'cli-headless','name':'Headless'}}))
    
    tests = [
        ('Plain text', 'Hello CLI'),
        ('Chinese', '你好世界 🎉'),
        ('Emoji', '🚀🔥💡'),
        ('Empty', ''),
        ('Multiline', 'a\nb\nc'),
        ('Special chars', '<b>&\"'),
    ]
    passed = failed = 0
    for label, text in tests:
        item_id = f'cli-{label}'
        event = {'event_type':'ClipboardCaptured','data':{'item':{
            'item_id':item_id,'source_device_id':'cli-headless','source_session_type':'Persistent',
            'kind':'Text','representations':[{'mime_type':'text/plain','content':{'Text':text}}],
            'size':len(text),'created_at':0,'payload_refs':[],'checksum':'abc','delivery_policy':'Broadcast'
        }}}
        await ws.send(json.dumps(event))
        await asyncio.sleep(0.5)
        
        # Verify via WebSocket: another client should receive it
        ws2 = await websockets.connect('ws://localhost:8080/ws/sync?device_id=cli-receiver')
        await ws2.send(json.dumps({'event_type':'DeviceJoined','data':{'device_id':'cli-receiver','name':'Receiver'}}))
        # Re-send from headless to trigger broadcast to receiver
        await ws.send(json.dumps(event))
        
        found = False
        for _ in range(10):
            try:
                msg = await asyncio.wait_for(ws2.recv(), timeout=2)
                d = json.loads(msg)
                if d.get('event_type') == 'ClipboardCaptured' and d['data']['item']['item_id'] == item_id:
                    actual = d['data']['item']['representations'][0]['content']['Text']
                    if actual == text:
                        print(f'  ✅ Sync: {label}')
                        passed += 1
                    else:
                        print(f'  ❌ Sync: {label} (mismatch)')
                        failed += 1
                    found = True
                    break
            except asyncio.TimeoutError:
                break
        if not found:
            print(f'  ❌ Sync: {label} (not received)')
            failed += 1
        await ws2.close()
    
    # Large text
    text = 'x' * 500000
    ws2 = await websockets.connect('ws://localhost:8080/ws/sync?device_id=cli-receiver2')
    await ws2.send(json.dumps({'event_type':'DeviceJoined','data':{'device_id':'cli-receiver2','name':'R2'}}))
    await ws.send(json.dumps({'event_type':'ClipboardCaptured','data':{'item':{
        'item_id':'cli-large','source_device_id':'cli-headless','source_session_type':'Persistent',
        'kind':'Text','representations':[{'mime_type':'text/plain','content':{'Text':text}}],
        'size':len(text),'created_at':0,'payload_refs':[],'checksum':'abc','delivery_policy':'Broadcast'
    }}}))
    try:
        msg = await asyncio.wait_for(ws2.recv(), timeout=5)
        d = json.loads(msg)
        if d.get('event_type') == 'ClipboardCaptured' and len(d['data']['item']['representations'][0]['content']['Text']) == 500000:
            print('  ✅ Large text (500KB)'); passed += 1
        else:
            print('  ❌ Large text (500KB)'); failed += 1
    except asyncio.TimeoutError:
        print('  ❌ Large text (500KB) timeout'); failed += 1
    await ws2.close()
    await ws.close()
    print(f'\n  Results: {passed} passed, {failed} failed')
asyncio.run(test())
" 2>&1
