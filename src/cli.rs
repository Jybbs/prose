//! Command-line interface.
//!
//! Exposes two subcommands: `check` reports violations without
//! modifying files, `format` rewrites in place (or prints a unified
//! diff with `--diff`). Both accept positional paths and a `--stdin`
//! flag for pipeline use. `--stdin` and path arguments are mutually
//! exclusive via clap's `conflicts_with`.

use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    about,
    arg_required_else_help = true,
    name = "prose",
    propagate_version = true,
    version
)]
pub enum Cli {
    /// Check files for formatting violations without rewriting.
    Check(CheckArgs),

    /// Rewrite files to conform to the prose style.
    Format(FormatArgs),
}

#[derive(Debug, clap::Args)]
pub struct CheckArgs {
    /// One or more files or directories to check. Omit when using `--stdin`.
    #[arg(conflicts_with = "stdin", value_name = "PATH")]
    pub paths: Vec<PathBuf>,

    /// Read source from stdin instead of the filesystem.
    #[arg(long)]
    pub stdin: bool,
}

#[derive(Debug, clap::Args)]
pub struct FormatArgs {
    /// Show a unified diff instead of writing changes.
    #[arg(long)]
    pub diff: bool,

    /// One or more files or directories to format. Omit when using `--stdin`.
    #[arg(conflicts_with = "stdin", value_name = "PATH")]
    pub paths: Vec<PathBuf>,

    /// Read source from stdin instead of the filesystem.
    #[arg(long)]
    pub stdin: bool,
}

pub fn run() -> anyhow::Result<()> {
    match Cli::parse() {
        Cli::Check(args) => check(args),
        Cli::Format(args) => format(args),
    }
}

fn check(_args: CheckArgs) -> anyhow::Result<()> {
    eprintln!("prose check: not yet implemented");
    Ok(())
}

fn format(_args: FormatArgs) -> anyhow::Result<()> {
    eprintln!("prose format: not yet implemented");
    Ok(())
}

#[cfg(test)]
mod tests {
    use clap::{error::ErrorKind, CommandFactory};

    use super::*;

    #[test]
    fn check_parses_with_no_input_source() {
        let cli = Cli::try_parse_from(["prose", "check"]).expect("parses");
        let Cli::Check(args) = cli else {
            panic!("expected Check variant");
        };
        assert!(args.paths.is_empty());
        assert!(!args.stdin);
    }

    #[test]
    fn check_parses_with_paths() {
        let cli = Cli::try_parse_from(["prose", "check", "a.py", "b/"]).expect("parses");
        let Cli::Check(args) = cli else {
            panic!("expected Check variant");
        };
        assert_eq!(args.paths, [PathBuf::from("a.py"), PathBuf::from("b/")]);
        assert!(!args.stdin);
    }

    #[test]
    fn check_parses_with_stdin() {
        let cli = Cli::try_parse_from(["prose", "check", "--stdin"]).expect("parses");
        let Cli::Check(args) = cli else {
            panic!("expected Check variant");
        };
        assert!(args.paths.is_empty());
        assert!(args.stdin);
    }

    #[test]
    fn check_rejects_paths_with_stdin() {
        let err = Cli::try_parse_from(["prose", "check", "--stdin", "a.py"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
    }

    #[test]
    fn command_is_well_formed() {
        Cli::command().debug_assert();
    }

    #[test]
    fn command_version_matches_crate() {
        assert_eq!(Cli::command().get_version(), Some(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn format_parses_with_diff_and_paths() {
        let cli = Cli::try_parse_from(["prose", "format", "--diff", "a.py"]).expect("parses");
        let Cli::Format(args) = cli else {
            panic!("expected Format variant");
        };
        assert!(args.diff);
        assert_eq!(args.paths, [PathBuf::from("a.py")]);
        assert!(!args.stdin);
    }

    #[test]
    fn format_parses_with_stdin() {
        let cli = Cli::try_parse_from(["prose", "format", "--stdin"]).expect("parses");
        let Cli::Format(args) = cli else {
            panic!("expected Format variant");
        };
        assert!(args.paths.is_empty());
        assert!(args.stdin);
    }

    #[test]
    fn format_rejects_paths_with_stdin() {
        let err = Cli::try_parse_from(["prose", "format", "--stdin", "a.py"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
    }
}
