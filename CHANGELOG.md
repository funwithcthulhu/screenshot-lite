# Changelog

## 0.4.0

- Added clickable preview actions for copy path and delete.
- Added `region --last` to reuse the last successful region rectangle.
- Added `monitor` capture and `monitor --index`.
- Added `config output-dir` as a shorter way to show or set the screenshot directory.
- Added a Windows self-extracting installer release asset.

## 0.3.0

- Added tray menu actions for copying and revealing the last screenshot.
- Added `history` to list recent PNG files from the configured output directory.
- Added `history --open` and `history --reveal` for recent screenshots.
- Added `--preview` for showing a saved capture in a small window.
- Added `--edit` for opening the editor after capture.
- Added `config dir`, `config validate`, and `config reset`.
- Added `--output` to the editor command.
- Added editor outline, arrow, numbered marker, undo, and explicit save actions.
- Added editor text labels.
- Added a Windows uninstall Start Menu shortcut.
- Expanded CI to build and test on Windows, Linux, and macOS.

## 0.2.0

- Added Windows tray mode with hotkeys, notifications, and a right-click menu.
- Added Windows interactive region selection.
- Added a minimal editor for redact, highlight, and crop actions.
- Added Windows package, install, uninstall, and optional startup support.
- Added regression coverage around redaction and post-capture behavior.

## 0.1.0

- Added full-screen capture.
- Added explicit rectangle capture.
- Added PNG output to a configured directory.
- Added optional clipboard copy.
- Added destructive pixel redaction into a new PNG file.
