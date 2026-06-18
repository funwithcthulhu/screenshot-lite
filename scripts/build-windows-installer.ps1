param(
    [string]$Version = "0.4.0",
    [string]$PackageDir = (Join-Path (Resolve-Path (Join-Path $PSScriptRoot "..")) "target\dist\shotlite-v$Version-windows-x86_64")
)

$ErrorActionPreference = "Stop"

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$distRoot = Join-Path $root "target\dist"
$installerPath = Join-Path $distRoot "shotlite-v$Version-windows-x86_64-installer.exe"
$sedPath = Join-Path $distRoot "shotlite-v$Version-windows-x86_64-installer.sed"

if (!(Test-Path $PackageDir)) {
    throw "package directory not found: $PackageDir"
}

$required = @(
    "shotlite.exe",
    "install-windows.ps1",
    "uninstall-windows.ps1",
    "README.md",
    "CHANGELOG.md",
    "LICENSE-MIT",
    "LICENSE-APACHE"
)

foreach ($name in $required) {
    if (!(Test-Path (Join-Path $PackageDir $name))) {
        throw "missing package file: $name"
    }
}

$sourceDir = (Resolve-Path $PackageDir).Path
$sed = @"
[Version]
Class=IEXPRESS
SEDVersion=3

[Options]
PackagePurpose=InstallApp
ShowInstallProgramWindow=0
HideExtractAnimation=1
UseLongFileName=1
InsideCompressed=0
CAB_FixedSize=0
CAB_ResvCodeSigning=0
RebootMode=N
InstallPrompt=
DisplayLicense=
FinishMessage=shotlite has been installed.
TargetName=$installerPath
FriendlyName=shotlite $Version installer
AppLaunched=powershell.exe -ExecutionPolicy Bypass -File install-windows.ps1
PostInstallCmd=<None>
AdminQuietInstCmd=powershell.exe -ExecutionPolicy Bypass -File install-windows.ps1
UserQuietInstCmd=powershell.exe -ExecutionPolicy Bypass -File install-windows.ps1
SourceFiles=SourceFiles

[SourceFiles]
SourceFiles0=$sourceDir

[SourceFiles0]
%FILE0%=shotlite.exe
%FILE1%=install-windows.ps1
%FILE2%=uninstall-windows.ps1
%FILE3%=README.md
%FILE4%=CHANGELOG.md
%FILE5%=LICENSE-MIT
%FILE6%=LICENSE-APACHE

[Strings]
FILE0=shotlite.exe
FILE1=install-windows.ps1
FILE2=uninstall-windows.ps1
FILE3=README.md
FILE4=CHANGELOG.md
FILE5=LICENSE-MIT
FILE6=LICENSE-APACHE
"@

New-Item -ItemType Directory -Force $distRoot | Out-Null
Set-Content -Path $sedPath -Value $sed -Encoding ASCII
Remove-Item -Force $installerPath -ErrorAction SilentlyContinue

$process = Start-Process -FilePath "iexpress.exe" -ArgumentList @("/N", "/Q", $sedPath) -Wait -PassThru -WindowStyle Hidden

if (!(Test-Path $installerPath)) {
    throw "iexpress failed with exit code $($process.ExitCode) and did not create installer: $installerPath"
}

Get-FileHash $installerPath -Algorithm SHA256
