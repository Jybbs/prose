//! Command-line interface.
//!
//! Exposes two subcommands: `check` reports violations without modifying files,
//! `format` rewrites files in place (or prints a diff with `--diff`).

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "prose", version, about)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check files for formatting violations without rewriting.
    Check {
        paths: Vec<PathBuf>,
    },

    /// Rewrite files to conform to the prose style.
    Format {
        /// Show a unified diff instead of writing changes.
        #[arg(long)]
        diff: bool,

        paths: Vec<PathBuf>,
    },
}

pub fn run() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Command::Check { paths } => {
            let _ = paths;
            eprintln!("prose check: not yet implemented");
            ExitCode::SUCCESS
        }
        Command::Format { diff, paths } => {
            let _ = (diff, paths);
            eprintln!("prose format: not yet implemented");
            ExitCode::SUCCESS
        }
    }
}
