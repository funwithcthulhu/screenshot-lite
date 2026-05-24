use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use image::{Rgba, RgbaImage, imageops};
use minifb::{InputCallback, Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};
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
    #[error("failed to save {path}: {source}")]
    Save {
        path: PathBuf,
        source: image::ImageError,
    },
    #[error(transparent)]
    Redact(#[from] redact::RedactError),
}

pub fn edit_file(path: &Path, output: Option<PathBuf>) -> Result<PathBuf, EditorError> {
    let mut image = load_image(path)?;
    let view = ImageView::new(&image);
    let mut window = Window::new(
        "shotlite edit: drag, R redact, H highlight, O outline, A arrow, T text, 1-9 marker, U undo, S save, C crop",
        view.width,
        view.height,
        WindowOptions::default(),
    )?;
    let typed_text = Arc::new(Mutex::new(String::new()));
    window.set_input_callback(Box::new(TextInput::new(typed_text.clone())));
    let mut drag_start = None;
    let mut selection = None;
    let mut history = Vec::new();
    let mut text_draft = None;

    while window.is_open() {
        if text_draft.is_some() && window.is_key_pressed(Key::Escape, KeyRepeat::No) {
            text_draft = None;
            selection = None;
            take_typed_text(&typed_text);
        } else if text_draft.is_none() && window.is_key_down(Key::Escape) {
            break;
        }

        if text_draft.is_none() && window.get_mouse_down(MouseButton::Left) {
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
            if text_draft.is_none() && window.is_key_pressed(Key::R, KeyRepeat::No) {
                push_history(&mut history, &image);
                redact::apply_redaction(&mut image, rect, Rgba([0, 0, 0, 255]))?;
                selection = None;
            }
            if text_draft.is_none() && window.is_key_pressed(Key::H, KeyRepeat::No) {
                push_history(&mut history, &image);
                redact::apply_highlight(&mut image, rect)?;
                selection = None;
            }
            if text_draft.is_none() && window.is_key_pressed(Key::O, KeyRepeat::No) {
                push_history(&mut history, &image);
                draw_outline(&mut image, rect, Rgba([255, 210, 0, 255]))?;
                selection = None;
            }
            if text_draft.is_none() && window.is_key_pressed(Key::A, KeyRepeat::No) {
                push_history(&mut history, &image);
                draw_arrow(&mut image, rect, Rgba([255, 210, 0, 255]))?;
                selection = None;
            }
            if text_draft.is_none() && window.is_key_pressed(Key::T, KeyRepeat::No) {
                take_typed_text(&typed_text);
                text_draft = Some(TextDraft::new(rect));
            }
            if text_draft.is_none() && window.is_key_pressed(Key::C, KeyRepeat::No) {
                return crop_current_image(path, &image, rect, output.clone());
            }
            if text_draft.is_none()
                && let Some(marker) = pressed_marker_key(&window)
            {
                push_history(&mut history, &image);
                draw_marker(&mut image, rect, marker)?;
                selection = None;
            }
        }
        if let Some(draft) = text_draft.as_mut() {
            draft.append(&take_typed_text(&typed_text));
            if window.is_key_pressed(Key::Backspace, KeyRepeat::Yes) {
                draft.pop();
            }
            if window.is_key_pressed(Key::Enter, KeyRepeat::No) && !draft.text.is_empty() {
                push_history(&mut history, &image);
                draw_text_label(&mut image, draft.rect, &draft.text)?;
                selection = None;
                text_draft = None;
            }
        }
        if text_draft.is_none()
            && window.is_key_pressed(Key::U, KeyRepeat::No)
            && let Some(previous) = history.pop()
        {
            image = previous;
        }
        if text_draft.is_none() && window.is_key_pressed(Key::S, KeyRepeat::No) {
            return save_current_image(path, &image, output.clone());
        }

        let preview = if let Some(draft) = &text_draft {
            let mut preview = image.clone();
            if !draft.text.is_empty() {
                draw_text_label(&mut preview, draft.rect, &draft.text)?;
            }
            Some(preview)
        } else {
            None
        };
        let mut buffer = view.buffer_for(preview.as_ref().unwrap_or(&image));
        if let Some(rect) = selection {
            view.draw_rect(&mut buffer, rect);
        }
        window.update_with_buffer(&buffer, view.width, view.height)?;
    }

    Err(EditorError::Canceled)
}

struct TextInput {
    text: Arc<Mutex<String>>,
}

impl TextInput {
    fn new(text: Arc<Mutex<String>>) -> Self {
        Self { text }
    }
}

impl InputCallback for TextInput {
    fn add_char(&mut self, uni_char: u32) {
        let Some(ch) = char::from_u32(uni_char) else {
            return;
        };
        if !is_label_char(ch) {
            return;
        }

        let mut text = self.text.lock().unwrap();
        if text.len() < 128 {
            text.push(ch);
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TextDraft {
    rect: Rect,
    text: String,
}

impl TextDraft {
    fn new(rect: Rect) -> Self {
        Self {
            rect,
            text: String::new(),
        }
    }

    fn append(&mut self, text: &str) {
        self.text.push_str(text);
    }

    fn pop(&mut self) {
        self.text.pop();
    }
}

fn take_typed_text(text: &Arc<Mutex<String>>) -> String {
    std::mem::take(&mut *text.lock().unwrap())
}

fn is_label_char(ch: char) -> bool {
    ch.is_ascii_graphic() || ch == ' '
}

fn load_image(path: &Path) -> Result<RgbaImage, EditorError> {
    image::open(path)
        .map_err(|source| EditorError::Open {
            path: path.to_path_buf(),
            source,
        })
        .map(|image| image.to_rgba8())
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EditorOperation {
    Redact,
    Highlight,
    Crop,
}

#[cfg(test)]
fn apply_operation(
    path: &Path,
    rect: Rect,
    output: Option<PathBuf>,
    operation: EditorOperation,
) -> Result<PathBuf, EditorError> {
    match operation {
        EditorOperation::Redact => redact::redact_file(path, rect, output),
        EditorOperation::Highlight => redact::highlight_file(path, rect, output),
        EditorOperation::Crop => redact::crop_file(path, rect, output),
    }
    .map_err(EditorError::from)
}

fn push_history(history: &mut Vec<RgbaImage>, image: &RgbaImage) {
    history.push(image.clone());
    if history.len() > 20 {
        history.remove(0);
    }
}

fn save_current_image(
    input: &Path,
    image: &RgbaImage,
    output: Option<PathBuf>,
) -> Result<PathBuf, EditorError> {
    let output = output.unwrap_or_else(|| editor_output_path(input, "edited"));
    image.save(&output).map_err(|source| EditorError::Save {
        path: output.clone(),
        source,
    })?;
    Ok(output)
}

fn crop_current_image(
    input: &Path,
    image: &RgbaImage,
    rect: Rect,
    output: Option<PathBuf>,
) -> Result<PathBuf, EditorError> {
    let rect = checked_rect(rect, image.width(), image.height())?;
    let output = output.unwrap_or_else(|| editor_output_path(input, "cropped"));
    let cropped = imageops::crop_imm(image, rect.x, rect.y, rect.width, rect.height).to_image();
    cropped.save(&output).map_err(|source| EditorError::Save {
        path: output.clone(),
        source,
    })?;
    Ok(output)
}

fn editor_output_path(input: &Path, suffix: &str) -> PathBuf {
    let parent = input.parent().unwrap_or_else(|| Path::new(""));
    let stem = input
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(suffix);
    parent.join(format!("{stem}-{suffix}.png"))
}

#[derive(Clone, Copy)]
struct CheckedRect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

fn checked_rect(
    rect: Rect,
    image_width: u32,
    image_height: u32,
) -> Result<CheckedRect, EditorError> {
    rect.checked_for_image(image_width, image_height)
        .map(|rect| CheckedRect {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height,
        })
        .map_err(EditorError::from)
}

fn draw_outline(image: &mut RgbaImage, rect: Rect, color: Rgba<u8>) -> Result<(), EditorError> {
    let rect = checked_rect(rect, image.width(), image.height())?;
    let right = rect.x + rect.width - 1;
    let bottom = rect.y + rect.height - 1;

    draw_line(
        image,
        rect.x as i32,
        rect.y as i32,
        right as i32,
        rect.y as i32,
        color,
    );
    draw_line(
        image,
        rect.x as i32,
        bottom as i32,
        right as i32,
        bottom as i32,
        color,
    );
    draw_line(
        image,
        rect.x as i32,
        rect.y as i32,
        rect.x as i32,
        bottom as i32,
        color,
    );
    draw_line(
        image,
        right as i32,
        rect.y as i32,
        right as i32,
        bottom as i32,
        color,
    );
    Ok(())
}

fn draw_arrow(image: &mut RgbaImage, rect: Rect, color: Rgba<u8>) -> Result<(), EditorError> {
    let rect = checked_rect(rect, image.width(), image.height())?;
    let start_x = rect.x as i32;
    let start_y = rect.y as i32;
    let end_x = (rect.x + rect.width - 1) as i32;
    let end_y = (rect.y + rect.height - 1) as i32;

    draw_line(image, start_x, start_y, end_x, end_y, color);
    draw_line(image, end_x, end_y, end_x - 8, end_y, color);
    draw_line(image, end_x, end_y, end_x, end_y - 8, color);
    Ok(())
}

fn draw_marker(image: &mut RgbaImage, rect: Rect, marker: u8) -> Result<(), EditorError> {
    let rect = checked_rect(rect, image.width(), image.height())?;
    let cx = rect.x + rect.width / 2;
    let cy = rect.y + rect.height / 2;
    let radius = 10;
    let fill = Rgba([255, 210, 0, 255]);
    let ink = Rgba([0, 0, 0, 255]);

    for y in -radius..=radius {
        for x in -radius..=radius {
            if x * x + y * y <= radius * radius {
                put_pixel_checked(image, cx as i32 + x, cy as i32 + y, fill);
            }
        }
    }
    draw_digit(image, cx as i32 - 3, cy as i32 - 5, marker, ink);
    Ok(())
}

fn draw_text_label(image: &mut RgbaImage, rect: Rect, text: &str) -> Result<(), EditorError> {
    let rect = checked_rect(rect, image.width(), image.height())?;
    let text = label_text(text);
    if text.is_empty() {
        return Ok(());
    }

    let scale = 2;
    let padding = 4;
    let char_advance = 6 * scale + 2;
    let max_chars =
        ((rect.width.saturating_sub(padding as u32 * 2)) as i32 / char_advance).max(1) as usize;
    let text = text.chars().take(max_chars).collect::<String>();
    let text_width = (text.chars().count() as i32 * char_advance - 2).max(1);
    let background_width = rect.width.min((text_width + padding * 2) as u32);
    let background_height = rect.height.min((7 * scale + padding * 2) as u32);
    let background = Rgba([255, 210, 0, 255]);
    let ink = Rgba([0, 0, 0, 255]);

    for y in rect.y..rect.y + background_height {
        for x in rect.x..rect.x + background_width {
            image.put_pixel(x, y, background);
        }
    }

    let mut x = rect.x as i32 + padding;
    let y = rect.y as i32 + padding;
    for ch in text.chars() {
        draw_char(image, x, y, ch, scale, ink);
        x += char_advance;
    }

    Ok(())
}

fn label_text(text: &str) -> String {
    text.chars()
        .filter(|ch| is_label_char(*ch))
        .take(64)
        .collect()
}

fn draw_char(image: &mut RgbaImage, x: i32, y: i32, ch: char, scale: i32, color: Rgba<u8>) {
    let glyph = glyph_for(ch);
    for (row, pattern) in glyph.iter().enumerate() {
        for (column, value) in pattern.bytes().enumerate() {
            if value != b'#' {
                continue;
            }
            let left = x + column as i32 * scale;
            let top = y + row as i32 * scale;
            for dy in 0..scale {
                for dx in 0..scale {
                    put_pixel_checked(image, left + dx, top + dy, color);
                }
            }
        }
    }
}

fn glyph_for(ch: char) -> [&'static str; 7] {
    match ch.to_ascii_uppercase() {
        'A' => [
            ".###.", "#...#", "#...#", "#####", "#...#", "#...#", "#...#",
        ],
        'B' => [
            "####.", "#...#", "#...#", "####.", "#...#", "#...#", "####.",
        ],
        'C' => [
            ".###.", "#...#", "#....", "#....", "#....", "#...#", ".###.",
        ],
        'D' => [
            "####.", "#...#", "#...#", "#...#", "#...#", "#...#", "####.",
        ],
        'E' => [
            "#####", "#....", "#....", "####.", "#....", "#....", "#####",
        ],
        'F' => [
            "#####", "#....", "#....", "####.", "#....", "#....", "#....",
        ],
        'G' => [
            ".###.", "#...#", "#....", "#.###", "#...#", "#...#", ".###.",
        ],
        'H' => [
            "#...#", "#...#", "#...#", "#####", "#...#", "#...#", "#...#",
        ],
        'I' => [
            "#####", "..#..", "..#..", "..#..", "..#..", "..#..", "#####",
        ],
        'J' => [
            "#####", "...#.", "...#.", "...#.", "...#.", "#..#.", ".##..",
        ],
        'K' => [
            "#...#", "#..#.", "#.#..", "##...", "#.#..", "#..#.", "#...#",
        ],
        'L' => [
            "#....", "#....", "#....", "#....", "#....", "#....", "#####",
        ],
        'M' => [
            "#...#", "##.##", "#.#.#", "#.#.#", "#...#", "#...#", "#...#",
        ],
        'N' => [
            "#...#", "##..#", "##..#", "#.#.#", "#..##", "#..##", "#...#",
        ],
        'O' => [
            ".###.", "#...#", "#...#", "#...#", "#...#", "#...#", ".###.",
        ],
        'P' => [
            "####.", "#...#", "#...#", "####.", "#....", "#....", "#....",
        ],
        'Q' => [
            ".###.", "#...#", "#...#", "#...#", "#.#.#", "#..#.", ".##.#",
        ],
        'R' => [
            "####.", "#...#", "#...#", "####.", "#.#..", "#..#.", "#...#",
        ],
        'S' => [
            ".####", "#....", "#....", ".###.", "....#", "....#", "####.",
        ],
        'T' => [
            "#####", "..#..", "..#..", "..#..", "..#..", "..#..", "..#..",
        ],
        'U' => [
            "#...#", "#...#", "#...#", "#...#", "#...#", "#...#", ".###.",
        ],
        'V' => [
            "#...#", "#...#", "#...#", "#...#", "#...#", ".#.#.", "..#..",
        ],
        'W' => [
            "#...#", "#...#", "#...#", "#.#.#", "#.#.#", "##.##", "#...#",
        ],
        'X' => [
            "#...#", "#...#", ".#.#.", "..#..", ".#.#.", "#...#", "#...#",
        ],
        'Y' => [
            "#...#", "#...#", ".#.#.", "..#..", "..#..", "..#..", "..#..",
        ],
        'Z' => [
            "#####", "....#", "...#.", "..#..", ".#...", "#....", "#####",
        ],
        '0' => [
            ".###.", "#...#", "#..##", "#.#.#", "##..#", "#...#", ".###.",
        ],
        '1' => [
            "..#..", ".##..", "..#..", "..#..", "..#..", "..#..", ".###.",
        ],
        '2' => [
            ".###.", "#...#", "....#", "...#.", "..#..", ".#...", "#####",
        ],
        '3' => [
            "####.", "....#", "....#", ".###.", "....#", "....#", "####.",
        ],
        '4' => [
            "#...#", "#...#", "#...#", "#####", "....#", "....#", "....#",
        ],
        '5' => [
            "#####", "#....", "#....", "####.", "....#", "....#", "####.",
        ],
        '6' => [
            ".###.", "#....", "#....", "####.", "#...#", "#...#", ".###.",
        ],
        '7' => [
            "#####", "....#", "...#.", "..#..", ".#...", ".#...", ".#...",
        ],
        '8' => [
            ".###.", "#...#", "#...#", ".###.", "#...#", "#...#", ".###.",
        ],
        '9' => [
            ".###.", "#...#", "#...#", ".####", "....#", "....#", ".###.",
        ],
        '!' => [
            "..#..", "..#..", "..#..", "..#..", "..#..", ".....", "..#..",
        ],
        '?' => [
            ".###.", "#...#", "....#", "...#.", "..#..", ".....", "..#..",
        ],
        '-' => [
            ".....", ".....", ".....", "#####", ".....", ".....", ".....",
        ],
        '_' => [
            ".....", ".....", ".....", ".....", ".....", ".....", "#####",
        ],
        ':' => [
            ".....", "..#..", "..#..", ".....", "..#..", "..#..", ".....",
        ],
        '.' => [
            ".....", ".....", ".....", ".....", ".....", "..#..", "..#..",
        ],
        ',' => [
            ".....", ".....", ".....", ".....", ".....", "..#..", ".#...",
        ],
        '/' => [
            "....#", "...#.", "...#.", "..#..", ".#...", ".#...", "#....",
        ],
        ' ' => [
            ".....", ".....", ".....", ".....", ".....", ".....", ".....",
        ],
        _ => [
            "#####", "....#", "...#.", "..#..", ".....", "..#..", ".....",
        ],
    }
}

fn draw_digit(image: &mut RgbaImage, x: i32, y: i32, digit: u8, color: Rgba<u8>) {
    const SEGMENTS: [[bool; 7]; 10] = [
        [true, true, true, true, true, true, false],
        [false, true, true, false, false, false, false],
        [true, true, false, true, true, false, true],
        [true, true, true, true, false, false, true],
        [false, true, true, false, false, true, true],
        [true, false, true, true, false, true, true],
        [true, false, true, true, true, true, true],
        [true, true, true, false, false, false, false],
        [true, true, true, true, true, true, true],
        [true, true, true, true, false, true, true],
    ];

    for (index, active) in SEGMENTS[digit as usize].iter().enumerate() {
        if !active {
            continue;
        }
        match index {
            0 => draw_line(image, x, y, x + 6, y, color),
            1 => draw_line(image, x + 6, y, x + 6, y + 5, color),
            2 => draw_line(image, x + 6, y + 5, x + 6, y + 10, color),
            3 => draw_line(image, x, y + 10, x + 6, y + 10, color),
            4 => draw_line(image, x, y + 5, x, y + 10, color),
            5 => draw_line(image, x, y, x, y + 5, color),
            6 => draw_line(image, x, y + 5, x + 6, y + 5, color),
            _ => {}
        }
    }
}

fn draw_line(image: &mut RgbaImage, mut x0: i32, mut y0: i32, x1: i32, y1: i32, color: Rgba<u8>) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut error = dx + dy;

    loop {
        put_pixel_checked(image, x0, y0, color);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let error2 = 2 * error;
        if error2 >= dy {
            error += dy;
            x0 += sx;
        }
        if error2 <= dx {
            error += dx;
            y0 += sy;
        }
    }
}

fn put_pixel_checked(image: &mut RgbaImage, x: i32, y: i32, color: Rgba<u8>) {
    if x >= 0 && y >= 0 && x < image.width() as i32 && y < image.height() as i32 {
        image.put_pixel(x as u32, y as u32, color);
    }
}

fn pressed_marker_key(window: &Window) -> Option<u8> {
    [
        (Key::Key1, 1),
        (Key::Key2, 2),
        (Key::Key3, 3),
        (Key::Key4, 4),
        (Key::Key5, 5),
        (Key::Key6, 6),
        (Key::Key7, 7),
        (Key::Key8, 8),
        (Key::Key9, 9),
    ]
    .into_iter()
    .find_map(|(key, digit)| window.is_key_pressed(key, KeyRepeat::No).then_some(digit))
}

#[derive(Clone)]
struct ImageView {
    width: usize,
    height: usize,
    scale: f32,
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

        Self {
            width,
            height,
            scale,
        }
    }

    fn buffer_for(&self, image: &RgbaImage) -> Vec<u32> {
        let mut buffer = vec![0; self.width * self.height];

        for y in 0..self.height {
            for x in 0..self.width {
                let source_x = ((x as f32 / self.scale).floor() as u32).min(image.width() - 1);
                let source_y = ((y as f32 / self.scale).floor() as u32).min(image.height() - 1);
                let pixel = image.get_pixel(source_x, source_y);
                buffer[y * self.width + x] =
                    u32::from(pixel[0]) << 16 | u32::from(pixel[1]) << 8 | u32::from(pixel[2]);
            }
        }

        buffer
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
                None,
                operation,
            )
            .unwrap();

            assert!(output.exists());
            assert_eq!(fs::read(&path).unwrap(), before);
            fs::remove_file(path).unwrap();
            fs::remove_file(output).unwrap();
        }
    }

    #[test]
    fn editor_operation_honors_explicit_output_path() {
        let path = temp_path("explicit-input.png");
        let output = temp_path("explicit-output.png");
        write_test_image(&path);

        let actual = apply_operation(
            &path,
            Rect {
                x: 1,
                y: 1,
                width: 2,
                height: 1,
            },
            Some(output.clone()),
            EditorOperation::Redact,
        )
        .unwrap();

        assert_eq!(actual, output);
        assert!(actual.exists());
        assert!(!path.with_file_name("explicit-input-redacted.png").exists());
        fs::remove_file(path).unwrap();
        fs::remove_file(output).unwrap();
    }

    #[test]
    fn save_current_image_writes_edited_copy_and_preserves_input() {
        let path = temp_path("save-input.png");
        write_test_image(&path);
        let original = fs::read(&path).unwrap();
        let mut image = load_image(&path).unwrap();
        draw_outline(
            &mut image,
            Rect {
                x: 0,
                y: 0,
                width: 2,
                height: 2,
            },
            Rgba([255, 210, 0, 255]),
        )
        .unwrap();

        let output = save_current_image(&path, &image, None).unwrap();

        let expected_name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(|stem| format!("{stem}-edited.png"))
            .unwrap();
        assert_eq!(
            output.file_name().and_then(|name| name.to_str()),
            Some(expected_name.as_str())
        );
        assert_eq!(fs::read(&path).unwrap(), original);
        assert_ne!(fs::read(&output).unwrap(), original);
        fs::remove_file(path).unwrap();
        fs::remove_file(output).unwrap();
    }

    #[test]
    fn annotation_tools_change_pixels_without_changing_dimensions() {
        let mut image = RgbaImage::from_pixel(24, 24, Rgba([255, 255, 255, 255]));

        draw_outline(
            &mut image,
            Rect {
                x: 2,
                y: 2,
                width: 8,
                height: 8,
            },
            Rgba([255, 210, 0, 255]),
        )
        .unwrap();
        draw_arrow(
            &mut image,
            Rect {
                x: 4,
                y: 4,
                width: 12,
                height: 12,
            },
            Rgba([255, 210, 0, 255]),
        )
        .unwrap();
        draw_marker(
            &mut image,
            Rect {
                x: 8,
                y: 8,
                width: 8,
                height: 8,
            },
            3,
        )
        .unwrap();

        assert_eq!(image.dimensions(), (24, 24));
        assert_eq!(image.get_pixel(2, 2), &Rgba([255, 210, 0, 255]));
        assert_ne!(image.get_pixel(12, 12), &Rgba([255, 255, 255, 255]));
    }

    #[test]
    fn text_label_changes_pixels_without_changing_dimensions() {
        let mut image = RgbaImage::from_pixel(80, 24, Rgba([255, 255, 255, 255]));

        draw_text_label(
            &mut image,
            Rect {
                x: 2,
                y: 2,
                width: 70,
                height: 18,
            },
            "Bug 1",
        )
        .unwrap();

        assert_eq!(image.dimensions(), (80, 24));
        assert_eq!(image.get_pixel(2, 2), &Rgba([255, 210, 0, 255]));
        assert_eq!(image.get_pixel(6, 6), &Rgba([0, 0, 0, 255]));
    }

    #[test]
    fn label_text_keeps_printable_ascii_only() {
        assert_eq!(label_text("Hi\tthere\n!"), "Hithere!");
    }

    #[test]
    fn text_draft_appends_and_removes_text() {
        let mut draft = TextDraft::new(Rect {
            x: 1,
            y: 2,
            width: 30,
            height: 20,
        });

        draft.append("hello");
        draft.pop();

        assert_eq!(draft.text, "hell");
    }

    #[test]
    fn crop_current_image_crops_current_canvas() {
        let path = temp_path("crop-input.png");
        let output = temp_path("crop-output.png");
        let mut image = RgbaImage::from_pixel(4, 4, Rgba([255, 255, 255, 255]));
        image.put_pixel(1, 1, Rgba([1, 2, 3, 255]));
        image.save(&path).unwrap();

        let actual = crop_current_image(
            &path,
            &image,
            Rect {
                x: 1,
                y: 1,
                width: 2,
                height: 2,
            },
            Some(output.clone()),
        )
        .unwrap();

        let cropped = image::open(&actual).unwrap().to_rgba8();
        assert_eq!(actual, output);
        assert_eq!(cropped.dimensions(), (2, 2));
        assert_eq!(cropped.get_pixel(0, 0), &Rgba([1, 2, 3, 255]));
        fs::remove_file(path).unwrap();
        fs::remove_file(output).unwrap();
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
