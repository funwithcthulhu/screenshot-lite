param(
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA "shotlite")
)

$ErrorActionPreference = "Stop"

$source = Join-Path $PSScriptRoot "shotlite.exe"
if (!(Test-Path $source)) {
    throw "shotlite.exe not found next to install-windows.ps1"
}

New-Item -ItemType Directory -Force $InstallDir | Out-Null
Copy-Item $source (Join-Path $InstallDir "shotlite.exe") -Force

$startMenu = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs"
$shortcutPath = Join-Path $startMenu "shotlite tray.lnk"
$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut($shortcutPath)
$shortcut.TargetPath = Join-Path $InstallDir "shotlite.exe"
$shortcut.Arguments = "tray"
$shortcut.WorkingDirectory = $InstallDir
$shortcut.Save()

Write-Output "Installed shotlite to $InstallDir"
Write-Output "Start Menu shortcut: $shortcutPath"
