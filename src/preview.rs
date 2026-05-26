use std::path::{Path, PathBuf};

use image::RgbaImage;
use minifb::{Key, Window, WindowOptions};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PreviewError {
    #[error("failed to open {path}: {source}")]
    Open {
        path: PathBuf,
        source: image::ImageError,
    },
    #[error("window error: {0}")]
    Window(#[from] minifb::Error),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreviewAction {
    Copy,
    Edit,
    Open,
    Reveal,
}

pub fn show_file(path: &Path) -> Result<Option<PreviewAction>, PreviewError> {
    let image = image::open(path)
        .map_err(|source| PreviewError::Open {
            path: path.to_path_buf(),
            source,
        })?
        .to_rgba8();
    show_image(&image)
}

fn show_image(image: &RgbaImage) -> Result<Option<PreviewAction>, PreviewError> {
    let view = PreviewView::new(image);
    let buffer = view.buffer_for(image);
    let mut window = Window::new(
        "shotlite preview: C copy, E edit, O open, R reveal, Esc close",
        view.width,
        view.height,
        WindowOptions::default(),
    )?;

    while window.is_open() {
        if let Some(action) = preview_action_from_keys(
            [Key::Escape, Key::C, Key::E, Key::O, Key::R]
                .into_iter()
                .filter(|key| window.is_key_down(*key)),
        ) {
            return Ok(action);
        }
        window.update_with_buffer(&buffer, view.width, view.height)?;
    }

    Ok(None)
}

fn preview_action_from_keys(keys: impl IntoIterator<Item = Key>) -> Option<Option<PreviewAction>> {
    for key in keys {
        match key {
            Key::Escape => return Some(None),
            Key::C => return Some(Some(PreviewAction::Copy)),
            Key::E => return Some(Some(PreviewAction::Edit)),
            Key::O => return Some(Some(PreviewAction::Open)),
            Key::R => return Some(Some(PreviewAction::Reveal)),
            _ => {}
        }
    }

    None
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PreviewView {
    width: usize,
    height: usize,
    scale_percent: u32,
}

impl PreviewView {
    fn new(image: &RgbaImage) -> Self {
        preview_view_size(image.width(), image.height(), 900, 700)
    }

    fn buffer_for(&self, image: &RgbaImage) -> Vec<u32> {
        let mut buffer = vec![0; self.width * self.height];

        for y in 0..self.height {
            for x in 0..self.width {
                let source_x = ((x as u32 * 100) / self.scale_percent).min(image.width() - 1);
                let source_y = ((y as u32 * 100) / self.scale_percent).min(image.height() - 1);
                let pixel = image.get_pixel(source_x, source_y);
                buffer[y * self.width + x] =
                    u32::from(pixel[0]) << 16 | u32::from(pixel[1]) << 8 | u32::from(pixel[2]);
            }
        }

        buffer
    }
}

fn preview_view_size(
    image_width: u32,
    image_height: u32,
    max_width: u32,
    max_height: u32,
) -> PreviewView {
    let width_scale = max_width.saturating_mul(100) / image_width.max(1);
    let height_scale = max_height.saturating_mul(100) / image_height.max(1);
    let scale_percent = width_scale.min(height_scale).clamp(1, 100);
    let width = (image_width.max(1) * scale_percent / 100).max(1) as usize;
    let height = (image_height.max(1) * scale_percent / 100).max(1) as usize;

    PreviewView {
        width,
        height,
        scale_percent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    #[test]
    fn preview_keeps_small_images_at_original_size() {
        assert_eq!(
            preview_view_size(400, 300, 900, 700),
            PreviewView {
                width: 400,
                height: 300,
                scale_percent: 100
            }
        );
    }

    #[test]
    fn preview_scales_large_images_down() {
        let view = preview_view_size(1920, 1080, 900, 700);

        assert_eq!(view.width, 883);
        assert_eq!(view.height, 496);
        assert_eq!(view.scale_percent, 46);
    }

    #[test]
    fn preview_buffer_preserves_pixel_color() {
        let image = RgbaImage::from_pixel(2, 1, Rgba([10, 20, 30, 255]));
        let view = PreviewView::new(&image);

        assert_eq!(view.buffer_for(&image), [0x0a141e, 0x0a141e]);
    }

    #[test]
    fn preview_keys_map_to_actions() {
        assert_eq!(
            preview_action_from_keys([Key::C]),
            Some(Some(PreviewAction::Copy))
        );
        assert_eq!(
            preview_action_from_keys([Key::E]),
            Some(Some(PreviewAction::Edit))
        );
        assert_eq!(
            preview_action_from_keys([Key::O]),
            Some(Some(PreviewAction::Open))
        );
        assert_eq!(
            preview_action_from_keys([Key::R]),
            Some(Some(PreviewAction::Reveal))
        );
        assert_eq!(preview_action_from_keys([Key::Escape]), Some(None));
    }

    #[test]
    fn preview_escape_wins_when_multiple_keys_are_down() {
        assert_eq!(preview_action_from_keys([Key::Escape, Key::C]), Some(None));
    }
}
