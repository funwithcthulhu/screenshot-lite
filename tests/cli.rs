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
fn redact_command_writes_new_file_and_leaves_input_unchanged() {
    let dir = temp_test_dir("redact-output");
    let input = dir.join("input.png");
    let output = dir.join("chosen.png");
    write_test_image(&input);
    let original = fs::read(&input).unwrap();

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
    assert_eq!(fs::read(&input).unwrap(), original);
    let output_image = image::open(&output).unwrap();
    assert_eq!(output_image.width(), 4);
    assert_eq!(output_image.height(), 3);

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn redact_command_rejects_zero_width_rect() {
    assert_redact_fails_with("1,1,0,2", "rect width and height must be greater than zero");
}

#[test]
fn redact_command_rejects_zero_height_rect() {
    assert_redact_fails_with("1,1,2,0", "rect width and height must be greater than zero");
}

#[test]
fn redact_command_rejects_negative_coordinates() {
    assert_redact_fails_with("-1,0,1,1", "rect is outside image bounds (4x3)");
    assert_redact_fails_with("0,-1,1,1", "rect is outside image bounds (4x3)");
}

#[test]
fn redact_command_rejects_rect_outside_image_bounds() {
    assert_redact_fails_with("3,2,2,1", "rect is outside image bounds (4x3)");
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

fn assert_redact_fails_with(rect: &str, expected: &str) {
    let dir = temp_test_dir("bad-rect");
    let input = dir.join("input.png");
    write_test_image(&input);
    let original = fs::read(&input).unwrap();

    let result = shotlite()
        .arg("redact")
        .arg(input.to_str().unwrap())
        .arg(format!("--rect={rect}"))
        .output()
        .unwrap();

    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr).contains(expected),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(fs::read(&input).unwrap(), original);
    assert!(!dir.join("input-redacted.png").exists());

    fs::remove_dir_all(dir).unwrap();
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
