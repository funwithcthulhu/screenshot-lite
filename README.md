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

Capture a rectangle:

```sh
shotlite region --rect 10,20,400,300
```

Copy a capture to the clipboard too:

```sh
shotlite full --clipboard
```

Redact an image by filling a rectangle with black pixels:

```sh
shotlite redact input.png --rect 10,20,200,80
```

By default, redaction writes `input-redacted.png`. Use `--output` to choose another path.

Show or set the output directory:

```sh
shotlite config show
shotlite config set output-dir C:\Users\you\Pictures\Screenshots
```

## Limitations

- Interactive region selection is not implemented yet. Use `region --rect x,y,w,h`.
- Clipboard support depends on the local platform clipboard.
- Capture support depends on `xcap` support for the current desktop/session.

## License

No license file is included yet.
