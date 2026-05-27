use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::{history::HistoryAction, redact::Rect};

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
        preview: bool,
        #[arg(long)]
        edit: bool,
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
        preview: bool,
        #[arg(long)]
        edit: bool,
        #[arg(long)]
        clipboard: bool,
    },
    Edit {
        file: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
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
    History {
        #[arg(short, long, default_value_t = 20)]
        limit: usize,
        #[arg(long, value_name = "INDEX", conflicts_with = "reveal")]
        open: Option<usize>,
        #[arg(long, value_name = "INDEX", conflicts_with = "open")]
        reveal: Option<usize>,
    },
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    Path,
    Dir,
    OutputDir { path: Option<PathBuf> },
    Show,
    Open,
    Validate,
    Reset,
    Set { key: ConfigKey, value: PathBuf },
}

#[derive(Clone, Debug, clap::ValueEnum)]
pub enum ConfigKey {
    OutputDir,
}

pub fn history_action(open: Option<usize>, reveal: Option<usize>) -> Option<HistoryAction> {
    match (open, reveal) {
        (Some(index), None) => Some(HistoryAction::Open(index)),
        (None, Some(index)) => Some(HistoryAction::Reveal(index)),
        _ => None,
    }
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
                preview,
                edit,
                ..
            } => {
                assert_eq!(output, Some(PathBuf::from(r".\shots\screen.png")));
                assert!(open);
                assert!(!reveal);
                assert!(!preview);
                assert!(!edit);
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
                preview,
                edit,
                ..
            } => {
                assert_eq!(output_dir, Some(PathBuf::from(r".\shots")));
                assert!(!open);
                assert!(reveal);
                assert!(!preview);
                assert!(!edit);
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn full_preview_parses_without_changing_output_selection() {
        let cli = Cli::try_parse_from([
            "shotlite",
            "full",
            "--output",
            r".\shots\screen.png",
            "--preview",
        ])
        .unwrap();

        match cli.command {
            Command::Full {
                output, preview, ..
            } => {
                assert_eq!(output, Some(PathBuf::from(r".\shots\screen.png")));
                assert!(preview);
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn full_edit_parses_without_changing_output_selection() {
        let cli = Cli::try_parse_from([
            "shotlite",
            "full",
            "--output",
            r".\shots\screen.png",
            "--edit",
        ])
        .unwrap();

        match cli.command {
            Command::Full { output, edit, .. } => {
                assert_eq!(output, Some(PathBuf::from(r".\shots\screen.png")));
                assert!(edit);
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn config_open_parses() {
        let cli = Cli::try_parse_from(["shotlite", "config", "open"]).unwrap();

        match cli.command {
            Command::Config {
                command: ConfigCommand::Open,
            } => {}
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn config_extra_commands_parse() {
        for command in ["dir", "validate", "reset"] {
            Cli::try_parse_from(["shotlite", "config", command]).unwrap();
        }
    }

    #[test]
    fn config_output_dir_shows_or_sets_output_dir() {
        let show = Cli::try_parse_from(["shotlite", "config", "output-dir"]).unwrap();
        match show.command {
            Command::Config {
                command: ConfigCommand::OutputDir { path },
            } => assert_eq!(path, None),
            other => panic!("unexpected command: {other:?}"),
        }

        let set = Cli::try_parse_from(["shotlite", "config", "output-dir", r".\shots"]).unwrap();
        match set.command {
            Command::Config {
                command: ConfigCommand::OutputDir { path },
            } => assert_eq!(path, Some(PathBuf::from(r".\shots"))),
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn history_uses_default_limit() {
        let cli = Cli::try_parse_from(["shotlite", "history"]).unwrap();

        match cli.command {
            Command::History {
                limit,
                open,
                reveal,
            } => {
                assert_eq!(limit, 20);
                assert_eq!(history_action(open, reveal), None);
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn history_accepts_explicit_limit() {
        let cli = Cli::try_parse_from(["shotlite", "history", "--limit", "5"]).unwrap();

        match cli.command {
            Command::History { limit, .. } => assert_eq!(limit, 5),
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn history_open_selects_index() {
        let cli = Cli::try_parse_from(["shotlite", "history", "--open", "2"]).unwrap();

        match cli.command {
            Command::History { open, reveal, .. } => {
                assert_eq!(history_action(open, reveal), Some(HistoryAction::Open(2)));
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn history_reveal_selects_index() {
        let cli = Cli::try_parse_from(["shotlite", "history", "--reveal", "3"]).unwrap();

        match cli.command {
            Command::History { open, reveal, .. } => {
                assert_eq!(history_action(open, reveal), Some(HistoryAction::Reveal(3)));
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn history_open_and_reveal_conflict() {
        assert!(
            Cli::try_parse_from(["shotlite", "history", "--open", "1", "--reveal", "1"]).is_err()
        );
    }

    #[test]
    fn readme_shotlite_examples_parse() {
        let examples = [
            ["shotlite", "full"].as_slice(),
            ["shotlite", "full", "--output-dir", r".\shots"].as_slice(),
            ["shotlite", "full", "--output", r".\shots\screen.png"].as_slice(),
            ["shotlite", "region"].as_slice(),
            ["shotlite", "region", "--rect", "10,20,400,300"].as_slice(),
            ["shotlite", "full", "--clipboard"].as_slice(),
            ["shotlite", "full", "--preview"].as_slice(),
            ["shotlite", "full", "--edit"].as_slice(),
            ["shotlite", "region", "--edit"].as_slice(),
            ["shotlite", "full", "--open"].as_slice(),
            ["shotlite", "full", "--reveal"].as_slice(),
            ["shotlite", "history"].as_slice(),
            ["shotlite", "history", "--limit", "5"].as_slice(),
            ["shotlite", "history", "--open", "1"].as_slice(),
            ["shotlite", "history", "--reveal", "1"].as_slice(),
            ["shotlite", "redact", "input.png", "--rect", "10,20,200,80"].as_slice(),
            [
                "shotlite",
                "highlight",
                "input.png",
                "--rect",
                "10,20,200,80",
            ]
            .as_slice(),
            ["shotlite", "crop", "input.png", "--rect", "10,20,200,80"].as_slice(),
            ["shotlite", "edit", "input.png"].as_slice(),
            ["shotlite", "edit", "input.png", "--output", "edited.png"].as_slice(),
            ["shotlite", "tray"].as_slice(),
            ["shotlite", "config", "path"].as_slice(),
            ["shotlite", "config", "dir"].as_slice(),
            ["shotlite", "config", "open"].as_slice(),
            ["shotlite", "config", "output-dir"].as_slice(),
            [
                "shotlite",
                "config",
                "output-dir",
                r"C:\Users\you\Pictures\Screenshots",
            ]
            .as_slice(),
            ["shotlite", "config", "show"].as_slice(),
            ["shotlite", "config", "validate"].as_slice(),
            ["shotlite", "config", "reset"].as_slice(),
            [
                "shotlite",
                "config",
                "set",
                "output-dir",
                r"C:\Users\you\Pictures\Screenshots",
            ]
            .as_slice(),
        ];

        for example in examples {
            Cli::try_parse_from(example).unwrap_or_else(|error| {
                panic!("README example should parse: {example:?}\n{error}")
            });
        }
    }
}
