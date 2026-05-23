param(
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA "shotlite"),
    [switch]$StartWithWindows,
    [switch]$NoStartMenuShortcut
)

$ErrorActionPreference = "Stop"

$source = Join-Path $PSScriptRoot "shotlite.exe"
if (!(Test-Path $source)) {
    throw "shotlite.exe not found next to install-windows.ps1"
}

New-Item -ItemType Directory -Force $InstallDir | Out-Null
Copy-Item $source (Join-Path $InstallDir "shotlite.exe") -Force

if (!$NoStartMenuShortcut) {
    $startMenu = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs"
    $shortcutPath = Join-Path $startMenu "shotlite tray.lnk"
    $shell = New-Object -ComObject WScript.Shell
    $shortcut = $shell.CreateShortcut($shortcutPath)
    $shortcut.TargetPath = Join-Path $InstallDir "shotlite.exe"
    $shortcut.Arguments = "tray"
    $shortcut.WorkingDirectory = $InstallDir
    $shortcut.Save()
    Write-Output "Start Menu shortcut: $shortcutPath"
}

if ($StartWithWindows) {
    $runKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
    $command = "`"$(Join-Path $InstallDir "shotlite.exe")`" tray"
    New-Item -Path $runKey -Force | Out-Null
    New-ItemProperty -Path $runKey -Name "shotlite" -Value $command -PropertyType String -Force | Out-Null
    Write-Output "Start with Windows: enabled"
}

Write-Output "Installed shotlite to $InstallDir"
