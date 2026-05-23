use std::{
    path::{Path, PathBuf},
    process::Command,
};

use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PostCaptureAction {
    Open,
    Reveal,
}

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

pub fn post_capture_actions(open: bool, reveal: bool) -> Vec<PostCaptureAction> {
    let mut actions = Vec::new();
    if reveal {
        actions.push(PostCaptureAction::Reveal);
    }
    if open {
        actions.push(PostCaptureAction::Open);
    }
    actions
}

pub fn run_post_capture_actions(
    path: &Path,
    actions: &[PostCaptureAction],
) -> Result<(), FileActionError> {
    for action in actions {
        match action {
            PostCaptureAction::Open => open(path)?,
            PostCaptureAction::Reveal => reveal(path)?,
        }
    }

    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_flags_have_no_post_capture_actions() {
        assert_eq!(post_capture_actions(false, false), []);
    }

    #[test]
    fn open_flag_maps_to_open_action() {
        assert_eq!(post_capture_actions(true, false), [PostCaptureAction::Open]);
    }

    #[test]
    fn reveal_flag_maps_to_reveal_action() {
        assert_eq!(
            post_capture_actions(false, true),
            [PostCaptureAction::Reveal]
        );
    }

    #[test]
    fn open_and_reveal_keep_existing_order() {
        assert_eq!(
            post_capture_actions(true, true),
            [PostCaptureAction::Reveal, PostCaptureAction::Open]
        );
    }
}
