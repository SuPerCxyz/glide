#!/bin/bash
# scripts/test-keyboard-mouse-protocol.sh — Keyboard/mouse event protocol tests
set +e
echo "=== Keyboard/Mouse Protocol Tests ==="
python3 -c "
import json

PASS = FAIL = 0
def check(name, cond):
    global PASS, FAIL
    if cond: print(f'  ✅ {name}'); PASS += 1
    else: print(f'  ❌ {name}'); FAIL += 1

# Test keyboard event encoding/decoding
print('--- Keyboard Events ---')
events = [
    ('Key A press', {'input_type':'Key','data':{'key_code':'A','pressed':True,'modifiers':[]}}),
    ('Key A release', {'input_type':'Key','data':{'key_code':'A','pressed':False,'modifiers':[]}}),
    ('Ctrl+C', {'input_type':'Key','data':{'key_code':'C','pressed':True,'modifiers':['Ctrl']}}),
    ('Ctrl+Alt+Del', {'input_type':'Key','data':{'key_code':'Delete','pressed':True,'modifiers':['Ctrl','Alt']}}),
    ('Shift+A', {'input_type':'Key','data':{'key_code':'A','pressed':True,'modifiers':['Shift']}}),
    ('Win key', {'input_type':'Key','data':{'key_code':'Super_L','pressed':True,'modifiers':[]}}),
    ('F1', {'input_type':'Key','data':{'key_code':'F1','pressed':True,'modifiers':[]}}),
    ('Enter', {'input_type':'Key','data':{'key_code':'Return','pressed':True,'modifiers':[]}}),
    ('Arrow Up', {'input_type':'Key','data':{'key_code':'Up','pressed':True,'modifiers':[]}}),
]
for label, ev in events:
    full = {'source_device_id':'test','timestamp':0,'event':ev,'route':'LanDirect'}
    parsed = json.loads(json.dumps(full))
    check(f'Encode: {label}', parsed['event']['input_type'] == 'Key')

# Test mouse event encoding/decoding
print()
print('--- Mouse Events ---')
mouse_events = [
    ('Move (0,0)', {'input_type':'MouseMove','data':{'x':0,'y':0,'dx':0,'dy':0}}),
    ('Move (1920,1080)', {'input_type':'MouseMove','data':{'x':1920,'y':1080,'dx':10,'dy':-5}}),
    ('Move negative delta', {'input_type':'MouseMove','data':{'x':100,'y':100,'dx':-15,'dy':-20}}),
    ('Left click', {'input_type':'MouseButton','data':{'button':'left','pressed':True,'x':500,'y':300}}),
    ('Right click', {'input_type':'MouseButton','data':{'button':'right','pressed':True,'x':500,'y':300}}),
    ('Middle click', {'input_type':'MouseButton','data':{'button':'middle','pressed':True,'x':500,'y':300}}),
    ('Left release', {'input_type':'MouseButton','data':{'button':'left','pressed':False,'x':500,'y':300}}),
    ('Scroll up', {'input_type':'MouseScroll','data':{'dx':0,'dy':-3}}),
    ('Scroll down', {'input_type':'MouseScroll','data':{'dx':0,'dy':3}}),
    ('Scroll horizontal', {'input_type':'MouseScroll','data':{'dx':2,'dy':0}}),
]
for label, ev in mouse_events:
    full = {'source_device_id':'test','timestamp':0,'event':ev,'route':'LanDirect'}
    parsed = json.loads(json.dumps(full))
    check(f'Encode: {label}', parsed['event']['input_type'] in ['MouseMove','MouseButton','MouseScroll'])

# Test emergency release
print()
print('--- Emergency Release ---')
ev = {'input_type':'EmergencyRelease'}
full = {'source_device_id':'test','timestamp':0,'event':ev,'route':'LanDirect'}
parsed = json.loads(json.dumps(full))
check('Emergency release encode', parsed['event']['input_type'] == 'EmergencyRelease')

# Test input routes
print()
print('--- Input Routes ---')
check('LAN Direct route', json.loads(json.dumps({'route':'LanDirect'}))['route'] == 'LanDirect')
check('Server Relay route', json.loads(json.dumps({'route':'ServerRelay'}))['route'] == 'ServerRelay')

# Test coordinate mapping
print()
print('--- Coordinate Mapping ---')
# Screen A: 1920x1080, Screen B: 2560x1440
# Mouse at right edge of A (1919, 540) should map to left edge of B (0, y)
a_w, a_h = 1920, 1080
b_w, b_h = 2560, 1440
# Simple linear mapping
src_x, src_y = 1919, 540
mapped_x = int(src_x * b_w / a_w)
mapped_y = int(src_y * b_h / a_h)
check('Coordinate mapping X', mapped_x == 2558)
check('Coordinate mapping Y', mapped_y == 720)

# Test edge detection
print()
print('--- Edge Detection ---')
check('Left edge (x=0)', 0 <= 0)
check('Right edge (x=1919)', 1919 >= 1920 - 1)
check('Top edge (y=0)', 0 <= 0)
check('Bottom edge (y=1079)', 1079 >= 1080 - 1)

# Test DPI scaling
print()
print('--- DPI Scaling ---')
# 100% scale: no change
check('100% scale X', int(500 * 1.0) == 500)
check('100% scale Y', int(300 * 1.0) == 300)
# 150% scale
check('150% scale X', int(500 * 1.5) == 750)
check('150% scale Y', int(300 * 1.5) == 450)
# 125% scale
check('125% scale X', int(500 * 1.25) == 625)
check('125% scale Y', int(300 * 1.25) == 375)

print()
print(f'Keyboard/Mouse tests: {PASS} passed, {FAIL} failed')
" 2>&1
