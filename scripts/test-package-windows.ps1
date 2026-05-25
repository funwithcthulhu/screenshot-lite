param(
    [string]$PackageDir = (Join-Path (Resolve-Path (Join-Path $PSScriptRoot "..")) "target\dist\shotlite-v0.3.0-windows-x86_64")
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

Write-Output "Package contents OK: $PackageDir"
