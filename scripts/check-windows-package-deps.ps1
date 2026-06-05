param(
    [string]$InstallDir = "",
    [string]$MainExe = "glide.exe",
    [string]$BundleRoot = "",
    [switch]$RequireWebView2OfflineInstaller,
    [switch]$LaunchSmoke
)

$ErrorActionPreference = "Stop"
$Pass = 0
$Fail = 0
$Warn = 0

function Pass($Name) {
    Write-Host "PASS: $Name" -ForegroundColor Green
    $script:Pass++
}

function Fail($Name, $Detail = "") {
    Write-Host "FAIL: $Name" -ForegroundColor Red
    if ($Detail) { Write-Host "  $Detail" -ForegroundColor Red }
    $script:Fail++
}

function Warn($Name, $Detail = "") {
    Write-Host "WARN: $Name" -ForegroundColor Yellow
    if ($Detail) { Write-Host "  $Detail" -ForegroundColor Yellow }
    $script:Warn++
}

function Test-RequiredFile($Path, $Name) {
    if (Test-Path -LiteralPath $Path) {
        Pass $Name
    } else {
        Fail $Name "Missing: $Path"
    }
}

function Get-RepoRoot {
    $scriptDir = Split-Path -Parent $PSCommandPath
    return (Resolve-Path (Join-Path $scriptDir "..")).Path
}

function Find-Dumpbin {
    $vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
    if (Test-Path $vswhere) {
        $installPath = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
        if ($installPath) {
            $candidate = Get-ChildItem -Path (Join-Path $installPath "VC\Tools\MSVC") -Recurse -Filter dumpbin.exe -ErrorAction SilentlyContinue |
                Where-Object { $_.FullName -match "\\Hostx64\\x64\\dumpbin.exe$" } |
                Select-Object -First 1
            if ($candidate) { return $candidate.FullName }
        }
    }
    $cmd = Get-Command dumpbin.exe -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }
    return $null
}

function Get-AsciiStrings($Path) {
    $bytes = [System.IO.File]::ReadAllBytes($Path)
    $builder = New-Object System.Text.StringBuilder
    $strings = New-Object System.Collections.Generic.List[string]
    foreach ($byte in $bytes) {
        if ($byte -ge 32 -and $byte -le 126) {
            [void]$builder.Append([char]$byte)
        } else {
            if ($builder.Length -ge 8) { $strings.Add($builder.ToString()) }
            [void]$builder.Clear()
        }
    }
    if ($builder.Length -ge 8) { $strings.Add($builder.ToString()) }
    return $strings
}

Write-Host "=== Glide Windows package dependency check ==="
$repoRoot = Get-RepoRoot

if (-not $InstallDir) {
    $InstallDir = Join-Path $repoRoot "target\release"
}
$InstallDir = (Resolve-Path $InstallDir).Path
$mainExePath = Join-Path $InstallDir $MainExe

Test-RequiredFile $mainExePath "Main GUI executable exists"
Test-RequiredFile (Join-Path $InstallDir "glide-server.exe") "Server executable exists"
Test-RequiredFile (Join-Path $InstallDir "glide-cli.exe") "CLI executable exists"

$readme = Join-Path $InstallDir "README.md"
if (Test-Path $readme) {
    Pass "Portable package includes README"
} else {
    Warn "Portable README is not present" "Installed NSIS/MSI directories do not have to include README.md."
}

$tauriConfigPath = Join-Path $repoRoot "crates\glide-tauri\tauri.conf.json"
if (Test-Path $tauriConfigPath) {
    $tauriConfig = Get-Content $tauriConfigPath -Raw | ConvertFrom-Json
    $mode = $tauriConfig.bundle.windows.webviewInstallMode.type
    if ($RequireWebView2OfflineInstaller) {
        if ($mode -eq "offlineInstaller") {
            Pass "Tauri config embeds WebView2 offline installer"
        } else {
            Fail "Tauri config embeds WebView2 offline installer" "Actual webviewInstallMode: $mode"
        }
    }
    $targets = @($tauriConfig.bundle.targets)
    if ($targets -contains "nsis") { Pass "Tauri targets include NSIS" } else { Fail "Tauri targets include NSIS" }
    if ($targets -contains "msi") { Pass "Tauri targets include MSI" } else { Fail "Tauri targets include MSI" }
} else {
    Fail "Tauri config exists" "Missing: $tauriConfigPath"
}

if ($BundleRoot) {
    if (Test-Path $BundleRoot) {
        $nsis = Get-ChildItem -Path $BundleRoot -Recurse -File -Include "*setup*.exe", "*.exe" |
            Where-Object { $_.FullName -match "\\nsis\\" -or $_.Name -match "setup" } |
            Select-Object -First 1
        $msi = Get-ChildItem -Path $BundleRoot -Recurse -File -Filter "*.msi" | Select-Object -First 1
        if ($nsis) { Pass "NSIS installer artifact exists" } else { Fail "NSIS installer artifact exists" }
        if ($msi) { Pass "MSI installer artifact exists" } else { Fail "MSI installer artifact exists" }
    } else {
        Fail "Bundle output root exists" "Missing: $BundleRoot"
    }
}

$dumpbin = Find-Dumpbin
if ($dumpbin -and (Test-Path $mainExePath)) {
    $deps = & $dumpbin /dependents $mainExePath 2>$null
    $depText = ($deps -join "`n")
    if ($depText -match "VCRUNTIME|MSVCP") {
        Fail "MSVC runtime is statically linked" "dumpbin still reports VC runtime dependency."
    } else {
        Pass "MSVC runtime is statically linked or not required"
    }
    if ($depText -match "WebView2Loader\.dll") {
        Warn "WebView2Loader.dll dependency found" "Ensure the DLL is installed next to the app or provided by Tauri."
    } else {
        Pass "No direct WebView2Loader.dll import in main executable"
    }
} else {
    Warn "dumpbin dependency scan skipped" "Install Visual Studio Build Tools or run from a Developer PowerShell."
}

if (Test-Path $mainExePath) {
    $strings = Get-AsciiStrings $mainExePath
    $badPatterns = @(
        "\\home\\",
        "/home/",
        "\\target\\debug",
        "\\target\\release\\build",
        "\\crates\\glide",
        "C:\\Users\\runneradmin\\",
        "C:\\Users\\superc\\"
    )
    $badMatches = @()
    foreach ($pattern in $badPatterns) {
        $badMatches += $strings | Where-Object { $_ -like "*$pattern*" } | Select-Object -First 3
    }
    if ($badMatches.Count -eq 0) {
        Pass "Executable does not expose build-machine paths"
    } else {
        Fail "Executable does not expose build-machine paths" ($badMatches -join "`n")
    }
}

if ($LaunchSmoke -and (Test-Path $mainExePath)) {
    $proc = Start-Process -FilePath $mainExePath -PassThru -WindowStyle Minimized
    Start-Sleep -Seconds 10
    if ($proc.HasExited) {
        Fail "Main executable stays running after launch" "ExitCode=$($proc.ExitCode)"
    } else {
        Pass "Main executable starts and stays running"
        Stop-Process -Id $proc.Id -Force
    }
}

Write-Host ""
Write-Host "RESULTS: $Pass passed, $Warn warnings, $Fail failed"
if ($Fail -gt 0) { exit 1 }
exit 0
