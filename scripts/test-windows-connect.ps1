# test-windows-connect.ps1 — Windows client connection test script.
# Run in PowerShell on a Windows machine with the Glide client installed.
# Usage: .\scripts\test-windows-connect.ps1 -Server http://aicode.soocoo.xyz:8080

param(
    [string]$Server = "http://aicode.soocoo.xyz:8080",
    [string]$Token = "reg123"
)

$PASS = 0
$FAIL = 0

function Check($Name, $Condition) {
    if ($Condition) {
        Write-Host "  ✅ $Name" -ForegroundColor Green
        $script:PASS++
    } else {
        Write-Host "  ❌ $Name" -ForegroundColor Red
        $script:FAIL++
    }
}

Write-Host "=== Glide Windows Connection Tests ==="
Write-Host "Server: $Server"
Write-Host ""

# Phase 1: Network Connectivity
Write-Host "--- Network Connectivity ---"
$health = Test-NetConnection -ComputerName ([Uri]$Server).Host -Port ([Uri]$Server).Port -WarningAction SilentlyContinue
Check "Port reachable" $health.TcpTestSucceeded

$dns = Resolve-DnsName ([Uri]$Server).Host -ErrorAction SilentlyContinue
Check "DNS resolves" ($null -ne $dns)

# Phase 2: Server Health
Write-Host ""
Write-Host "--- Server Health ---"
try {
    $response = Invoke-RestMethod -Uri "$Server/api/v1/health"
    Check "Health OK" ($response.status -eq "ok")
    Check "Version present" ($null -ne $response.version)
    Write-Host "    Version: $($response.version)"
} catch {
    Check "Health OK" $false
}

# Phase 3: Device Registration
Write-Host ""
Write-Host "--- Device Registration ---"
$deviceId = [guid]::NewGuid().ToString()
$body = @{
    device_id = $deviceId
    name = $env:COMPUTERNAME
    platform = "windows"
    trusted = $true
    registration_token = $Token
} | ConvertTo-Json

try {
    $reg = Invoke-RestMethod -Uri "$Server/api/v1/devices/register" -Method POST -Body $body -ContentType "application/json"
    Check "Device registered" ($reg.status -eq "registered")
    Write-Host "    Device ID: $deviceId"
} catch {
    Check "Device registered" $false
    Write-Host "    Error: $_"
}

# Bad token
$badBody = @{
    device_id = "bad-device"
    name = "bad"
    registration_token = "wrong-token"
} | ConvertTo-Json

try {
    Invoke-RestMethod -Uri "$Server/api/v1/devices/register" -Method POST -Body $badBody -ContentType "application/json"
    Check "Bad token rejected" $false
} catch {
    Check "Bad token rejected" $true
}

# Phase 4: Clipboard Sync (WebSocket)
Write-Host ""
Write-Host "--- WebSocket Connection ---"
# PowerShell WebSocket test
try {
    $ws = [System.Net.WebSockets.ClientWebSocket]::new()
    $wsUri = $Server.Replace("http://", "ws://").Replace("https://", "wss://") + "/ws/sync?device_id=$deviceId"
    $ct = [System.Threading.CancellationToken]::None
    $ws.ConnectAsync([Uri]$wsUri, $ct).Wait(5000)
    Check "WebSocket connected" ($ws.State -eq [System.Net.WebSockets.WebSocketState]::Open)

    # Send identification
    $msg = @{
        event_type = "DeviceJoined"
        data = @{ device_id = $deviceId; name = $env:COMPUTERNAME }
    } | ConvertTo-Json
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($msg)
    $seg = [System.ArraySegment[byte]]::new($bytes)
    $ws.SendAsync($seg, [System.Net.WebSockets.WebSocketMessageType]::Text, $true, $ct).Wait(2000)
    Check "Sent identification" $true

    $ws.CloseAsync([System.Net.WebSockets.WebSocketCloseStatus]::NormalClosure, "test done", $ct).Wait(2000)
    Check "Clean disconnect" ($ws.State -eq [System.Net.WebSockets.WebSocketState]::Closed)
} catch {
    Check "WebSocket connection" $false
    Write-Host "    Error: $_"
}

# Phase 5: Clipboard History
Write-Host ""
Write-Host "--- History ---"
try {
    $history = Invoke-RestMethod -Uri "$Server/api/v1/clipboard/history?limit=5"
    Check "History endpoint works" ($null -ne $history.items)
    Write-Host "    Items: $($history.items.Count)"
} catch {
    Check "History endpoint works" $false
}

# Summary
Write-Host ""
Write-Host "=" * 50
Write-Host "RESULTS: $script:PASS passed, $script:FAIL failed"
Write-Host "=" * 50

if ($script:FAIL -gt 0) { exit 1 } else { exit 0 }
