# Privacy and redaction notes

This document describes the current image-file behavior in `shotlite`.

## Local behavior

`shotlite` reads and writes local files. The repository does not contain cloud upload, account, telemetry, or network-sharing code. Clipboard operations, file opening, and file revealing use local platform APIs or commands.

## Rectangle handling

Rectangle arguments use `x,y,w,h`.

- `w` and `h` must be greater than zero.
- Negative `x` or `y` values are rejected for image operations.
- A rectangle must fit inside the image bounds. Rectangles are rejected rather than clipped.

These cases are covered in `src/redact.rs` tests and CLI integration tests.

## Redaction

`shotlite redact <file> --rect x,y,w,h` opens the input image, converts it to an RGBA image, and fills every pixel inside the rectangle with solid black:

```text
Rgba([0, 0, 0, 255])
```

The redaction is a pixel write in the output image. It is not an overlay layer.

By default, redaction writes a sibling file named:

```text
<input-stem>-redacted.png
```

For example:

```text
input.png -> input-redacted.png
```

The default path leaves the input file unchanged. Passing `--output` writes to the exact path provided. If that path already exists, it is replaced. If `--output` points at the input file, the command can replace the input file.

## Highlight

`shotlite highlight <file> --rect x,y,w,h` opens the input image, converts it to RGBA, and writes a highlighted copy. For each pixel in the rectangle, the RGB channels are averaged with yellow:

```text
Rgba([255, 230, 0, 255])
```

The original alpha channel is preserved.

By default, highlight writes:

```text
<input-stem>-highlighted.png
```

`--output` follows the same overwrite behavior described above.

## Crop

`shotlite crop <file> --rect x,y,w,h` opens the input image, converts it to RGBA, validates the rectangle, and writes a cropped PNG containing only that rectangle.

By default, crop writes:

```text
<input-stem>-cropped.png
```

`--output` follows the same overwrite behavior described above.

## Unsupported input files

Image opening is handled by the `image` crate with the formats enabled in this project. Non-image input is reported as an open error and no default output file is written.

## What this does not protect against

Pixel redaction only affects the output image file written by `shotlite`.

It does not remove or change:

- the original input file, unless `--output` explicitly points at it;
- existing copies, backups, thumbnails, or synced versions made by other software;
- screenshots already copied to the clipboard before redaction;
- filesystem-level recoverability of overwritten files;
- metadata in files that are not rewritten by these commands.

Use the redacted output file as the shareable file. Do not share the original file if it contains information that should not be visible.
