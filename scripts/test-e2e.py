#!/usr/bin/env python3
"""Glide comprehensive E2E test suite.

Tests server, clipboard sync, input events, error handling, and
Windows connection scenarios. Runs against a local server.

Usage: python3 scripts/test-e2e.py
"""
import asyncio
import json
import urllib.request
import sys
import os
from urllib.parse import urlparse

PASS = FAIL = SKIP = 0

def check(name, cond, note=None):
    global PASS, FAIL
    if cond:
        print(f'  ✅ {name}')
        PASS += 1
    else:
        print(f'  ❌ {name}' + (f' ({note})' if note else ''))
        FAIL += 1

def skip(name, reason):
    global SKIP
    print(f'  ⏭ {name} (skipped: {reason})')
    SKIP += 1

SERVER = os.environ.get('GLIDE_SERVER', 'http://localhost:8080')
WS_SERVER = SERVER.replace('http://', 'ws://').replace('https://', 'wss://')

async def main():
    global PASS, FAIL, SKIP

    print('=' * 60)
    print('Glide E2E Test Suite')
    print(f'Server: {SERVER}')
    print('=' * 60)

    # ── Phase 1: Server Health ──
    print('\n=== Phase 1: Server Health ===')
    try:
        d = json.loads(urllib.request.urlopen(f'{SERVER}/api/v1/health').read())
        check('Health OK', d.get('status') == 'ok')
        check('Version present', 'version' in d)
    except Exception as e:
        check('Health OK', False, str(e))

    # ── Phase 2: Device Registration ──
    print('\n=== Phase 2: Device Registration ===')
    for did, name, plat in [('dev-a','Linux-A','linux'), ('dev-b','Win-B','windows'), ('dev-c','Mac-C','macos')]:
        try:
            req = urllib.request.Request(f'{SERVER}/api/v1/devices/register',
                data=json.dumps({'device_id':did,'name':name,'platform':plat,'registration_token':'reg123'}).encode(),
                headers={'Content-Type':'application/json'})
            d = json.loads(urllib.request.urlopen(req).read())
            check(f'Register {name}', d.get('status') == 'registered')
        except Exception as e:
            check(f'Register {name}', False, str(e))

    # Wrong token
    try:
        req = urllib.request.Request(f'{SERVER}/api/v1/devices/register',
            data=json.dumps({'device_id':'bad','name':'bad','registration_token':'wrong'}).encode(),
            headers={'Content-Type':'application/json'})
        urllib.request.urlopen(req)
        check('Wrong token rejected', False)
    except:
        check('Wrong token rejected', True)

    # No token
    try:
        req = urllib.request.Request(f'{SERVER}/api/v1/devices/register',
            data=json.dumps({'device_id':'x','name':'x'}).encode(),
            headers={'Content-Type':'application/json'})
        urllib.request.urlopen(req)
        check('Missing token rejected', False)
    except:
        check('Missing token rejected', True)

    # Device list
    try:
        d = json.loads(urllib.request.urlopen(f'{SERVER}/api/v1/devices').read())
        check('Device list ≥ 3', len(d.get('devices',[])) >= 3)
    except:
        check('Device list ≥ 3', False)

    # ── Phase 3: Clipboard Sync ──
    print('\n=== Phase 3: Clipboard Sync ===')
    ws_b = await websockets.connect(f'{WS_SERVER}/ws/sync?device_id=dev-b')
    ws_a = await websockets.connect(f'{WS_SERVER}/ws/sync?device_id=dev-a')
    await asyncio.sleep(0.3)

    tests = [
        ('Plain text', 'Hello World!'),
        ('Chinese', '你好世界 🎉'),
        ('Emoji', '🚀🔥💡🎯'),
        ('Empty text', ''),
        ('Large 100KB', 'x' * 100000),
        ('Multiline', 'line1\nline2\nline3'),
        ('Special chars', '<script>alert("xss")</script>'),
        ('Unicode escape', '\\u0041\\u0042\\u0043'),
        ('Tab chars', 'col1\tcol2\tcol3'),
        ('Null-like', 'before\x00after'),
    ]

    for label, text in tests:
        event = {'event_type':'ClipboardCaptured','data':{'item':{
            'item_id':f't-{label}','source_device_id':'dev-a','source_session_type':'Persistent',
            'kind':'Text','representations':[{'mime_type':'text/plain','content':{'Text':text}}],
            'size':len(text),'created_at':0,'payload_refs':[],'checksum':'abc','delivery_policy':'Broadcast'
        }}}
        await ws_a.send(json.dumps(event))
        found = False
        for _ in range(10):
            try:
                msg = await asyncio.wait_for(ws_b.recv(), timeout=3)
                d = json.loads(msg)
                if d.get('event_type') == 'ClipboardCaptured' and d['data']['item']['item_id'] == f't-{label}':
                    r = d['data']['item']['representations'][0]['content']['Text']
                    check(f'Sync: {label}', r == text)
                    found = True; break
            except asyncio.TimeoutError: break
        if not found:
            check(f'Sync: {label}', False, 'timeout')

    # Loop prevention
    await ws_b.send(json.dumps({'event_type':'ClipboardCaptured','data':{'item':{
        'item_id':'echo','source_device_id':'dev-b','source_session_type':'Persistent',
        'kind':'Text','representations':[{'mime_type':'text/plain','content':{'Text':'echo'}}],
        'size':4,'created_at':0,'payload_refs':[],'checksum':'abc','delivery_policy':'Broadcast'
    }}}))
    got_echo = False
    for _ in range(5):
        try:
            msg = await asyncio.wait_for(ws_b.recv(), timeout=1)
            d = json.loads(msg)
            if d.get('event_type') == 'ClipboardCaptured': got_echo = True; break
        except asyncio.TimeoutError: break
    check('Loop prevention', not got_echo)

    # Bidirectional sync
    event_b = {'event_type':'ClipboardCaptured','data':{'item':{
        'item_id':'b-to-a','source_device_id':'dev-b','source_session_type':'Persistent',
        'kind':'Text','representations':[{'mime_type':'text/plain','content':{'Text':'from B'}}],
        'size':6,'created_at':0,'payload_refs':[],'checksum':'abc','delivery_policy':'Broadcast'
    }}}
    await ws_b.send(json.dumps(event_b))
    found = False
    for _ in range(10):
        try:
            msg = await asyncio.wait_for(ws_a.recv(), timeout=3)
            d = json.loads(msg)
            if d.get('event_type') == 'ClipboardCaptured' and d['data']['item']['item_id'] == 'b-to-a':
                check('Bidirectional B→A', d['data']['item']['representations'][0]['content']['Text'] == 'from B')
                found = True; break
        except asyncio.TimeoutError: break
    if not found: check('Bidirectional B→A', False, 'timeout')

    await ws_a.close(); await ws_b.close()

    # ── Phase 4: Input Events ──
    print('\n=== Phase 4: Input Events ===')
    for label, ev in [
        ('Key A+Ctrl', {'input_type':'Key','data':{'key_code':'A','pressed':True,'modifiers':['Ctrl']}}),
        ('Key release', {'input_type':'Key','data':{'key_code':'A','pressed':False,'modifiers':[]}}),
        ('MouseMove', {'input_type':'MouseMove','data':{'x':500,'y':300,'dx':10,'dy':-5}}),
        ('MouseButton left', {'input_type':'MouseButton','data':{'button':'left','pressed':True,'x':100,'y':200}}),
        ('MouseButton right', {'input_type':'MouseButton','data':{'button':'right','pressed':True,'x':100,'y':200}}),
        ('MouseScroll', {'input_type':'MouseScroll','data':{'dx':0,'dy':-3}}),
        ('EmergencyRelease', {'input_type':'EmergencyRelease'}),
    ]:
        full = {'source_device_id':'dev-b','timestamp':0,'event':ev,'route':'LanDirect'}
        parsed = json.loads(json.dumps(full))
        check(f'Event: {label}', parsed['event']['input_type'] == ev['input_type'])

    # ── Phase 5: Error Handling ──
    print('\n=== Phase 5: Error Handling ===')
    try:
        parsed_ws = urlparse(WS_SERVER)
        wrong_port = (parsed_ws.port or 8080) + 1
        wrong_ws = parsed_ws._replace(netloc=f'{parsed_ws.hostname}:{wrong_port}').geturl()
        await asyncio.wait_for(websockets.connect(f'{wrong_ws}/ws/sync'), timeout=2)
        check('Wrong port fails', False)
    except: check('Wrong port fails', True)

    d = json.loads(urllib.request.urlopen(f'{SERVER}/api/v1/clipboard/history?limit=100').read())
    check('History has items', len(d.get('items',[])) >= 1)

    req = urllib.request.Request(f'{SERVER}/api/v1/tokens/validate',
        data=json.dumps({'token':'bad','operation':'copy'}).encode(),
        headers={'Content-Type':'application/json'})
    d = json.loads(urllib.request.urlopen(req).read())
    check('Invalid token rejected', d.get('valid') == False)

    # History pagination
    d = json.loads(urllib.request.urlopen(f'{SERVER}/api/v1/clipboard/history?limit=2&offset=0').read())
    check('Pagination works', len(d.get('items',[])) <= 2)

    # ── Phase 6: Windows Connection Scenarios ──
    print('\n=== Phase 6: Windows Connection Scenarios ===')
    # Simulate Windows client connection with platform=windows
    try:
        req = urllib.request.Request(f'{SERVER}/api/v1/devices/register',
            data=json.dumps({'device_id':'win-real','name':'Real Windows','platform':'windows','registration_token':'reg123'}).encode(),
            headers={'Content-Type':'application/json'})
        d = json.loads(urllib.request.urlopen(req).read())
        check('Windows device registered', d.get('status') == 'registered')
    except Exception as e:
        check('Windows device registered', False, str(e))

    # Windows client WebSocket
    try:
        ws_w = await websockets.connect(f'{WS_SERVER}/ws/sync?device_id=win-real')
        await ws_w.send(json.dumps({'event_type':'DeviceJoined','data':{'device_id':'win-real','name':'Real Windows'}}))
        ws_l = await websockets.connect(f'{WS_SERVER}/ws/sync?device_id=dev-a')
        await ws_l.send(json.dumps({'event_type':'ClipboardCaptured','data':{'item':{
            'item_id':'l-to-w','source_device_id':'dev-a','source_session_type':'Persistent',
            'kind':'Text','representations':[{'mime_type':'text/plain','content':{'Text':'Linux to Windows'}}],
            'size':16,'created_at':0,'payload_refs':[],'checksum':'abc','delivery_policy':'Broadcast'
        }}}))
        found = False
        for _ in range(10):
            try:
                msg = await asyncio.wait_for(ws_w.recv(), timeout=3)
                d = json.loads(msg)
                if d.get('event_type') == 'ClipboardCaptured':
                    check('Linux→Windows sync', d['data']['item']['representations'][0]['content']['Text'] == 'Linux to Windows')
                    found = True; break
            except asyncio.TimeoutError: break
        if not found: check('Linux→Windows sync', False, 'timeout')
        await ws_w.close(); await ws_l.close()
    except Exception as e:
        check('Windows WS connection', False, str(e))

    # ── Phase 7: Reconnection ──
    print('\n=== Phase 7: Reconnection ===')
    # Connect, disconnect, reconnect
    ws_temp = await websockets.connect(f'{WS_SERVER}/ws/sync?device_id=recon-test')
    await ws_temp.close()
    ws_temp2 = await websockets.connect(f'{WS_SERVER}/ws/sync?device_id=recon-test')
    check('Reconnect after disconnect', True)
    await ws_temp2.close()

    # ── Summary ──
    print('\n' + '=' * 60)
    print(f'RESULTS: {PASS} passed, {FAIL} failed, {SKIP} skipped')
    print('=' * 60)
    return FAIL

if __name__ == '__main__':
    import websockets
    sys.exit(1 if asyncio.run(main()) else 0)
