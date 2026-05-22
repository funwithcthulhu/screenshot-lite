use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Local};
use image::{RgbaImage, imageops};
use thiserror::Error;
use xcap::Monitor;

use crate::redact::Rect;

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("no monitors found")]
    NoMonitors,
    #[error("capture failed: {0}")]
    Capture(#[from] xcap::XCapError),
    #[error("failed to create output directory {path}: {source}")]
    CreateOutputDir {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to save {path}: {source}")]
    Save {
        path: PathBuf,
        source: image::ImageError,
    },
    #[error("interactive region selection is not implemented; pass --rect x,y,w,h")]
    InteractiveRegionUnsupported,
    #[error("region {rect} is not fully inside one monitor")]
    RegionOutOfBounds { rect: Rect },
}

pub struct CaptureResult {
    pub path: PathBuf,
    pub image: RgbaImage,
}

pub fn capture_full(output_dir: &Path) -> Result<CaptureResult, CaptureError> {
    let monitors = Monitor::all()?;
    let image = capture_monitors(&monitors)?;
    save_capture(output_dir, image)
}

pub fn capture_region(
    output_dir: &Path,
    rect: Option<Rect>,
) -> Result<CaptureResult, CaptureError> {
    let rect = rect.ok_or(CaptureError::InteractiveRegionUnsupported)?;
    let monitors = Monitor::all()?;

    for monitor in monitors {
        let monitor_x = monitor.x()?;
        let monitor_y = monitor.y()?;
        let monitor_width = monitor.width()?;
        let monitor_height = monitor.height()?;

        if rect_inside_monitor(rect, monitor_x, monitor_y, monitor_width, monitor_height) {
            let local_x = (rect.x - monitor_x) as u32;
            let local_y = (rect.y - monitor_y) as u32;
            let image = monitor.capture_region(local_x, local_y, rect.width, rect.height)?;
            return save_capture(output_dir, image);
        }
    }

    Err(CaptureError::RegionOutOfBounds { rect })
}

pub fn screenshot_filename(now: DateTime<Local>) -> String {
    format!("screenshot-{}.png", now.format("%Y%m%d-%H%M%S"))
}

fn capture_monitors(monitors: &[Monitor]) -> Result<RgbaImage, CaptureError> {
    if monitors.is_empty() {
        return Err(CaptureError::NoMonitors);
    }

    let bounds = monitor_bounds(monitors)?;
    let mut canvas = RgbaImage::new(bounds.width, bounds.height);

    for monitor in monitors {
        let image = monitor.capture_image()?;
        let x = (monitor.x()? - bounds.x) as i64;
        let y = (monitor.y()? - bounds.y) as i64;
        imageops::overlay(&mut canvas, &image, x, y);
    }

    Ok(canvas)
}

fn save_capture(output_dir: &Path, image: RgbaImage) -> Result<CaptureResult, CaptureError> {
    fs::create_dir_all(output_dir).map_err(|source| CaptureError::CreateOutputDir {
        path: output_dir.to_path_buf(),
        source,
    })?;

    let path = output_dir.join(screenshot_filename(Local::now()));
    image.save(&path).map_err(|source| CaptureError::Save {
        path: path.clone(),
        source,
    })?;

    Ok(CaptureResult { path, image })
}

fn rect_inside_monitor(
    rect: Rect,
    monitor_x: i32,
    monitor_y: i32,
    monitor_width: u32,
    monitor_height: u32,
) -> bool {
    let Some(rect_right) = rect.x.checked_add_unsigned(rect.width) else {
        return false;
    };
    let Some(rect_bottom) = rect.y.checked_add_unsigned(rect.height) else {
        return false;
    };
    let Some(monitor_right) = monitor_x.checked_add_unsigned(monitor_width) else {
        return false;
    };
    let Some(monitor_bottom) = monitor_y.checked_add_unsigned(monitor_height) else {
        return false;
    };

    rect.x >= monitor_x
        && rect.y >= monitor_y
        && rect_right <= monitor_right
        && rect_bottom <= monitor_bottom
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Bounds {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MonitorGeometry {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

fn monitor_bounds(monitors: &[Monitor]) -> Result<Bounds, CaptureError> {
    let geometries = monitors
        .iter()
        .map(|monitor| {
            Ok(MonitorGeometry {
                x: monitor.x()?,
                y: monitor.y()?,
                width: monitor.width()?,
                height: monitor.height()?,
            })
        })
        .collect::<Result<Vec<_>, xcap::XCapError>>()?;

    bounds_from_geometries(&geometries)
}

fn bounds_from_geometries(monitors: &[MonitorGeometry]) -> Result<Bounds, CaptureError> {
    if monitors.is_empty() {
        return Err(CaptureError::NoMonitors);
    }

    let mut left = i32::MAX;
    let mut top = i32::MAX;
    let mut right = i32::MIN;
    let mut bottom = i32::MIN;

    for monitor in monitors {
        let monitor_right = monitor
            .x
            .checked_add_unsigned(monitor.width)
            .ok_or(CaptureError::NoMonitors)?;
        let monitor_bottom = monitor
            .y
            .checked_add_unsigned(monitor.height)
            .ok_or(CaptureError::NoMonitors)?;

        left = left.min(monitor.x);
        top = top.min(monitor.y);
        right = right.max(monitor_right);
        bottom = bottom.max(monitor_bottom);
    }

    Ok(Bounds {
        x: left,
        y: top,
        width: (right - left) as u32,
        height: (bottom - top) as u32,
    })
}

#[cfg(test)]
mod tests {
    use chrono::{Local, TimeZone};

    use super::*;

    #[test]
    fn filename_uses_expected_format() {
        let now = Local.with_ymd_and_hms(2026, 5, 9, 14, 3, 4).unwrap();

        assert_eq!(screenshot_filename(now), "screenshot-20260509-140304.png");
    }

    #[test]
    fn region_must_fit_inside_monitor() {
        let rect = Rect {
            x: 10,
            y: 20,
            width: 30,
            height: 40,
        };

        assert!(rect_inside_monitor(rect, 0, 0, 100, 100));
        assert!(!rect_inside_monitor(rect, 0, 0, 20, 100));
    }

    #[test]
    fn region_can_fit_inside_negative_monitor_coordinates() {
        let rect = Rect {
            x: -100,
            y: 20,
            width: 50,
            height: 40,
        };

        assert!(rect_inside_monitor(rect, -200, 0, 200, 100));
    }

    #[test]
    fn monitor_bounds_cover_negative_and_positive_coordinates() {
        let bounds = bounds_from_geometries(&[
            MonitorGeometry {
                x: -1280,
                y: 0,
                width: 1280,
                height: 720,
            },
            MonitorGeometry {
                x: 0,
                y: -100,
                width: 1920,
                height: 1080,
            },
        ])
        .unwrap();

        assert_eq!(
            bounds,
            Bounds {
                x: -1280,
                y: -100,
                width: 3200,
                height: 1080
            }
        );
    }
}
