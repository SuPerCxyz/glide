# test-windows-gui-smoke.ps1 - Run Glide GUI startup diagnostics on Windows.
# Usage:
#   powershell -ExecutionPolicy Bypass -File .\scripts\test-windows-gui-smoke.ps1 -GuiExe .\glide.exe

param(
    [string]$GuiExe = ".\glide.exe",
    [string]$LogPath = "$env:TEMP\glide-gui-smoke.log"
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $GuiExe)) {
    Write-Host "Missing GUI executable: $GuiExe" -ForegroundColor Red
    exit 1
}

if (Test-Path $LogPath) {
    Remove-Item $LogPath -Force
}

$env:GLIDE_GUI_LOG = $LogPath

Write-Host "=== Glide Windows GUI Smoke ==="
Write-Host "GUI: $GuiExe"
Write-Host "Log: $LogPath"

& $GuiExe --smoke
if ($LASTEXITCODE -ne 0) {
    Write-Host "glide-gui --smoke failed: $LASTEXITCODE" -ForegroundColor Red
    if (Test-Path $LogPath) {
        Get-Content $LogPath
    }
    exit $LASTEXITCODE
}

if (-not (Test-Path $LogPath)) {
    Write-Host "Diagnostics log was not created." -ForegroundColor Red
    exit 1
}

& $GuiExe --diagnostics
if ($LASTEXITCODE -ne 0) {
    Write-Host "glide-gui --diagnostics failed: $LASTEXITCODE" -ForegroundColor Red
    exit $LASTEXITCODE
}

Write-Host "--- diagnostics ---"
Get-Content $LogPath
Write-Host "GUI smoke passed." -ForegroundColor Green
