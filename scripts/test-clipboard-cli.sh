#!/bin/bash
# scripts/test-clipboard-cli.sh — Linux CLI clipboard tests
set +e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/test-lib.sh"
export GLIDE_TEST_MANAGED_SERVER="${GLIDE_TEST_MANAGED_SERVER:-1}"
start_managed_server || exit 1
set +e

echo "=== Linux CLI Clipboard Tests ==="
python3 -c "
import json, os, sys, urllib.request, asyncio, websockets

WS_SERVER = os.environ['GLIDE_WS_SERVER']

async def test():
    # Connect receiver FIRST, then sender
    ws_r = await websockets.connect(f'{WS_SERVER}/ws/sync?device_id=cli-receiver')
    await ws_r.send(json.dumps({'event_type':'DeviceJoined','data':{'device_id':'cli-receiver','name':'Receiver'}}))
    
    ws_s = await websockets.connect(f'{WS_SERVER}/ws/sync?device_id=cli-sender')
    await ws_s.send(json.dumps({'event_type':'DeviceJoined','data':{'device_id':'cli-sender','name':'Sender'}}))
    await asyncio.sleep(0.3)
    
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
        event = {'event_type':'ClipboardCaptured','data':{'item':{
            'item_id':f'cli-{label}','source_device_id':'cli-sender','source_session_type':'Persistent',
            'kind':'Text','representations':[{'mime_type':'text/plain','content':{'Text':text}}],
            'size':len(text.encode('utf-8')),'created_at':0,'payload_refs':[],'checksum':'abc','delivery_policy':'Broadcast'
        }}}
        await ws_s.send(json.dumps(event))
        found = False
        for _ in range(10):
            try:
                msg = await asyncio.wait_for(ws_r.recv(), timeout=2)
                d = json.loads(msg)
                if d.get('event_type') == 'ClipboardCaptured' and d['data']['item']['item_id'] == f'cli-{label}':
                    actual = d['data']['item']['representations'][0]['content']['Text']
                    if actual == text:
                        print(f'  ✅ Sync: {label}'); passed += 1
                    else:
                        print(f'  ❌ Sync: {label} (mismatch: {repr(actual)[:30]})'); failed += 1
                    found = True; break
            except asyncio.TimeoutError: break
        if not found: print(f'  ❌ Sync: {label} (timeout)'); failed += 1
    
    # Large text
    text = 'x' * 500000
    await ws_s.send(json.dumps({'event_type':'ClipboardCaptured','data':{'item':{
        'item_id':'cli-large','source_device_id':'cli-sender','source_session_type':'Persistent',
        'kind':'Text','representations':[{'mime_type':'text/plain','content':{'Text':text}}],
        'size':len(text),'created_at':0,'payload_refs':[],'checksum':'abc','delivery_policy':'Broadcast'
    }}}))
    try:
        msg = await asyncio.wait_for(ws_r.recv(), timeout=5)
        d = json.loads(msg)
        if d.get('event_type') == 'ClipboardCaptured' and len(d['data']['item']['representations'][0]['content']['Text']) == 500000:
            print('  ✅ Large text (500KB)'); passed += 1
        else:
            print('  ❌ Large text (500KB)'); failed += 1
    except asyncio.TimeoutError:
        print('  ❌ Large text (500KB) timeout'); failed += 1
    
    await ws_s.close(); await ws_r.close()
    print(f'\n  Results: {passed} passed, {failed} failed')
    return failed
sys.exit(1 if asyncio.run(test()) else 0)
" 2>&1
exit ${PIPESTATUS[0]}
