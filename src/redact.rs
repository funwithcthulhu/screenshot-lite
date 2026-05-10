use std::{
    fmt,
    path::{Path, PathBuf},
    str::FromStr,
};

use image::{Rgba, RgbaImage};
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

#[derive(Debug, Error)]
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

fn default_output_path(input: &Path) -> PathBuf {
    let parent = input.parent().unwrap_or_else(|| Path::new(""));
    let stem = input
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("redacted");
    parent.join(format!("{stem}-redacted.png"))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!("1,2,0,4".parse::<Rect>().is_err());
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

        assert!(rect.checked_for_image(10, 10).is_err());
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
}
