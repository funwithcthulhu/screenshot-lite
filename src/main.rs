mod capture;
mod cli;
mod clipboard;
mod config;
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
            output_dir,
            clipboard,
        } => {
            let output_dir = capture_output_dir(output_dir)?;
            let capture = capture::capture_full(&output_dir)?;
            maybe_copy(clipboard, &capture.image)?;
            println!("{}", capture.path.display());
        }
        Command::Region {
            rect,
            output_dir,
            clipboard,
        } => {
            let output_dir = capture_output_dir(output_dir)?;
            let capture = capture::capture_region(&output_dir, rect)?;
            maybe_copy(clipboard, &capture.image)?;
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

fn capture_output_dir(output_dir: Option<std::path::PathBuf>) -> Result<std::path::PathBuf> {
    match output_dir {
        Some(output_dir) => Ok(output_dir),
        None => Ok(Config::load()?.output_dir),
    }
}

fn maybe_copy(copy: bool, image: &image::RgbaImage) -> Result<()> {
    if copy {
        clipboard::copy_image(image)?;
    }

    Ok(())
}
