mod capture;
mod cli;
mod clipboard;
mod config;
mod file_action;
mod paths;
mod redact;

use anyhow::{Context, Result, bail};
use clap::Parser;

use crate::{
    cli::{Cli, Command, ConfigCommand, ConfigKey},
    config::Config,
};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Full {
            output,
            output_dir,
            open,
            reveal,
            clipboard,
        } => {
            let output = capture_output(output, output_dir)?;
            let capture = capture::capture_full_to(output)?;
            maybe_copy(clipboard, &capture.image)?;
            after_capture(&capture.path, open, reveal)?;
            println!("{}", capture.path.display());
        }
        Command::Region {
            rect,
            output,
            output_dir,
            open,
            reveal,
            clipboard,
        } => {
            let output = capture_output(output, output_dir)?;
            let capture = capture::capture_region_to(output, rect)?;
            maybe_copy(clipboard, &capture.image)?;
            after_capture(&capture.path, open, reveal)?;
            println!("{}", capture.path.display());
        }
        Command::Edit { file } => {
            bail!(
                "edit is not implemented yet for {}; use `shotlite redact` for pixel redaction",
                file.display()
            );
        }
        Command::Redact { file, rect, output } => {
            let output = redact::redact_file(&file, rect, output)
                .with_context(|| format!("failed to redact {}", file.display()))?;
            println!("{}", output.display());
        }
        Command::Highlight { file, rect, output } => {
            let output = redact::highlight_file(&file, rect, output)
                .with_context(|| format!("failed to highlight {}", file.display()))?;
            println!("{}", output.display());
        }
        Command::Crop { file, rect, output } => {
            let output = redact::crop_file(&file, rect, output)
                .with_context(|| format!("failed to crop {}", file.display()))?;
            println!("{}", output.display());
        }
        Command::Config { command } => match command {
            ConfigCommand::Path => {
                let path = paths::config_file()
                    .context("could not determine the user config directory")?;
                println!("{}", path.display());
            }
            ConfigCommand::Show => {
                let config = Config::load()?;
                print!("{}", config.to_toml()?);
            }
            ConfigCommand::Set { key, value } => {
                let mut config = Config::load()?;
                match key {
                    ConfigKey::OutputDir => config.output_dir = value,
                }
                let path = config.save()?;
                println!("{}", path.display());
            }
        },
    }

    Ok(())
}

fn capture_output(
    output: Option<std::path::PathBuf>,
    output_dir: Option<std::path::PathBuf>,
) -> Result<capture::CaptureOutput> {
    match (output, output_dir) {
        (Some(output), _) => Ok(capture::CaptureOutput::File(output)),
        (None, Some(output_dir)) => Ok(capture::CaptureOutput::Directory(output_dir)),
        (None, None) => Ok(capture::CaptureOutput::Directory(
            Config::load()?.output_dir,
        )),
    }
}

fn after_capture(path: &std::path::Path, open: bool, reveal: bool) -> Result<()> {
    if reveal {
        file_action::reveal(path)?;
    }
    if open {
        file_action::open(path)?;
    }
    Ok(())
}

fn maybe_copy(copy: bool, image: &image::RgbaImage) -> Result<()> {
    if copy {
        clipboard::copy_image(image)?;
    }

    Ok(())
}
