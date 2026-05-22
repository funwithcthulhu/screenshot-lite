use std::{
    fmt,
    path::{Path, PathBuf},
    str::FromStr,
};

use image::{Rgba, RgbaImage, imageops};
use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImageRect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum RectParseError {
    #[error("rect must use x,y,w,h")]
    WrongPartCount,
    #[error("rect contains an invalid number")]
    InvalidNumber,
    #[error("rect width and height must be greater than zero")]
    Empty,
}

#[derive(Debug, Error)]
pub enum RedactError {
    #[error("rect is outside image bounds ({image_width}x{image_height})")]
    RectOutOfBounds { image_width: u32, image_height: u32 },
    #[error("failed to open {path}: {source}")]
    Open {
        path: PathBuf,
        source: image::ImageError,
    },
    #[error("failed to save {path}: {source}")]
    Save {
        path: PathBuf,
        source: image::ImageError,
    },
}

impl Rect {
    pub fn checked_for_image(
        self,
        image_width: u32,
        image_height: u32,
    ) -> Result<ImageRect, RedactError> {
        if self.x < 0 || self.y < 0 {
            return Err(RedactError::RectOutOfBounds {
                image_width,
                image_height,
            });
        }

        let x = self.x as u32;
        let y = self.y as u32;
        let Some(right) = x.checked_add(self.width) else {
            return Err(RedactError::RectOutOfBounds {
                image_width,
                image_height,
            });
        };
        let Some(bottom) = y.checked_add(self.height) else {
            return Err(RedactError::RectOutOfBounds {
                image_width,
                image_height,
            });
        };

        if right > image_width || bottom > image_height {
            return Err(RedactError::RectOutOfBounds {
                image_width,
                image_height,
            });
        }

        Ok(ImageRect {
            x,
            y,
            width: self.width,
            height: self.height,
        })
    }
}

impl FromStr for Rect {
    type Err = RectParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let parts: Vec<_> = value.split(',').map(str::trim).collect();
        let [x, y, width, height] = parts.as_slice() else {
            return Err(RectParseError::WrongPartCount);
        };

        let x = x.parse().map_err(|_| RectParseError::InvalidNumber)?;
        let y = y.parse().map_err(|_| RectParseError::InvalidNumber)?;
        let width = width.parse().map_err(|_| RectParseError::InvalidNumber)?;
        let height = height.parse().map_err(|_| RectParseError::InvalidNumber)?;

        if width == 0 || height == 0 {
            return Err(RectParseError::Empty);
        }

        Ok(Self {
            x,
            y,
            width,
            height,
        })
    }
}

impl fmt::Display for Rect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{},{},{},{}", self.x, self.y, self.width, self.height)
    }
}

pub fn redact_file(
    input: &Path,
    rect: Rect,
    output: Option<PathBuf>,
) -> Result<PathBuf, RedactError> {
    let mut image = image::open(input)
        .map_err(|source| RedactError::Open {
            path: input.to_path_buf(),
            source,
        })?
        .to_rgba8();

    apply_redaction(&mut image, rect, Rgba([0, 0, 0, 255]))?;

    let output = output.unwrap_or_else(|| default_output_path(input));
    image.save(&output).map_err(|source| RedactError::Save {
        path: output.clone(),
        source,
    })?;

    Ok(output)
}

pub fn highlight_file(
    input: &Path,
    rect: Rect,
    output: Option<PathBuf>,
) -> Result<PathBuf, RedactError> {
    let mut image = image::open(input)
        .map_err(|source| RedactError::Open {
            path: input.to_path_buf(),
            source,
        })?
        .to_rgba8();

    apply_highlight(&mut image, rect)?;

    let output = output.unwrap_or_else(|| sibling_output_path(input, "highlighted"));
    image.save(&output).map_err(|source| RedactError::Save {
        path: output.clone(),
        source,
    })?;

    Ok(output)
}

pub fn crop_file(
    input: &Path,
    rect: Rect,
    output: Option<PathBuf>,
) -> Result<PathBuf, RedactError> {
    let image = image::open(input)
        .map_err(|source| RedactError::Open {
            path: input.to_path_buf(),
            source,
        })?
        .to_rgba8();
    let rect = rect.checked_for_image(image.width(), image.height())?;
    let cropped = imageops::crop_imm(&image, rect.x, rect.y, rect.width, rect.height).to_image();

    let output = output.unwrap_or_else(|| sibling_output_path(input, "cropped"));
    cropped.save(&output).map_err(|source| RedactError::Save {
        path: output.clone(),
        source,
    })?;

    Ok(output)
}

pub fn apply_redaction(
    image: &mut RgbaImage,
    rect: Rect,
    color: Rgba<u8>,
) -> Result<(), RedactError> {
    let rect = rect.checked_for_image(image.width(), image.height())?;

    for y in rect.y..rect.y + rect.height {
        for x in rect.x..rect.x + rect.width {
            image.put_pixel(x, y, color);
        }
    }

    Ok(())
}

pub fn apply_highlight(image: &mut RgbaImage, rect: Rect) -> Result<(), RedactError> {
    let rect = rect.checked_for_image(image.width(), image.height())?;
    let color = Rgba([255, 230, 0, 255]);

    for y in rect.y..rect.y + rect.height {
        for x in rect.x..rect.x + rect.width {
            let pixel = image.get_pixel_mut(x, y);
            pixel.0 = [
                average(pixel.0[0], color.0[0]),
                average(pixel.0[1], color.0[1]),
                average(pixel.0[2], color.0[2]),
                pixel.0[3],
            ];
        }
    }

    Ok(())
}

fn default_output_path(input: &Path) -> PathBuf {
    sibling_output_path(input, "redacted")
}

fn sibling_output_path(input: &Path, suffix: &str) -> PathBuf {
    let parent = input.parent().unwrap_or_else(|| Path::new(""));
    let stem = input
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(suffix);
    parent.join(format!("{stem}-{suffix}.png"))
}

fn average(a: u8, b: u8) -> u8 {
    ((u16::from(a) + u16::from(b)) / 2) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn parses_rect() {
        assert_eq!(
            "10,20,200,80".parse::<Rect>().unwrap(),
            Rect {
                x: 10,
                y: 20,
                width: 200,
                height: 80
            }
        );
    }

    #[test]
    fn rejects_empty_rect() {
        assert_eq!("1,2,0,4".parse::<Rect>(), Err(RectParseError::Empty));
        assert_eq!("1,2,3,0".parse::<Rect>(), Err(RectParseError::Empty));
    }

    #[test]
    fn accepts_rect_inside_image() {
        let rect = Rect {
            x: 1,
            y: 2,
            width: 3,
            height: 4,
        };

        assert_eq!(
            rect.checked_for_image(10, 10).unwrap(),
            ImageRect {
                x: 1,
                y: 2,
                width: 3,
                height: 4
            }
        );
    }

    #[test]
    fn rejects_rect_outside_image() {
        let rect = Rect {
            x: 8,
            y: 8,
            width: 3,
            height: 1,
        };

        assert!(matches!(
            rect.checked_for_image(10, 10),
            Err(RedactError::RectOutOfBounds {
                image_width: 10,
                image_height: 10
            })
        ));
    }

    #[test]
    fn rejects_negative_rect_coordinates() {
        let rect = Rect {
            x: -1,
            y: 0,
            width: 1,
            height: 1,
        };

        assert!(matches!(
            rect.checked_for_image(10, 10),
            Err(RedactError::RectOutOfBounds {
                image_width: 10,
                image_height: 10
            })
        ));

        let rect = Rect {
            x: 0,
            y: -1,
            width: 1,
            height: 1,
        };

        assert!(matches!(
            rect.checked_for_image(10, 10),
            Err(RedactError::RectOutOfBounds {
                image_width: 10,
                image_height: 10
            })
        ));
    }

    #[test]
    fn redaction_changes_pixels() {
        let mut image = RgbaImage::from_pixel(4, 4, Rgba([255, 255, 255, 255]));

        apply_redaction(
            &mut image,
            Rect {
                x: 1,
                y: 1,
                width: 2,
                height: 2,
            },
            Rgba([0, 0, 0, 255]),
        )
        .unwrap();

        assert_eq!(image.get_pixel(1, 1), &Rgba([0, 0, 0, 255]));
        assert_eq!(image.get_pixel(0, 0), &Rgba([255, 255, 255, 255]));
    }

    #[test]
    fn highlight_changes_pixels_and_preserves_alpha() {
        let mut image = RgbaImage::from_pixel(2, 2, Rgba([10, 20, 30, 200]));

        apply_highlight(
            &mut image,
            Rect {
                x: 0,
                y: 0,
                width: 1,
                height: 1,
            },
        )
        .unwrap();

        assert_eq!(image.get_pixel(0, 0), &Rgba([132, 125, 15, 200]));
        assert_eq!(image.get_pixel(1, 1), &Rgba([10, 20, 30, 200]));
    }

    #[test]
    fn default_output_path_is_predictable() {
        assert_eq!(
            default_output_path(Path::new("input.png")),
            PathBuf::from("input-redacted.png")
        );
        assert_eq!(
            default_output_path(Path::new("nested/input.png")),
            PathBuf::from("nested").join("input-redacted.png")
        );
    }

    #[test]
    fn redact_file_writes_new_file_and_leaves_input_unchanged() {
        let dir = temp_test_dir("default-output");
        let input = dir.join("input.png");
        write_test_image(&input);
        let original = fs::read(&input).unwrap();

        let output = redact_file(
            &input,
            Rect {
                x: 1,
                y: 1,
                width: 2,
                height: 2,
            },
            None,
        )
        .unwrap();

        assert_eq!(output, dir.join("input-redacted.png"));
        assert_eq!(fs::read(&input).unwrap(), original);
        assert_ne!(fs::read(&output).unwrap(), original);

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn redact_file_honors_explicit_output_path() {
        let dir = temp_test_dir("explicit-output");
        let input = dir.join("input.png");
        let output = dir.join("chosen.png");
        write_test_image(&input);

        let actual = redact_file(
            &input,
            Rect {
                x: 0,
                y: 0,
                width: 1,
                height: 1,
            },
            Some(output.clone()),
        )
        .unwrap();

        assert_eq!(actual, output);
        assert!(actual.exists());
        assert!(!dir.join("input-redacted.png").exists());

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn redact_file_preserves_dimensions() {
        let dir = temp_test_dir("dimensions");
        let input = dir.join("input.png");
        let output = dir.join("output.png");
        write_test_image(&input);

        redact_file(
            &input,
            Rect {
                x: 1,
                y: 1,
                width: 2,
                height: 2,
            },
            Some(output.clone()),
        )
        .unwrap();

        let image = image::open(output).unwrap();
        assert_eq!(image.width(), 4);
        assert_eq!(image.height(), 3);

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn crop_file_writes_cropped_copy() {
        let dir = temp_test_dir("crop");
        let input = dir.join("input.png");
        let output = dir.join("cropped.png");
        write_test_image(&input);

        let actual = crop_file(
            &input,
            Rect {
                x: 1,
                y: 1,
                width: 2,
                height: 1,
            },
            Some(output.clone()),
        )
        .unwrap();

        let image = image::open(&actual).unwrap();
        assert_eq!(actual, output);
        assert_eq!(image.width(), 2);
        assert_eq!(image.height(), 1);

        fs::remove_dir_all(dir).unwrap();
    }

    fn write_test_image(path: &Path) {
        let mut image = RgbaImage::from_pixel(4, 3, Rgba([255, 255, 255, 255]));
        image.put_pixel(0, 0, Rgba([10, 20, 30, 255]));
        image.save(path).unwrap();
    }

    fn temp_test_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("shotlite-{name}-{unique}"));
        fs::create_dir(&path).unwrap();
        path
    }
}
