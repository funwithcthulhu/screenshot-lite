# shotlite

`shotlite` is a small local screenshot utility.

It currently supports:

- full-screen capture
- rectangle capture with explicit coordinates
- PNG output to a configured directory
- copying captures to the clipboard when the platform clipboard is available
- destructive pixel redaction into a new PNG file

There are no cloud features, accounts, telemetry, or AI features.

## Build

```sh
cargo build
```

## Usage

Capture all detected monitors:

```sh
shotlite full
```

Save one capture to a specific directory:

```sh
shotlite full --output-dir .\shots
```

Save one capture to an exact file:

```sh
shotlite full --output .\shots\screen.png
```

Capture a rectangle:

```sh
shotlite region --rect 10,20,400,300
```

Region capture is coordinate-only for now; interactive selection is intentionally not implemented yet.

Copy a capture to the clipboard too:

```sh
shotlite full --clipboard
```

Open or reveal the saved file after capture:

```sh
shotlite full --open
shotlite full --reveal
```

Redact an image by filling a rectangle with black pixels:

```sh
shotlite redact input.png --rect 10,20,200,80
```

By default, redaction writes `input-redacted.png` and leaves the input file unchanged.
Use `--output` to choose another path.

Highlight or crop an existing image:

```sh
shotlite highlight input.png --rect 10,20,200,80
shotlite crop input.png --rect 10,20,200,80
```

Show or set the output directory:

```sh
shotlite config path
shotlite config show
shotlite config set output-dir C:\Users\you\Pictures\Screenshots
```

## Limitations

- Interactive region selection is not implemented yet. Use `region --rect x,y,w,h`.
- Clipboard support depends on the local platform clipboard.
- Capture support depends on `xcap` support for the current desktop/session.

## License

Licensed under either MIT or Apache-2.0, at your option.
