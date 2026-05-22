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

```
cargo build
```

## Usage

Capture all detected monitors:

```
shotlite full
```

Save one capture to a specific directory:

```
shotlite full --output-dir .\shots
```

Save one capture to an exact file:

```
shotlite full --output .\shots\screen.png
```

Capture a rectangle:

```
shotlite region
shotlite region --rect 10,20,400,300
```

Interactive region selection is currently Windows-only. Use `--rect x,y,w,h` where the overlay is not available.

Copy a capture to the clipboard too:

```
shotlite full --clipboard
```

Open or reveal the saved file after capture:

```
shotlite full --open
shotlite full --reveal
```

Redact an image by filling a rectangle with black pixels:

```
shotlite redact input.png --rect 10,20,200,80
```

By default, redaction writes `input-redacted.png` and leaves the input file unchanged.
Use `--output` to choose another path.

Highlight or crop an existing image:

```
shotlite highlight input.png --rect 10,20,200,80
shotlite crop input.png --rect 10,20,200,80
```

Open a minimal editor window for an existing image:

```
shotlite edit input.png
```

In the editor, drag a rectangle, then press `R` to redact, `H` to highlight, or `C` to crop.

Run the Windows tray app:

```
shotlite tray
```

Tray hotkeys:

- `Ctrl+Shift+1`: full-screen capture
- `Ctrl+Shift+2`: region capture
- `Ctrl+Shift+Q`: quit tray mode

Show or set the output directory:

```
shotlite config path
shotlite config show
shotlite config set output-dir C:\Users\you\Pictures\Screenshots
```

## Limitations

- Interactive region selection and tray mode are currently Windows-only.
- Clipboard support depends on the local platform clipboard.
- Capture support depends on `xcap` support for the current desktop/session.

## Packaging

Windows packaging scripts are in `scripts/`.

## License

Licensed under either MIT or Apache-2.0.
