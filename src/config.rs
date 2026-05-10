use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::paths;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("could not determine the user config directory")]
    MissingConfigDir,
    #[error("failed to read {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse {path}: {source}")]
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("failed to write {path}: {source}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to create {path}: {source}")]
    CreateDir {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to encode config: {0}")]
    Encode(#[from] toml::ser::Error),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub output_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            output_dir: paths::default_output_dir(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        let Some(path) = paths::config_file() else {
            return Ok(Self::default());
        };

        if !path.exists() {
            return Ok(Self::default());
        }

        let text = fs::read_to_string(&path).map_err(|source| ConfigError::Read {
            path: path.clone(),
            source,
        })?;

        toml::from_str(&text).map_err(|source| ConfigError::Parse { path, source })
    }

    pub fn save(&self) -> Result<PathBuf, ConfigError> {
        let path = paths::config_file().ok_or(ConfigError::MissingConfigDir)?;
        write_config(self, &path)?;
        Ok(path)
    }

    pub fn to_toml(&self) -> Result<String, ConfigError> {
        Ok(toml::to_string_pretty(self)?)
    }
}

fn write_config(config: &Config, path: &Path) -> Result<(), ConfigError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| ConfigError::CreateDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let text = config.to_toml()?;
    fs::write(path, text).map_err(|source| ConfigError::Write {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_an_output_dir() {
        let config = Config::default();

        assert!(!config.output_dir.as_os_str().is_empty());
    }

    #[test]
    fn config_toml_uses_output_dir_key() {
        let config = Config {
            output_dir: PathBuf::from("shots"),
        };

        let text = config.to_toml().unwrap();

        assert!(text.contains("output_dir"));
        assert!(text.contains("shots"));
    }

    #[test]
    fn config_toml_round_trips_output_dir() {
        let config = Config {
            output_dir: PathBuf::from("custom-shots"),
        };

        let text = config.to_toml().unwrap();
        let parsed: Config = toml::from_str(&text).unwrap();

        assert_eq!(parsed.output_dir, PathBuf::from("custom-shots"));
    }
}
