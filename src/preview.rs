use std::path::{Path, PathBuf};

use image::RgbaImage;
use minifb::{Key, MouseButton, MouseMode, Window, WindowOptions};
use thiserror::Error;

const ACTION_BAR_HEIGHT: usize = 48;
const BUTTONS: [PreviewButton; 7] = [
    PreviewButton {
        label: "Copy",
        action: Some(PreviewAction::Copy),
    },
    PreviewButton {
        label: "Path",
        action: Some(PreviewAction::CopyPath),
    },
    PreviewButton {
        label: "Edit",
        action: Some(PreviewAction::Edit),
    },
    PreviewButton {
        label: "Open",
        action: Some(PreviewAction::Open),
    },
    PreviewButton {
        label: "Reveal",
        action: Some(PreviewAction::Reveal),
    },
    PreviewButton {
        label: "Delete",
        action: Some(PreviewAction::Delete),
    },
    PreviewButton {
        label: "Close",
        action: None,
    },
];

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
    CopyPath,
    Edit,
    Open,
    Reveal,
    Delete,
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
        "shotlite preview: C copy, P path, E edit, O open, R reveal, Del delete, Esc close",
        view.width,
        view.height,
        WindowOptions::default(),
    )?;

    while window.is_open() {
        if let Some(action) = preview_action_from_keys(
            [
                Key::Escape,
                Key::C,
                Key::P,
                Key::E,
                Key::O,
                Key::R,
                Key::Delete,
            ]
            .into_iter()
            .filter(|key| window.is_key_down(*key)),
        ) {
            return Ok(action);
        }
        if window.get_mouse_down(MouseButton::Left)
            && let Some((x, y)) = window.get_mouse_pos(MouseMode::Discard)
            && let Some(action) = preview_action_from_click(&view, x, y)
        {
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
            Key::P => return Some(Some(PreviewAction::CopyPath)),
            Key::E => return Some(Some(PreviewAction::Edit)),
            Key::O => return Some(Some(PreviewAction::Open)),
            Key::R => return Some(Some(PreviewAction::Reveal)),
            Key::Delete => return Some(Some(PreviewAction::Delete)),
            _ => {}
        }
    }

    None
}

fn preview_action_from_click(view: &PreviewView, x: f32, y: f32) -> Option<Option<PreviewAction>> {
    if x < 0.0 || y < view.image_height as f32 {
        return None;
    }

    button_at(view, x as usize, y as usize).map(|button| button.action)
}

fn button_at(view: &PreviewView, x: usize, y: usize) -> Option<PreviewButton> {
    if y < view.image_height || y >= view.height || x >= view.width {
        return None;
    }

    let button_count = BUTTONS.len();
    let index = (x.saturating_mul(button_count) / view.width.max(1)).min(button_count - 1);
    Some(BUTTONS[index])
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PreviewButton {
    label: &'static str,
    action: Option<PreviewAction>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PreviewView {
    width: usize,
    height: usize,
    image_height: usize,
    scale_percent: u32,
}

impl PreviewView {
    fn new(image: &RgbaImage) -> Self {
        preview_view_size(image.width(), image.height(), 900, 700)
    }

    fn buffer_for(&self, image: &RgbaImage) -> Vec<u32> {
        let mut buffer = vec![0x202020; self.width * self.height];

        for y in 0..self.image_height {
            for x in 0..self.width {
                let source_x = ((x as u32 * 100) / self.scale_percent).min(image.width() - 1);
                let source_y = ((y as u32 * 100) / self.scale_percent).min(image.height() - 1);
                let pixel = image.get_pixel(source_x, source_y);
                buffer[y * self.width + x] =
                    u32::from(pixel[0]) << 16 | u32::from(pixel[1]) << 8 | u32::from(pixel[2]);
            }
        }

        self.draw_action_bar(&mut buffer);
        buffer
    }

    fn draw_action_bar(&self, buffer: &mut [u32]) {
        for y in self.image_height..self.height {
            for x in 0..self.width {
                let Some(button) = button_at(self, x, y) else {
                    continue;
                };
                let color = if button.action.is_some() {
                    0x303030
                } else {
                    0x282828
                };
                buffer[y * self.width + x] = color;
                if x > 0 && button_at(self, x - 1, y) != Some(button) {
                    buffer[y * self.width + x] = 0x505050;
                }
            }
        }

        for (index, button) in BUTTONS.iter().enumerate() {
            let left = self.width * index / BUTTONS.len();
            let right = self.width * (index + 1) / BUTTONS.len();
            let center_x = left + (right.saturating_sub(left) / 2);
            let center_y = self.image_height + ACTION_BAR_HEIGHT / 2;
            draw_text_centered(
                buffer,
                self.width,
                self.height,
                center_x,
                center_y,
                button.label,
                0xf0f0f0,
            );
        }
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
    let image_height = (image_height.max(1) * scale_percent / 100).max(1) as usize;

    PreviewView {
        width,
        height: image_height + ACTION_BAR_HEIGHT,
        image_height,
        scale_percent,
    }
}

fn draw_text_centered(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    center_x: usize,
    center_y: usize,
    text: &str,
    color: u32,
) {
    let scale = 2;
    let advance = 6 * scale;
    let text_width = text.chars().count() * advance;
    let x = center_x.saturating_sub(text_width / 2) as i32;
    let y = center_y.saturating_sub(7 * scale / 2) as i32;
    let mut target = DrawTarget {
        buffer,
        width,
        height,
    };

    for (index, ch) in text.chars().enumerate() {
        draw_char(
            &mut target,
            x + (index * advance) as i32,
            y,
            ch,
            color,
            scale as i32,
        );
    }
}

struct DrawTarget<'a> {
    buffer: &'a mut [u32],
    width: usize,
    height: usize,
}

fn draw_char(target: &mut DrawTarget<'_>, x: i32, y: i32, ch: char, color: u32, scale: i32) {
    for (row, pattern) in glyph_for(ch).iter().enumerate() {
        for column in 0..5 {
            if pattern & (1 << (4 - column)) == 0 {
                continue;
            }
            for dy in 0..scale {
                for dx in 0..scale {
                    let px = x + column * scale + dx;
                    let py = y + row as i32 * scale + dy;
                    if px >= 0
                        && py >= 0
                        && (px as usize) < target.width
                        && (py as usize) < target.height
                    {
                        target.buffer[py as usize * target.width + px as usize] = color;
                    }
                }
            }
        }
    }
}

fn glyph_for(ch: char) -> [u8; 7] {
    match ch.to_ascii_uppercase() {
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'C' => [
            0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111,
        ],
        'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'H' => [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'I' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ],
        'Y' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        _ => [0; 7],
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
                height: 348,
                image_height: 300,
                scale_percent: 100
            }
        );
    }

    #[test]
    fn preview_scales_large_images_down() {
        let view = preview_view_size(1920, 1080, 900, 700);

        assert_eq!(view.width, 883);
        assert_eq!(view.height, 544);
        assert_eq!(view.image_height, 496);
        assert_eq!(view.scale_percent, 46);
    }

    #[test]
    fn preview_buffer_preserves_pixel_color_and_draws_action_bar() {
        let image = RgbaImage::from_pixel(2, 1, Rgba([10, 20, 30, 255]));
        let view = PreviewView::new(&image);

        let buffer = view.buffer_for(&image);

        assert_eq!(buffer[0], 0x0a141e);
        assert_eq!(buffer[1], 0x0a141e);
        assert!(buffer[view.width * view.image_height..].contains(&0xf0f0f0));
    }

    #[test]
    fn preview_keys_map_to_actions() {
        assert_eq!(
            preview_action_from_keys([Key::C]),
            Some(Some(PreviewAction::Copy))
        );
        assert_eq!(
            preview_action_from_keys([Key::P]),
            Some(Some(PreviewAction::CopyPath))
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
        assert_eq!(
            preview_action_from_keys([Key::Delete]),
            Some(Some(PreviewAction::Delete))
        );
        assert_eq!(preview_action_from_keys([Key::Escape]), Some(None));
    }

    #[test]
    fn preview_escape_wins_when_multiple_keys_are_down() {
        assert_eq!(preview_action_from_keys([Key::Escape, Key::C]), Some(None));
    }

    #[test]
    fn preview_clicks_map_to_visible_buttons() {
        let view = preview_view_size(500, 200, 900, 700);
        let y = view.image_height as f32 + 10.0;

        assert_eq!(
            preview_action_from_click(&view, 10.0, y),
            Some(Some(PreviewAction::Copy))
        );
        assert_eq!(
            preview_action_from_click(&view, 80.0, y),
            Some(Some(PreviewAction::CopyPath))
        );
        assert_eq!(
            preview_action_from_click(&view, 150.0, y),
            Some(Some(PreviewAction::Edit))
        );
        assert_eq!(
            preview_action_from_click(&view, 220.0, y),
            Some(Some(PreviewAction::Open))
        );
        assert_eq!(
            preview_action_from_click(&view, 300.0, y),
            Some(Some(PreviewAction::Reveal))
        );
        assert_eq!(
            preview_action_from_click(&view, 370.0, y),
            Some(Some(PreviewAction::Delete))
        );
        assert_eq!(preview_action_from_click(&view, 450.0, y), Some(None));
    }

    #[test]
    fn preview_clicks_ignore_image_area_and_outside_window() {
        let view = preview_view_size(500, 200, 900, 700);

        assert_eq!(preview_action_from_click(&view, 10.0, 10.0), None);
        assert_eq!(
            preview_action_from_click(&view, 10.0, view.height as f32 + 1.0),
            None
        );
        assert_eq!(preview_action_from_click(&view, -1.0, 210.0), None);
    }
}
