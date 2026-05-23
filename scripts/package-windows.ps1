param(
    [string]$Configuration = "release",
    [string]$Version = "0.1.0",
    [string]$SignToolPath = $env:SIGNTOOL_PATH,
    [string]$CertificateSha1 = $env:CODESIGN_CERT_SHA1
)

$ErrorActionPreference = "Stop"

cargo build --$Configuration

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$distRoot = Join-Path $root "target\dist"
$packageName = "shotlite-v$Version-windows-x86_64"
$packageDir = Join-Path $distRoot $packageName
$zipPath = Join-Path $distRoot "$packageName.zip"
$exe = Join-Path $root "target\$Configuration\shotlite.exe"

Remove-Item -Recurse -Force $packageDir -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $packageDir | Out-Null

Copy-Item $exe $packageDir
Copy-Item (Join-Path $root "README.md") $packageDir
Copy-Item (Join-Path $root "LICENSE-MIT") $packageDir
Copy-Item (Join-Path $root "LICENSE-APACHE") $packageDir
Copy-Item (Join-Path $PSScriptRoot "install-windows.ps1") $packageDir
Copy-Item (Join-Path $PSScriptRoot "uninstall-windows.ps1") $packageDir

if ($SignToolPath -and $CertificateSha1) {
    & $SignToolPath sign /sha1 $CertificateSha1 /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 (Join-Path $packageDir "shotlite.exe")
}

Compress-Archive -Path (Join-Path $packageDir "*") -DestinationPath $zipPath -Force
Get-FileHash $zipPath -Algorithm SHA256
