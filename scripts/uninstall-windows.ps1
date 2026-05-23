param(
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA "shotlite")
)

$ErrorActionPreference = "Stop"

$startMenu = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs"
$shortcutPath = Join-Path $startMenu "shotlite tray.lnk"
Remove-Item $shortcutPath -Force -ErrorAction SilentlyContinue

$runKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
Remove-ItemProperty -Path $runKey -Name "shotlite" -ErrorAction SilentlyContinue

Remove-Item -Recurse -Force $InstallDir -ErrorAction SilentlyContinue

Write-Output "Removed shotlite from $InstallDir"
Write-Output "Removed Start Menu shortcut: $shortcutPath"
Write-Output "Removed Start with Windows entry"
