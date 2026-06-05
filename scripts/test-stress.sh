#!/bin/bash
# scripts/test-stress.sh — Multi-client concurrent sync stress test
set +e
PASS=0; FAIL=0
SERVER="http://localhost:8080"
check() { if [ "$2" = "0" ]; then echo "  ✅ $1"; PASS=$((PASS+1)); else echo "  ❌ $1"; FAIL=$((FAIL+1)); fi }

echo "=== Stress Tests ==="

# Test 1: 10 concurrent clients all sending simultaneously
echo ""
echo "--- 10 Concurrent Clients ---"
python3 -c "
import asyncio, json, websockets, time

SERVER = 'ws://localhost:8080/ws/sync'
N = 10
MESSAGES_PER_CLIENT = 5

async def sender(client_id, count):
    ws = await websockets.connect(f'{SERVER}?device_id=stress-{client_id}')
    await ws.send(json.dumps({'event_type':'DeviceJoined','data':{'device_id':f'stress-{client_id}','name':f'Stress{client_id}'}}))
    await asyncio.sleep(0.2)

    sent = []
    for i in range(count):
        text = f'msg-{client_id}-{i}'
        event = {'event_type':'ClipboardCaptured','data':{'item':{
            'item_id':f'stress-{client_id}-{i}','source_device_id':f'stress-{client_id}',
            'source_session_type':'Persistent','kind':'Text',
            'representations':[{'mime_type':'text/plain','content':{'Text':text}}],
            'size':len(text),'created_at':0,'payload_refs':[],'checksum':'abc','delivery_policy':'Broadcast'
        }}}
        await ws.send(json.dumps(event))
        sent.append(text)
        await asyncio.sleep(0.05)

    await ws.close()
    return sent

async def receiver():
    ws = await websockets.connect(f'{SERVER}?device_id=stress-receiver')
    await ws.send(json.dumps({'event_type':'DeviceJoined','data':{'device_id':'stress-receiver','name':'Receiver'}}))
    await asyncio.sleep(0.3)

    received = []
    deadline = asyncio.get_event_loop().time() + 30
    while asyncio.get_event_loop().time() < deadline:
        try:
            msg = await asyncio.wait_for(ws.recv(), timeout=2)
            d = json.loads(msg)
            if d.get('event_type') == 'ClipboardCaptured':
                text = d['data']['item']['representations'][0]['content']['Text']
                received.append(text)
        except asyncio.TimeoutError:
            break

    await ws.close()
    return received

async def main():
    # Start receiver first
    recv_task = asyncio.create_task(receiver())
    await asyncio.sleep(1)

    # Start all senders
    start = time.time()
    send_tasks = [asyncio.create_task(sender(i, MESSAGES_PER_CLIENT)) for i in range(N)]
    all_sent = await asyncio.gather(*send_tasks)
    elapsed = time.time() - start

    # Collect received
    received = await recv_task

    total_sent = sum(len(s) for s in all_sent)
    print(f'  Sent: {total_sent} messages from {N} clients in {elapsed:.1f}s')
    print(f'  Received: {len(received)} messages')

    if len(received) >= total_sent * 0.8:
        print(f'  ✅ Stress test: {N} clients x {MESSAGES_PER_CLIENT} msgs')
    else:
        print(f'  ❌ Stress test: only {len(received)}/{total_sent} received')

asyncio.run(main())
" 2>&1

# Test 2: Rapid connect/disconnect
echo ""
echo "--- Rapid Connect/Disconnect ---"
python3 -c "
import asyncio, websockets, json

async def test():
    count = 0
    for i in range(20):
        try:
            ws = await asyncio.wait_for(
                websockets.connect(f'ws://localhost:8080/ws/sync?device_id=rapid-{i}'),
                timeout=3
            )
            await ws.send(json.dumps({'event_type':'DeviceJoined','data':{'device_id':f'rapid-{i}','name':f'Rapid{i}'}}))
            await ws.close()
            count += 1
        except:
            pass
    if count >= 18:
        print(f'  ✅ Rapid connect/disconnect: {count}/20')
    else:
        print(f'  ❌ Rapid connect/disconnect: {count}/20')

asyncio.run(test())
" 2>&1

# Test 3: Large payload
echo ""
echo "--- Large Payload ---"
python3 -c "
import asyncio, websockets, json

async def test():
    ws = await websockets.connect('ws://localhost:8080/ws/sync?device_id=large-test')
    await ws.send(json.dumps({'event_type':'DeviceJoined','data':{'device_id':'large-test','name':'LargeTest'}}))

    text = 'A' * 1000000
    event = {'event_type':'ClipboardCaptured','data':{'item':{
        'item_id':'large-1mb','source_device_id':'large-test','source_session_type':'Persistent',
        'kind':'Text','representations':[{'mime_type':'text/plain','content':{'Text':text}}],
        'size':len(text),'created_at':0,'payload_refs':[],'checksum':'abc','delivery_policy':'Broadcast'
    }}}
    await ws.send(json.dumps(event))
    print('  ✅ 1MB payload sent')
    await ws.close()

asyncio.run(test())
" 2>&1

# Test 4: Server restart recovery
echo ""
echo "--- Server Restart Recovery ---"
docker restart glide-server > /dev/null 2>&1
sleep 3
curl -sf "$SERVER/api/v1/health" > /dev/null 2>&1
check "Server recovered after restart" $?

python3 -c "
import asyncio, websockets, json

async def test():
    ws = await websockets.connect('ws://localhost:8080/ws/sync?device_id=restart-test')
    await ws.send(json.dumps({'event_type':'DeviceJoined','data':{'device_id':'restart-test','name':'RestartTest'}}))
    print('  ✅ Client reconnects after restart')
    await ws.close()

asyncio.run(test())
" 2>&1

echo ""
echo "========================================"
echo "Stress tests: $PASS passed, $FAIL failed"
echo "========================================"
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
