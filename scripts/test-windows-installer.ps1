param(
    [Parameter(Mandatory = $true)]
    [string]$InstallerPath,
    [string]$InstallDir = "$env:LOCALAPPDATA\Programs\Glide",
    [switch]$Msi
)

$ErrorActionPreference = "Stop"
$installer = (Resolve-Path $InstallerPath).Path

Write-Host "=== Glide Windows installer test ==="
Write-Host "Installer: $installer"
Write-Host "InstallDir: $InstallDir"

if ($Msi -or $installer.ToLowerInvariant().EndsWith(".msi")) {
    $args = "/i `"$installer`" /qn /norestart"
    $proc = Start-Process msiexec.exe -ArgumentList $args -Wait -PassThru
    if ($proc.ExitCode -ne 0) {
        throw "MSI install failed with exit code $($proc.ExitCode)"
    }
} else {
    $proc = Start-Process -FilePath $installer -ArgumentList "/S" -Wait -PassThru
    if ($proc.ExitCode -ne 0) {
        throw "NSIS install failed with exit code $($proc.ExitCode)"
    }
}

if (-not (Test-Path $InstallDir)) {
    $candidates = @(
        "$env:LOCALAPPDATA\Glide",
        "$env:LOCALAPPDATA\Programs\Glide",
        "$env:ProgramFiles\Glide",
        "${env:ProgramFiles(x86)}\Glide"
    )
    $found = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
    if ($found) { $InstallDir = $found } else { throw "Install directory not found" }
}

pwsh -NoProfile -ExecutionPolicy Bypass -File "$PSScriptRoot\check-windows-package-deps.ps1" `
    -InstallDir $InstallDir `
    -MainExe "Glide.exe" `
    -LaunchSmoke

Write-Host "Installer smoke test passed."
