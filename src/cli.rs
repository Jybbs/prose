//! Command-line interface.
//!
//! Exposes two subcommands: `check` reports violations without modifying files,
//! `format` rewrites files in place (or prints a diff with `--diff`).

use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(name = "prose", version, about)]
pub enum Cli {
    /// Check files for formatting violations without rewriting.
    Check { paths: Vec<PathBuf> },

    /// Rewrite files to conform to the prose style.
    Format {
        /// Show a unified diff instead of writing changes.
        #[arg(long)]
        diff: bool,

        paths: Vec<PathBuf>,
    },
}

pub fn run() -> anyhow::Result<()> {
    let subcommand = match Cli::parse() {
        Cli::Check { .. } => "check",
        Cli::Format { .. } => "format",
    };
    eprintln!("prose {subcommand}: not yet implemented");
    Ok(())
}
