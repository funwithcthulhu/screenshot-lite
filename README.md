# shotlite

`shotlite` is a small local screenshot utility.

It currently supports:

- full-screen capture
- rectangle capture with explicit coordinates or a Windows selection overlay
- PNG output to a configured directory
- copying captures to the clipboard when the platform clipboard is available
- destructive pixel redaction into a new PNG file

There are no cloud features, accounts, telemetry, or AI features.

## Build

```text
cargo build
```

## Usage

Capture all detected monitors:

```text
shotlite full
```

Save one capture to a specific directory:

```text
shotlite full --output-dir .\shots
```

Save one capture to an exact file:

```text
shotlite full --output .\shots\screen.png
```

Capture a rectangle:

```text
shotlite region
shotlite region --rect 10,20,400,300
```

Interactive region selection is currently Windows-only. Use `--rect x,y,w,h` where the overlay is not available.

Copy a capture to the clipboard too:

```text
shotlite full --clipboard
```

Show a small preview window after capture:

```text
shotlite full --preview
```

The preview window has clickable actions for copy, edit, open, reveal, and close. The same actions are available from the keyboard: `C`, `E`, `O`, `R`, and `Esc`.

Open the editor after capture:

```text
shotlite full --edit
shotlite region --edit
```

Open or reveal the saved file after capture:

```text
shotlite full --open
shotlite full --reveal
```

List recent PNG files in the configured output directory:

```text
shotlite history
shotlite history --limit 5
shotlite history --open 1
shotlite history --reveal 1
```

History indexes are one-based. If a listed file is moved or deleted before `--open` or `--reveal` runs, the command reports that path as missing.

Redact an image by filling a rectangle with black pixels:

```text
shotlite redact input.png --rect 10,20,200,80
```

By default, redaction writes `input-redacted.png` and leaves the input file unchanged.
Use `--output` to choose another path.

Highlight or crop an existing image:

```text
shotlite highlight input.png --rect 10,20,200,80
shotlite crop input.png --rect 10,20,200,80
```

Open a minimal editor window for an existing image:

```text
shotlite edit input.png
shotlite edit input.png --output edited.png
```

In the editor, drag a rectangle, then press `R` to redact, `H` to highlight, `O` to draw an outline, `A` to draw an arrow, `T` to type a text label, or `1` through `9` to add a numbered marker. Text labels are applied with `Enter`; `Backspace` edits the label and `Esc` cancels text entry. Press `U` to undo, `S` to save, or `C` to crop and save. `--output` chooses the file written by the edit operation.

Run the Windows tray app:

```text
shotlite tray
```

Right-click the tray icon for capture, copy/open/reveal last screenshot, folder, config, startup, and quit actions.

Tray hotkeys:

- `Ctrl+Shift+1`: full-screen capture
- `Ctrl+Shift+2`: region capture
- `Ctrl+Shift+Q`: quit tray mode

Show or set the output directory:

```text
shotlite config path
shotlite config dir
shotlite config open
shotlite config output-dir
shotlite config output-dir C:\Users\you\Pictures\Screenshots
shotlite config show
shotlite config validate
shotlite config reset
shotlite config set output-dir C:\Users\you\Pictures\Screenshots
```

`config output-dir` is the shorter form for reading or changing the screenshot directory. `config set output-dir` is kept for compatibility.

## Limitations

- Interactive region selection and tray mode are currently Windows-only.
- Clipboard support depends on the local platform clipboard.
- Capture support depends on `xcap` support for the current desktop/session.
- Linux and macOS support is currently CLI-first; tray and global hotkeys are not implemented there.
- CI builds and tests the Rust code on Windows, Linux, and macOS.

## Additional docs

- [Privacy and redaction notes](docs/privacy-and-redaction.md)
- [Windows smoke test](docs/windows-smoke-test.md)

## Packaging

Download the Windows zip from the GitHub release page, unzip it, then run the installer from the unpacked directory.

Build a Windows package:

```text
powershell -ExecutionPolicy Bypass -File scripts\package-windows.ps1
```

Install from the unpacked package:

```text
powershell -ExecutionPolicy Bypass -File install-windows.ps1
powershell -ExecutionPolicy Bypass -File install-windows.ps1 -StartWithWindows
```

Skip the Start Menu shortcut:

```text
powershell -ExecutionPolicy Bypass -File install-windows.ps1 -NoStartMenuShortcut
```

Uninstall:

```text
powershell -ExecutionPolicy Bypass -File uninstall-windows.ps1
```

The uninstall script removes the installed executable, Start Menu shortcut, and startup entry. It does not remove the config file or screenshots.

By default, screenshots are written to the configured output directory. The config file path can be printed with:

```text
shotlite config path
```

The config directory can be printed with:

```text
shotlite config dir
```

On Windows, the default config path is under `%APPDATA%\shotlite\config.toml`.

## License

Licensed under either MIT or Apache-2.0.
