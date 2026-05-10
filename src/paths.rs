use std::path::PathBuf;

use directories::{BaseDirs, UserDirs};

pub fn config_file() -> Option<PathBuf> {
    BaseDirs::new().map(|dirs| dirs.config_dir().join("shotlite").join("config.toml"))
}

pub fn default_output_dir() -> PathBuf {
    UserDirs::new()
        .and_then(|dirs| dirs.picture_dir().map(|path| path.join("Screenshots")))
        .or_else(|| {
            BaseDirs::new().map(|dirs| dirs.home_dir().join("Pictures").join("Screenshots"))
        })
        .unwrap_or_else(|| PathBuf::from("screenshots"))
}
