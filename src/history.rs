use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum HistoryError {
    #[error("output directory does not exist or is not a directory: {0}")]
    MissingOutputDir(PathBuf),
    #[error("failed to read output directory {path}: {source}")]
    ReadDir {
        path: PathBuf,
        source: std::io::Error,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryEntry {
    pub path: PathBuf,
    modified: Option<SystemTime>,
}

pub fn recent_pngs(output_dir: &Path, limit: usize) -> Result<Vec<HistoryEntry>, HistoryError> {
    if !output_dir.is_dir() {
        return Err(HistoryError::MissingOutputDir(output_dir.to_path_buf()));
    }

    let mut entries = Vec::new();
    let dir = fs::read_dir(output_dir).map_err(|source| HistoryError::ReadDir {
        path: output_dir.to_path_buf(),
        source,
    })?;

    for entry in dir {
        let Ok(entry) = entry else {
            continue;
        };
        let path = entry.path();
        if !is_png_file(&path) {
            continue;
        }

        let modified = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .ok();
        entries.push(HistoryEntry { path, modified });
    }

    sort_newest_first(&mut entries);
    entries.truncate(limit);
    Ok(entries)
}

fn is_png_file(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
}

fn sort_newest_first(entries: &mut [HistoryEntry]) {
    entries.sort_by(|left, right| {
        right
            .modified
            .cmp(&left.modified)
            .then_with(|| left.path.cmp(&right.path))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};

    #[test]
    fn recent_pngs_lists_png_files_only() {
        let dir = temp_test_dir("pngs-only");
        fs::write(dir.join("a.png"), b"png").unwrap();
        fs::write(dir.join("b.PNG"), b"png").unwrap();
        fs::write(dir.join("c.txt"), b"text").unwrap();
        fs::create_dir(dir.join("nested.png")).unwrap();

        let mut paths = recent_pngs(&dir, 10)
            .unwrap()
            .into_iter()
            .map(|entry| entry.path.file_name().unwrap().to_owned())
            .collect::<Vec<_>>();
        paths.sort();

        assert_eq!(paths, ["a.png", "b.PNG"]);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn recent_pngs_applies_limit() {
        let dir = temp_test_dir("limit");
        for name in ["a.png", "b.png", "c.png"] {
            fs::write(dir.join(name), b"png").unwrap();
        }

        assert_eq!(recent_pngs(&dir, 2).unwrap().len(), 2);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn recent_pngs_rejects_missing_output_dir() {
        let dir = temp_test_dir("missing");
        let missing = dir.join("missing");

        let error = recent_pngs(&missing, 10).unwrap_err().to_string();

        assert!(error.contains("output directory does not exist"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn sort_newest_first_uses_modified_time_then_path() {
        let mut entries = vec![entry("b.png", 2), entry("a.png", 2), entry("c.png", 1)];

        sort_newest_first(&mut entries);

        let paths = entries
            .into_iter()
            .map(|entry| entry.path)
            .collect::<Vec<_>>();
        assert_eq!(
            paths,
            [
                PathBuf::from("a.png"),
                PathBuf::from("b.png"),
                PathBuf::from("c.png")
            ]
        );
    }

    fn entry(path: &str, seconds: u64) -> HistoryEntry {
        HistoryEntry {
            path: PathBuf::from(path),
            modified: Some(UNIX_EPOCH + Duration::from_secs(seconds)),
        }
    }

    fn temp_test_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("shotlite-history-{name}-{unique}"));
        fs::create_dir(&path).unwrap();
        path
    }
}
