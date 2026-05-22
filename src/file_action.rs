use std::{
    path::{Path, PathBuf},
    process::Command,
};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum FileActionError {
    #[error("failed to open {path}: {source}")]
    Open {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to reveal {path}: {source}")]
    Reveal {
        path: PathBuf,
        source: std::io::Error,
    },
    #[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
    #[error("open is not supported on this platform")]
    OpenUnsupported,
    #[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
    #[error("reveal is not supported on this platform")]
    RevealUnsupported,
}

pub fn open(path: &Path) -> Result<(), FileActionError> {
    open_path(path)
}

pub fn reveal(path: &Path) -> Result<(), FileActionError> {
    reveal_path(path)
}

#[cfg(target_os = "windows")]
fn open_path(path: &Path) -> Result<(), FileActionError> {
    Command::new("cmd")
        .args(["/C", "start", ""])
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|source| FileActionError::Open {
            path: path.to_path_buf(),
            source,
        })
}

#[cfg(target_os = "windows")]
fn reveal_path(path: &Path) -> Result<(), FileActionError> {
    let selector = format!("/select,{}", path.display());
    Command::new("explorer")
        .arg(selector)
        .spawn()
        .map(|_| ())
        .map_err(|source| FileActionError::Reveal {
            path: path.to_path_buf(),
            source,
        })
}

#[cfg(target_os = "macos")]
fn open_path(path: &Path) -> Result<(), FileActionError> {
    Command::new("open")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|source| FileActionError::Open {
            path: path.to_path_buf(),
            source,
        })
}

#[cfg(target_os = "macos")]
fn reveal_path(path: &Path) -> Result<(), FileActionError> {
    Command::new("open")
        .args(["-R"])
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|source| FileActionError::Reveal {
            path: path.to_path_buf(),
            source,
        })
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open_path(path: &Path) -> Result<(), FileActionError> {
    Command::new("xdg-open")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|source| FileActionError::Open {
            path: path.to_path_buf(),
            source,
        })
}

#[cfg(all(unix, not(target_os = "macos")))]
fn reveal_path(path: &Path) -> Result<(), FileActionError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    Command::new("xdg-open")
        .arg(parent)
        .spawn()
        .map(|_| ())
        .map_err(|source| FileActionError::Reveal {
            path: path.to_path_buf(),
            source,
        })
}

#[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
fn open_path(_path: &Path) -> Result<(), FileActionError> {
    Err(FileActionError::OpenUnsupported)
}

#[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
fn reveal_path(_path: &Path) -> Result<(), FileActionError> {
    Err(FileActionError::RevealUnsupported)
}
