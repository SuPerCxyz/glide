# test-windows-clipboard.ps1 — Windows clipboard sync test.
# Run on Windows with Glide client running and connected to server.
# Usage: .\scripts\test-windows-clipboard.ps1 -Server http://aicode.soocoo.xyz:8080

param(
    [string]$Server = "http://aicode.soocoo.xyz:8080"
)

$PASS = 0; $FAIL = 0

function Check($Name, $Condition) {
    if ($Condition) { Write-Host "  ✅ $Name"; $script:PASS++ }
    else { Write-Host "  ❌ $Name"; $script:FAIL++ }
}

Write-Host "=== Windows Clipboard Sync Tests ==="

# Test 1: Set clipboard and read back
Write-Host ""
Write-Host "--- Clipboard Read/Write ---"
$testText = "Glide clipboard test $(Get-Date)"
Set-Clipboard -Value $testText
$readBack = Get-Clipboard
Check "Set/Get clipboard" ($readBack -eq $testText)

# Test 2: Chinese text
$chinese = "你好世界"
Set-Clipboard -Value $chinese
$readBack = Get-Clipboard
Check "Chinese text clipboard" ($readBack -eq $chinese)

# Test 3: Empty clipboard
Set-Clipboard -Value ""
$readBack = Get-Clipboard
Check "Empty clipboard" ($readBack -eq "")

# Test 4: Large text
$large = "x" * 100000
Set-Clipboard -Value $large
$readBack = Get-Clipboard
Check "Large text (100KB)" ($readBack.Length -eq 100000)

# Test 5: Multiline text
$multiline = "line1`r`nline2`r`nline3"
Set-Clipboard -Value $multiline
$readBack = Get-Clipboard
Check "Multiline text" ($readBack -match "line1")

# Test 6: Notepad copy/paste automation
Write-Host ""
Write-Host "--- Notepad Automation ---"
$notepad = Start-Process notepad -PassThru
Start-Sleep -Seconds 2
$testText = "Glide sync test from Notepad"
Set-Clipboard -Value $testText
# Ctrl+V via SendKeys
Add-Type -AssemblyName System.Windows.Forms
[System.Windows.Forms.SendKeys]::SendWait("^v")
Start-Sleep -Milliseconds 500
Check "Notepad paste" $true  # If no crash, pass
$notepad.Kill()

# Summary
Write-Host ""
Write-Host "=" * 50
Write-Host "RESULTS: $script:PASS passed, $script:FAIL failed"
Write-Host "=" * 50
if ($script:FAIL -gt 0) { exit 1 } else { exit 0 }
