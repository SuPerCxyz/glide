param(
    [string]$InstallDir = "$env:LOCALAPPDATA\Programs\Glide",
    [string]$Server = "http://aicode.soocoo.xyz:8080",
    [int]$StartupSeconds = 10
)

$ErrorActionPreference = "Stop"
$Pass = 0
$Fail = 0

function Check($Name, $Condition, $Detail = "") {
    if ($Condition) {
        Write-Host "PASS: $Name" -ForegroundColor Green
        $script:Pass++
    } else {
        Write-Host "FAIL: $Name" -ForegroundColor Red
        if ($Detail) { Write-Host "  $Detail" -ForegroundColor Red }
        $script:Fail++
    }
}

Write-Host "=== Glide installed Windows client test ==="
Write-Host "InstallDir: $InstallDir"
Write-Host "Server: $Server"

if (-not (Test-Path $InstallDir)) {
    $candidates = @(
        "$env:LOCALAPPDATA\Glide",
        "$env:LOCALAPPDATA\Programs\Glide",
        "$env:ProgramFiles\Glide",
        "${env:ProgramFiles(x86)}\Glide"
    )
    $InstallDir = ($candidates | Where-Object { Test-Path $_ } | Select-Object -First 1)
}

Check "Install directory exists" (Test-Path $InstallDir) $InstallDir
$mainExe = Join-Path $InstallDir "Glide.exe"
if (-not (Test-Path $mainExe)) {
    $mainExe = Join-Path $InstallDir "glide.exe"
}
Check "Main executable exists" (Test-Path $mainExe) $mainExe

$configDir = Join-Path $env:APPDATA "Glide"
$logDir = Join-Path $env:LOCALAPPDATA "Glide\logs"
New-Item -ItemType Directory -Force -Path $configDir | Out-Null
New-Item -ItemType Directory -Force -Path $logDir | Out-Null
Check "User config directory writable" (Test-Path $configDir)
Check "Log directory writable" (Test-Path $logDir)

try {
    $response = Invoke-WebRequest -UseBasicParsing -Uri $Server -TimeoutSec 10
    Check "Server reachable" ($response.StatusCode -ge 200 -and $response.StatusCode -lt 500)
} catch {
    Check "Server reachable" $false $_.Exception.Message
}

if (Test-Path $mainExe) {
    $proc = Start-Process -FilePath $mainExe -PassThru
    Start-Sleep -Seconds $StartupSeconds
    Check "GUI starts and stays running" (-not $proc.HasExited)

    Set-Clipboard -Value "Glide installed client smoke test"
    Start-Sleep -Milliseconds 500
    Check "Windows clipboard text roundtrip" ((Get-Clipboard) -eq "Glide installed client smoke test")

    Set-Clipboard -Value "Chinese Emoji multiline`r`n你好 🚀"
    Start-Sleep -Milliseconds 500
    $clip = Get-Clipboard
    Check "Unicode clipboard roundtrip" ($clip -match "你好")

    if (-not $proc.HasExited) {
        Stop-Process -Id $proc.Id -Force
    }
}

Write-Host ""
Write-Host "RESULTS: $Pass passed, $Fail failed"
if ($Fail -gt 0) { exit 1 }
exit 0
