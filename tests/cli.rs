use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use image::{Rgba, RgbaImage};

fn shotlite() -> Command {
    Command::new(env!("CARGO_BIN_EXE_shotlite"))
}

#[test]
fn redact_command_honors_explicit_output() {
    let dir = temp_test_dir("redact-output");
    let input = dir.join("input.png");
    let output = dir.join("chosen.png");
    write_test_image(&input);

    let result = shotlite()
        .args([
            "redact",
            input.to_str().unwrap(),
            "--rect",
            "1,1,2,2",
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&result.stdout).trim(),
        output.display().to_string()
    );
    assert!(output.exists());
    assert!(!dir.join("input-redacted.png").exists());

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn redact_command_rejects_invalid_rect() {
    let dir = temp_test_dir("bad-rect");
    let input = dir.join("input.png");
    write_test_image(&input);

    let result = shotlite()
        .args(["redact", input.to_str().unwrap(), "--rect", "1,1,0,2"])
        .output()
        .unwrap();

    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr)
            .contains("rect width and height must be greater than zero")
    );

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn config_path_prints_config_location() {
    let result = shotlite().args(["config", "path"]).output().unwrap();

    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );

    let stdout = String::from_utf8_lossy(&result.stdout);
    let path = Path::new(stdout.trim());
    assert_eq!(path.file_name().unwrap(), "config.toml");
    assert!(
        path.components()
            .any(|component| component.as_os_str() == "shotlite")
    );
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
    let path = std::env::temp_dir().join(format!("shotlite-cli-{name}-{unique}"));
    fs::create_dir(&path).unwrap();
    path
}
