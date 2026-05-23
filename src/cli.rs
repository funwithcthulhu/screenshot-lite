use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::redact::Rect;

#[derive(Debug, Parser)]
#[command(name = "shotlite")]
#[command(about = "Small local screenshot utility")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Full {
        #[arg(long, conflicts_with = "output_dir")]
        output: Option<PathBuf>,
        #[arg(long)]
        output_dir: Option<PathBuf>,
        #[arg(long)]
        open: bool,
        #[arg(long)]
        reveal: bool,
        #[arg(long)]
        clipboard: bool,
    },
    Region {
        #[arg(long)]
        rect: Option<Rect>,
        #[arg(long, conflicts_with = "output_dir")]
        output: Option<PathBuf>,
        #[arg(long)]
        output_dir: Option<PathBuf>,
        #[arg(long)]
        open: bool,
        #[arg(long)]
        reveal: bool,
        #[arg(long)]
        clipboard: bool,
    },
    Edit {
        file: PathBuf,
    },
    Tray,
    Redact {
        file: PathBuf,
        #[arg(long)]
        rect: Rect,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    Highlight {
        file: PathBuf,
        #[arg(long)]
        rect: Rect,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    Crop {
        file: PathBuf,
        #[arg(long)]
        rect: Rect,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    Path,
    Show,
    Set { key: ConfigKey, value: PathBuf },
}

#[derive(Clone, Debug, clap::ValueEnum)]
pub enum ConfigKey {
    OutputDir,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn full_open_keeps_explicit_output_file() {
        let cli = Cli::try_parse_from([
            "shotlite",
            "full",
            "--output",
            r".\shots\screen.png",
            "--open",
        ])
        .unwrap();

        match cli.command {
            Command::Full {
                output,
                open,
                reveal,
                ..
            } => {
                assert_eq!(output, Some(PathBuf::from(r".\shots\screen.png")));
                assert!(open);
                assert!(!reveal);
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn full_reveal_keeps_explicit_output_dir() {
        let cli = Cli::try_parse_from(["shotlite", "full", "--output-dir", r".\shots", "--reveal"])
            .unwrap();

        match cli.command {
            Command::Full {
                output_dir,
                open,
                reveal,
                ..
            } => {
                assert_eq!(output_dir, Some(PathBuf::from(r".\shots")));
                assert!(!open);
                assert!(reveal);
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }
}
