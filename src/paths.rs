use std::path::PathBuf;

use directories::{BaseDirs, UserDirs};

pub fn config_file() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("SHOTLITE_CONFIG_DIR") {
        return Some(config_file_in(std::path::Path::new(&path)));
    }

    BaseDirs::new().map(|dirs| config_file_in(dirs.config_dir()))
}

pub fn default_output_dir() -> PathBuf {
    let picture_dir = UserDirs::new().and_then(|dirs| dirs.picture_dir().map(PathBuf::from));
    let home_dir = BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf());

    default_output_dir_from(picture_dir.as_deref(), home_dir.as_deref())
        .unwrap_or_else(|| PathBuf::from("screenshots"))
}

pub fn config_file_in(config_dir: &std::path::Path) -> PathBuf {
    config_dir.join("shotlite").join("config.toml")
}

pub fn default_output_dir_from(
    picture_dir: Option<&std::path::Path>,
    home_dir: Option<&std::path::Path>,
) -> Option<PathBuf> {
    picture_dir
        .map(|path| path.join("Screenshots"))
        .or_else(|| home_dir.map(|path| path.join("Pictures").join("Screenshots")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn config_file_uses_shotlite_config_toml() {
        assert_eq!(
            config_file_in(std::path::Path::new("config")),
            PathBuf::from("config").join("shotlite").join("config.toml")
        );
    }

    #[test]
    fn config_file_honors_config_dir_override() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("shotlite-paths-override-{unique}"));
        unsafe {
            std::env::set_var("SHOTLITE_CONFIG_DIR", &root);
        }

        let path = config_file().unwrap();

        assert_eq!(path, root.join("shotlite").join("config.toml"));
        unsafe {
            std::env::remove_var("SHOTLITE_CONFIG_DIR");
        }
    }

    #[test]
    fn config_file_in_does_not_create_config_directory() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("shotlite-paths-{unique}"));

        let path = config_file_in(&root);

        assert_eq!(path, root.join("shotlite").join("config.toml"));
        assert!(!root.exists());
    }

    #[test]
    fn default_output_prefers_picture_dir() {
        assert_eq!(
            default_output_dir_from(
                Some(std::path::Path::new("Pictures")),
                Some(std::path::Path::new("Home")),
            ),
            Some(PathBuf::from("Pictures").join("Screenshots"))
        );
    }

    #[test]
    fn default_output_falls_back_to_home_pictures() {
        assert_eq!(
            default_output_dir_from(None, Some(std::path::Path::new("Home"))),
            Some(PathBuf::from("Home").join("Pictures").join("Screenshots"))
        );
    }
}
