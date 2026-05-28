# Windows smoke test

This is a manual smoke checklist for a Windows build. It is not a substitute for `cargo test`.

Use a temporary output directory:

```text
mkdir target\qa
```

## Capture

Full-screen capture:

```text
shotlite full --output target\qa\full.png
```

Expected result:

- command exits successfully;
- `target\qa\full.png` exists;
- stdout prints the saved path.

Rectangle capture with explicit coordinates:

```text
shotlite region --rect 0,0,100,100 --output target\qa\region-rect.png
```

Expected result:

- command exits successfully when the rectangle fits inside a monitor;
- `target\qa\region-rect.png` exists;
- stdout prints the saved path.

Windows region overlay:

```text
shotlite region --output target\qa\region-overlay.png
```

Expected result:

- the selection overlay opens on Windows;
- selecting a region writes `target\qa\region-overlay.png`;
- canceling or platform failure is reported as an error.

## Clipboard

```text
shotlite full --clipboard --output target\qa\clipboard.png
```

Expected result:

- `target\qa\clipboard.png` exists;
- the clipboard contains an image if the platform clipboard accepts it.

## Preview and editor

Preview after capture:

```text
shotlite full --preview --output target\qa\preview.png
```

Expected result:

- `target\qa\preview.png` exists;
- a preview window opens;
- the preview window shows clickable copy, edit, open, reveal, and close actions;
- `C` copies the image;
- `E` opens the editor;
- `O` opens the file;
- `R` reveals the file;
- `Esc` closes the preview window.

Edit after capture:

```text
shotlite full --edit --output target\qa\edit-source.png
```

Expected result:

- `target\qa\edit-source.png` is created;
- the editor opens for that file;
- `S` writes `target\qa\edit-source-edited.png`;
- stdout prints the edited path after saving.

Existing file editor:

```text
shotlite edit target\qa\full.png --output target\qa\full-edited.png
```

Expected result:

- the editor opens;
- drag a rectangle and use `R`, `H`, `O`, `A`, `T`, or `1` through `9`;
- `S` writes `target\qa\full-edited.png`;
- the input file remains present.

## Tray

Start tray mode:

```text
shotlite tray
```

Expected result on Windows:

- a tray icon appears;
- `Ctrl+Shift+1` captures the full screen;
- `Ctrl+Shift+2` opens region selection;
- `Ctrl+Shift+Q` exits tray mode;
- right-click menu entries open or reveal recent screenshots when available.

Tray mode is currently Windows-only.

## Config commands

Print config path:

```text
shotlite config path
```

Print config directory:

```text
shotlite config dir
```

Show config:

```text
shotlite config show
```

Validate configured output directory:

```text
shotlite config validate
```

Reset config to defaults:

```text
shotlite config reset
```

Set output directory:

```text
shotlite config output-dir target\qa
shotlite config output-dir
shotlite config set output-dir target\qa
shotlite config show
```

Expected result:

- `config output-dir` prints the configured screenshot directory;
- `config show` prints `output_dir = "target\\qa"` or an equivalent path representation.

## Package scripts

Build a Windows package:

```text
powershell -ExecutionPolicy Bypass -File scripts\package-windows.ps1
```

Check package contents:

```text
powershell -ExecutionPolicy Bypass -File scripts\test-package-windows.ps1
```

Install from an unpacked package directory:

```text
powershell -ExecutionPolicy Bypass -File install-windows.ps1
```

Install and add the startup entry:

```text
powershell -ExecutionPolicy Bypass -File install-windows.ps1 -StartWithWindows
```

Uninstall:

```text
powershell -ExecutionPolicy Bypass -File uninstall-windows.ps1
```

The uninstall script removes the installed executable, Start Menu shortcuts, and the `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` entry named `shotlite`. It does not remove the config file or screenshot files.
