use std::{borrow::Cow, path::Path};

use arboard::{Clipboard, ImageData};
use image::RgbaImage;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClipboardError {
    #[error("clipboard is not available: {0}")]
    Open(#[source] arboard::Error),
    #[error("failed to copy image to clipboard: {0}")]
    Copy(#[source] arboard::Error),
    #[error("failed to open image for clipboard: {0}")]
    Image(#[from] image::ImageError),
}

pub fn copy_text(text: &str) -> Result<(), ClipboardError> {
    let mut clipboard = Clipboard::new().map_err(ClipboardError::Open)?;
    clipboard
        .set_text(text.to_owned())
        .map_err(ClipboardError::Copy)
}

pub fn copy_image_file(path: &Path) -> Result<(), ClipboardError> {
    let image = image::open(path)?.to_rgba8();
    copy_image(&image)
}

pub fn copy_image(image: &RgbaImage) -> Result<(), ClipboardError> {
    let mut clipboard = Clipboard::new().map_err(ClipboardError::Open)?;
    let data = ImageData {
        width: image.width() as usize,
        height: image.height() as usize,
        bytes: Cow::Borrowed(image.as_raw()),
    };

    clipboard.set_image(data).map_err(ClipboardError::Copy)
}
