use std::path::{Path, PathBuf};

use image::RgbaImage;
use minifb::{Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};
use thiserror::Error;

use crate::redact::{self, Rect};

#[derive(Debug, Error)]
pub enum EditorError {
    #[error("failed to open {path}: {source}")]
    Open {
        path: PathBuf,
        source: image::ImageError,
    },
    #[error("editor was closed without saving")]
    Canceled,
    #[error("window error: {0}")]
    Window(#[from] minifb::Error),
    #[error(transparent)]
    Redact(#[from] redact::RedactError),
}

pub fn edit_file(path: &Path) -> Result<PathBuf, EditorError> {
    let image = load_image(path)?;
    let view = ImageView::new(&image);
    let mut window = Window::new(
        "shotlite edit: drag a rectangle, then R redact, H highlight, C crop, Esc close",
        view.width,
        view.height,
        WindowOptions::default(),
    )?;
    let mut drag_start = None;
    let mut selection = None;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        if window.get_mouse_down(MouseButton::Left) {
            if drag_start.is_none() {
                drag_start = mouse_point(&window);
            }
            if let (Some(start), Some(current)) = (drag_start, mouse_point(&window)) {
                selection =
                    Rect::from_points(start, current).and_then(|rect| view.to_image_rect(rect));
            }
        } else {
            drag_start = None;
        }

        if let Some(rect) = selection {
            if window.is_key_pressed(Key::R, KeyRepeat::No) {
                return apply_operation(path, rect, EditorOperation::Redact);
            }
            if window.is_key_pressed(Key::H, KeyRepeat::No) {
                return apply_operation(path, rect, EditorOperation::Highlight);
            }
            if window.is_key_pressed(Key::C, KeyRepeat::No) {
                return apply_operation(path, rect, EditorOperation::Crop);
            }
        }

        let mut buffer = view.buffer.clone();
        if let Some(rect) = selection {
            view.draw_rect(&mut buffer, rect);
        }
        window.update_with_buffer(&buffer, view.width, view.height)?;
    }

    Err(EditorError::Canceled)
}

fn load_image(path: &Path) -> Result<RgbaImage, EditorError> {
    image::open(path)
        .map_err(|source| EditorError::Open {
            path: path.to_path_buf(),
            source,
        })
        .map(|image| image.to_rgba8())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EditorOperation {
    Redact,
    Highlight,
    Crop,
}

fn apply_operation(
    path: &Path,
    rect: Rect,
    operation: EditorOperation,
) -> Result<PathBuf, EditorError> {
    match operation {
        EditorOperation::Redact => redact::redact_file(path, rect, None),
        EditorOperation::Highlight => redact::highlight_file(path, rect, None),
        EditorOperation::Crop => redact::crop_file(path, rect, None),
    }
    .map_err(EditorError::from)
}

#[derive(Clone)]
struct ImageView {
    width: usize,
    height: usize,
    scale: f32,
    buffer: Vec<u32>,
}

impl ImageView {
    fn new(image: &RgbaImage) -> Self {
        let max_width = 1200.0;
        let max_height = 800.0;
        let scale = (max_width / image.width() as f32)
            .min(max_height / image.height() as f32)
            .min(1.0);
        let width = (image.width() as f32 * scale).round().max(1.0) as usize;
        let height = (image.height() as f32 * scale).round().max(1.0) as usize;
        let mut buffer = vec![0; width * height];

        for y in 0..height {
            for x in 0..width {
                let source_x = ((x as f32 / scale).floor() as u32).min(image.width() - 1);
                let source_y = ((y as f32 / scale).floor() as u32).min(image.height() - 1);
                let pixel = image.get_pixel(source_x, source_y);
                buffer[y * width + x] =
                    u32::from(pixel[0]) << 16 | u32::from(pixel[1]) << 8 | u32::from(pixel[2]);
            }
        }

        Self {
            width,
            height,
            scale,
            buffer,
        }
    }

    fn to_image_rect(&self, rect: Rect) -> Option<Rect> {
        Some(Rect {
            x: (rect.x as f32 / self.scale).floor() as i32,
            y: (rect.y as f32 / self.scale).floor() as i32,
            width: (rect.width as f32 / self.scale).ceil() as u32,
            height: (rect.height as f32 / self.scale).ceil() as u32,
        })
    }

    fn draw_rect(&self, buffer: &mut [u32], rect: Rect) {
        let rect = Rect {
            x: (rect.x as f32 * self.scale).round() as i32,
            y: (rect.y as f32 * self.scale).round() as i32,
            width: (rect.width as f32 * self.scale).round().max(1.0) as u32,
            height: (rect.height as f32 * self.scale).round().max(1.0) as u32,
        };
        let left = rect.x.max(0) as usize;
        let top = rect.y.max(0) as usize;
        let right = (rect.x + rect.width as i32).clamp(0, self.width as i32) as usize;
        let bottom = (rect.y + rect.height as i32).clamp(0, self.height as i32) as usize;

        if left >= right || top >= bottom {
            return;
        }

        let color = 0xffd000;
        for x in left..right {
            buffer[top * self.width + x] = color;
            buffer[(bottom - 1) * self.width + x] = color;
        }
        for y in top..bottom {
            buffer[y * self.width + left] = color;
            buffer[y * self.width + right - 1] = color;
        }
    }
}

fn mouse_point(window: &Window) -> Option<(i32, i32)> {
    window
        .get_mouse_pos(MouseMode::Clamp)
        .map(|(x, y)| (x.round() as i32, y.round() as i32))
}

trait RectFromPoints {
    fn from_points(start: (i32, i32), end: (i32, i32)) -> Option<Rect>;
}

impl RectFromPoints for Rect {
    fn from_points(start: (i32, i32), end: (i32, i32)) -> Option<Rect> {
        let left = start.0.min(end.0);
        let top = start.1.min(end.1);
        let width = start.0.abs_diff(end.0);
        let height = start.1.abs_diff(end.1);
        if width == 0 || height == 0 {
            return None;
        }
        Some(Rect {
            x: left,
            y: top,
            width,
            height,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn load_image_reports_missing_input_path() {
        let path = temp_path("missing.png");

        let error = load_image(&path).unwrap_err().to_string();

        assert!(error.contains("failed to open"));
        assert!(error.contains("missing.png"));
    }

    #[test]
    fn load_image_reports_unsupported_input_path() {
        let path = temp_path("not-image.txt");
        fs::write(&path, "not an image").unwrap();

        let error = load_image(&path).unwrap_err().to_string();

        assert!(error.contains("failed to open"));
        assert!(error.contains("not-image.txt"));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn loading_image_does_not_modify_input() {
        let path = temp_path("input.png");
        write_test_image(&path);
        let before = fs::read(&path).unwrap();

        let image = load_image(&path).unwrap();

        assert_eq!(image.dimensions(), (4, 3));
        assert_eq!(fs::read(&path).unwrap(), before);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn editor_operations_write_copies_and_preserve_input() {
        for operation in [
            EditorOperation::Redact,
            EditorOperation::Highlight,
            EditorOperation::Crop,
        ] {
            let path = temp_path(&format!("{operation:?}.png"));
            write_test_image(&path);
            let before = fs::read(&path).unwrap();

            let output = apply_operation(
                &path,
                Rect {
                    x: 1,
                    y: 1,
                    width: 2,
                    height: 1,
                },
                operation,
            )
            .unwrap();

            assert!(output.exists());
            assert_eq!(fs::read(&path).unwrap(), before);
            fs::remove_file(path).unwrap();
            fs::remove_file(output).unwrap();
        }
    }

    fn write_test_image(path: &Path) {
        let mut image = RgbaImage::from_pixel(4, 3, Rgba([255, 255, 255, 255]));
        image.put_pixel(0, 0, Rgba([10, 20, 30, 255]));
        image.save(path).unwrap();
    }

    fn temp_path(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("shotlite-editor-{unique}-{name}"))
    }
}
