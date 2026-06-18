param(
    [string]$PackageDir = (Join-Path (Resolve-Path (Join-Path $PSScriptRoot "..")) "target\dist\shotlite-v0.4.0-windows-x86_64")
)

$ErrorActionPreference = "Stop"

$required = @(
    "shotlite.exe",
    "README.md",
    "CHANGELOG.md",
    "LICENSE-MIT",
    "LICENSE-APACHE",
    "install-windows.ps1",
    "uninstall-windows.ps1"
)

foreach ($name in $required) {
    $path = Join-Path $PackageDir $name
    if (!(Test-Path $path)) {
        throw "missing package file: $name"
    }
}

$packageName = Split-Path $PackageDir -Leaf
$installer = Join-Path (Split-Path $PackageDir -Parent) "$packageName-installer.exe"
if (Test-Path $installer) {
    Write-Output "Installer OK: $installer"
}

Write-Output "Package contents OK: $PackageDir"
