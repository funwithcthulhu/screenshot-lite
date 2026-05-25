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
fn redact_command_uses_default_output_name() {
    let dir = temp_test_dir("redact-default-output");
    let input = dir.join("input.png");
    let output = dir.join("input-redacted.png");
    write_test_image(&input);
    let original = fs::read(&input).unwrap();

    let result = shotlite()
        .args(["redact", input.to_str().unwrap(), "--rect", "1,1,2,2"])
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
    assert_eq!(fs::read(&input).unwrap(), original);

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
fn highlight_command_writes_new_file() {
    let dir = temp_test_dir("highlight");
    let input = dir.join("input.png");
    let output = dir.join("highlight.png");
    write_test_image(&input);
    let original = fs::read(&input).unwrap();

    let result = shotlite()
        .args([
            "highlight",
            input.to_str().unwrap(),
            "--rect",
            "1,1,2,1",
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
    assert_eq!(fs::read(&input).unwrap(), original);
    assert!(output.exists());

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn crop_command_writes_cropped_file() {
    let dir = temp_test_dir("crop");
    let input = dir.join("input.png");
    let output = dir.join("crop.png");
    write_test_image(&input);

    let result = shotlite()
        .args([
            "crop",
            input.to_str().unwrap(),
            "--rect",
            "1,1,2,1",
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
    let image = image::open(output).unwrap();
    assert_eq!(image.width(), 2);
    assert_eq!(image.height(), 1);

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

#[test]
fn edit_command_rejects_missing_input_image() {
    let dir = temp_test_dir("edit-missing");
    let input = dir.join("missing.png");

    let result = shotlite()
        .args(["edit", input.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("failed to edit"));
    assert!(stderr.contains("failed to open"));
    assert!(stderr.contains("missing.png"));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn edit_command_rejects_non_image_input() {
    let dir = temp_test_dir("edit-non-image");
    let input = dir.join("input.txt");
    fs::write(&input, "not an image").unwrap();
    let original = fs::read(&input).unwrap();

    let result = shotlite()
        .args(["edit", input.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("failed to edit"));
    assert!(stderr.contains("failed to open"));
    assert!(stderr.contains("input.txt"));
    assert_eq!(fs::read(&input).unwrap(), original);

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn history_lists_pngs_from_configured_output_dir() {
    let dir = temp_test_dir("history-list");
    let output_dir = dir.join("shots");
    fs::create_dir(&output_dir).unwrap();
    fs::write(output_dir.join("first.png"), b"png").unwrap();
    fs::write(output_dir.join("second.PNG"), b"png").unwrap();
    fs::write(output_dir.join("notes.txt"), b"text").unwrap();
    write_config(&dir, &output_dir);

    let result = shotlite()
        .env("SHOTLITE_CONFIG_DIR", &dir)
        .args(["history"])
        .output()
        .unwrap();

    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(stdout.contains("first.png"));
    assert!(stdout.contains("second.PNG"));
    assert!(!stdout.contains("notes.txt"));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn history_limit_restricts_listed_entries() {
    let dir = temp_test_dir("history-limit");
    let output_dir = dir.join("shots");
    fs::create_dir(&output_dir).unwrap();
    fs::write(output_dir.join("first.png"), b"png").unwrap();
    fs::write(output_dir.join("second.png"), b"png").unwrap();
    write_config(&dir, &output_dir);

    let result = shotlite()
        .env("SHOTLITE_CONFIG_DIR", &dir)
        .args(["history", "--limit", "1"])
        .output()
        .unwrap();

    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let lines = String::from_utf8_lossy(&result.stdout)
        .lines()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].ends_with(".png"));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn history_open_zero_reports_index_error_without_file_action() {
    let dir = temp_test_dir("history-open-zero");
    let output_dir = dir.join("shots");
    fs::create_dir(&output_dir).unwrap();
    fs::write(output_dir.join("first.png"), b"png").unwrap();
    write_config(&dir, &output_dir);

    let result = shotlite()
        .env("SHOTLITE_CONFIG_DIR", &dir)
        .args(["history", "--open", "0"])
        .output()
        .unwrap();

    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("history index must be greater than zero"));
    assert!(!stderr.contains("failed to open"));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn history_reveal_out_of_range_reports_valid_count_without_file_action() {
    let dir = temp_test_dir("history-reveal-out-of-range");
    let output_dir = dir.join("shots");
    fs::create_dir(&output_dir).unwrap();
    fs::write(output_dir.join("first.png"), b"png").unwrap();
    write_config(&dir, &output_dir);

    let result = shotlite()
        .env("SHOTLITE_CONFIG_DIR", &dir)
        .args(["history", "--reveal", "3"])
        .output()
        .unwrap();

    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("history index 3 is not available; found 1 screenshot(s)"));
    assert!(!stderr.contains("failed to reveal"));

    fs::remove_dir_all(dir).unwrap();
}

fn write_test_image(path: &Path) {
    let mut image = RgbaImage::from_pixel(4, 3, Rgba([255, 255, 255, 255]));
    image.put_pixel(0, 0, Rgba([10, 20, 30, 255]));
    image.save(path).unwrap();
}

fn write_config(config_dir: &Path, output_dir: &Path) {
    let path = config_dir.join("shotlite").join("config.toml");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        path,
        format!("output_dir = {:?}\n", output_dir.to_string_lossy()),
    )
    .unwrap();
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
