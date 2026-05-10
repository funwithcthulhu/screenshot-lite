use std::borrow::Cow;

use arboard::{Clipboard, ImageData};
use image::RgbaImage;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClipboardError {
    #[error("clipboard is not available: {0}")]
    Open(#[source] arboard::Error),
    #[error("failed to copy image to clipboard: {0}")]
    Copy(#[source] arboard::Error),
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
