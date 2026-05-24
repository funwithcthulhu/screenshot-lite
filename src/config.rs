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

        Self::load_from(&path)
    }

    fn load_from(path: &Path) -> Result<Self, ConfigError> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let text = fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_path_buf(),
            source,
        })?;

        toml::from_str(&text).map_err(|source| ConfigError::Parse {
            path: path.to_path_buf(),
            source,
        })
    }

    pub fn save(&self) -> Result<PathBuf, ConfigError> {
        let path = paths::config_file().ok_or(ConfigError::MissingConfigDir)?;
        self.save_to(&path)?;
        Ok(path)
    }

    fn save_to(&self, path: &Path) -> Result<(), ConfigError> {
        write_config(self, path)
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

    let text = config_toml_for_write(config, path)?;
    fs::write(path, text).map_err(|source| ConfigError::Write {
        path: path.to_path_buf(),
        source,
    })
}

fn config_toml_for_write(config: &Config, path: &Path) -> Result<String, ConfigError> {
    let mut table = if path.exists() {
        let text = fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        toml::from_str::<toml::Table>(&text).map_err(|source| ConfigError::Parse {
            path: path.to_path_buf(),
            source,
        })?
    } else {
        toml::Table::new()
    };
    table.insert(
        "output_dir".to_owned(),
        toml::Value::String(config.output_dir.to_string_lossy().to_string()),
    );

    Ok(toml::to_string_pretty(&table)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

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

    #[test]
    fn missing_config_file_uses_default_without_creating_file() {
        let dir = temp_test_dir("missing");
        let path = dir.join("shotlite").join("config.toml");

        let config = Config::load_from(&path).unwrap();

        assert_eq!(config.output_dir, Config::default().output_dir);
        assert!(!path.exists());
        assert!(!path.parent().unwrap().exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn malformed_config_file_reports_parse_error() {
        let dir = temp_test_dir("malformed");
        let path = dir.join("config.toml");
        fs::write(&path, "output_dir = [").unwrap();

        let error = Config::load_from(&path).unwrap_err().to_string();

        assert!(error.contains("failed to parse"));
        assert!(error.contains("config.toml"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn non_string_output_dir_reports_parse_error() {
        let dir = temp_test_dir("bad-output-dir");
        let path = dir.join("config.toml");
        fs::write(&path, "output_dir = 123").unwrap();

        let error = Config::load_from(&path).unwrap_err().to_string();

        assert!(error.contains("failed to parse"));
        assert!(error.contains("output_dir"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn save_preserves_unrelated_config_fields() {
        let dir = temp_test_dir("preserve");
        let path = dir.join("config.toml");
        fs::write(
            &path,
            r#"
output_dir = "old"
theme = "plain"

[future]
enabled = true
"#,
        )
        .unwrap();

        Config {
            output_dir: PathBuf::from("new shots"),
        }
        .save_to(&path)
        .unwrap();
        let value: toml::Value = toml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();

        assert_eq!(value["output_dir"].as_str(), Some("new shots"));
        assert_eq!(value["theme"].as_str(), Some("plain"));
        assert_eq!(value["future"]["enabled"].as_bool(), Some(true));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn paths_with_spaces_write_and_read_correctly() {
        let dir = temp_test_dir("spaces");
        let path = dir.join("config.toml");
        let output_dir = dir.join("Screenshots With Spaces");

        Config {
            output_dir: output_dir.clone(),
        }
        .save_to(&path)
        .unwrap();

        assert_eq!(Config::load_from(&path).unwrap().output_dir, output_dir);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn nonexistent_output_dir_is_stored_without_validation() {
        let dir = temp_test_dir("nonexistent-output");
        let path = dir.join("config.toml");
        let output_dir = dir.join("does-not-exist");

        Config {
            output_dir: output_dir.clone(),
        }
        .save_to(&path)
        .unwrap();

        assert!(!output_dir.exists());
        assert_eq!(Config::load_from(&path).unwrap().output_dir, output_dir);
        fs::remove_dir_all(dir).unwrap();
    }

    fn temp_test_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("shotlite-config-{name}-{unique}"));
        fs::create_dir(&path).unwrap();
        path
    }
}
