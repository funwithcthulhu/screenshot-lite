# Windows QA

Manual checks before publishing a Windows build:

- `shotlite full --output target\qa\full.png`
- `shotlite region --output target\qa\region.png`
- `shotlite region --rect 0,0,100,100 --output target\qa\region-rect.png`
- `shotlite full --clipboard --output target\qa\clipboard.png`
- `shotlite edit target\qa\full.png`
- `shotlite tray`

Display cases to check:

- one monitor at 100% scale
- one monitor above 100% scale
- two monitors with different scale factors
- monitor positioned left of the primary display
- laptop display plus external monitor

Packaging:

- `scripts\package-windows.ps1`
- If signing is available, set `SIGNTOOL_PATH` and `CODESIGN_CERT_SHA1` before running the package script.
