#!/bin/bash
# scripts/test-reconnect.sh — Reconnection and auth tests
set +e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/test-lib.sh"
export GLIDE_TEST_MANAGED_SERVER="${GLIDE_TEST_MANAGED_SERVER:-1}"
start_managed_server || exit 1
set +e

echo "=== Reconnection & Auth Tests ==="
python3 -c "
import asyncio, json, os, sys, websockets, urllib.request, time

SERVER = os.environ['GLIDE_SERVER']
WS_SERVER = os.environ['GLIDE_WS_SERVER']
PASS = FAIL = 0

def check(name, cond):
    global PASS, FAIL
    if cond: print(f'  ✅ {name}'); PASS += 1
    else: print(f'  ❌ {name}'); FAIL += 1

async def test():
    global PASS, FAIL
    
    # Test 1: Auth - valid registration token
    print('--- Authentication ---')
    try:
        req = urllib.request.Request(f'{SERVER}/api/v1/devices/register',
            data=json.dumps({'device_id':'auth-ok','name':'AuthOK','platform':'linux','registration_token':'reg123'}).encode(),
            headers={'Content-Type':'application/json'})
        urllib.request.urlopen(req)
        check('Valid token accepted', True)
    except: check('Valid token accepted', False)
    
    # Test 2: Auth - invalid token
    try:
        req = urllib.request.Request(f'{SERVER}/api/v1/devices/register',
            data=json.dumps({'device_id':'auth-bad','name':'AuthBad','registration_token':'wrong'}).encode(),
            headers={'Content-Type':'application/json'})
        urllib.request.urlopen(req)
        check('Invalid token rejected', False)
    except: check('Invalid token rejected', True)
    
    # Test 3: Auth - no token field
    try:
        req = urllib.request.Request(f'{SERVER}/api/v1/devices/register',
            data=json.dumps({'device_id':'auth-none','name':'AuthNone'}).encode(),
            headers={'Content-Type':'application/json'})
        urllib.request.urlopen(req)
        check('Missing token rejected', False)
    except: check('Missing token rejected', True)
    
    # Test 4: WebSocket reconnect after close
    print()
    print('--- Reconnection ---')
    ws = await websockets.connect(f'{WS_SERVER}/ws/sync?device_id=recon-1')
    await ws.send(json.dumps({'event_type':'DeviceJoined','data':{'device_id':'recon-1','name':'Recon'}}))
    await ws.close()
    
    ws2 = await websockets.connect(f'{WS_SERVER}/ws/sync?device_id=recon-1')
    await ws2.send(json.dumps({'event_type':'DeviceJoined','data':{'device_id':'recon-1','name':'Recon2'}}))
    check('Reconnect after close', True)
    await ws2.close()
    
    # Test 5: Multiple clients connect simultaneously
    print()
    print('--- Multi-client ---')
    clients = []
    for i in range(5):
        ws = await websockets.connect(f'{WS_SERVER}/ws/sync?device_id=multi-{i}')
        await ws.send(json.dumps({'event_type':'DeviceJoined','data':{'device_id':f'multi-{i}','name':f'Multi{i}'}}))
        clients.append(ws)
    check('5 simultaneous clients', True)
    
    # Verify sync works between them
    ws_sender = clients[0]
    ws_receiver = clients[4]
    
    await ws_sender.send(json.dumps({'event_type':'ClipboardCaptured','data':{'item':{
        'item_id':'multi-sync','source_device_id':'multi-0','source_session_type':'Persistent',
        'kind':'Text','representations':[{'mime_type':'text/plain','content':{'Text':'multi-sync-test'}}],
        'size':14,'created_at':0,'payload_refs':[],'checksum':'abc','delivery_policy':'Broadcast'
    }}}))
    
    found = False
    for _ in range(10):
        try:
            msg = await asyncio.wait_for(ws_receiver.recv(), timeout=2)
            d = json.loads(msg)
            if d.get('event_type') == 'ClipboardCaptured' and d['data']['item']['item_id'] == 'multi-sync':
                check('Sync between 5 clients', True)
                found = True; break
        except asyncio.TimeoutError: break
    if not found: check('Sync between 5 clients', False)
    
    for ws in clients: await ws.close()
    
    # Test 6: Device list has all registered devices
    print()
    print('--- Device Registry ---')
    resp = urllib.request.urlopen(f'{SERVER}/api/v1/devices')
    data = json.loads(resp.read())
    device_count = len(data.get('devices', []))
    check('Devices registered', device_count >= 2)
    
    print(f'\n  Results: {PASS} passed, {FAIL} failed')
    return FAIL

sys.exit(1 if asyncio.run(test()) else 0)
" 2>&1
exit ${PIPESTATUS[0]}
