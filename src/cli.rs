//! Command-line interface.
//!
//! Exposes two subcommands: `check` reports violations without
//! modifying files, `format` rewrites in place (or prints a unified
//! diff with `--diff`). Both accept positional paths and a `--stdin`
//! flag for pipeline use. `--stdin` and path arguments are mutually
//! exclusive via clap's `conflicts_with`.
//!
//! Path mode parallelizes across files via `rayon`. Set
//! `RAYON_NUM_THREADS=1` to force single-threaded execution when
//! debugging a rule. Stdin mode is single-threaded by construction.

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Context;
use clap::Parser;
use rayon::iter::{ParallelBridge, ParallelIterator};

use crate::config::Config;
use crate::pipeline::Pipeline;
use crate::source::Source;
use crate::walker;

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

pub fn run() -> anyhow::Result<ExitCode> {
    match Cli::parse() {
        Cli::Check(args) => Ok(if check_with_io(args, io::stdin())? {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        }),
        Cli::Format(args) => {
            format_with_io(args, io::stdin(), io::stdout().lock())?;
            Ok(ExitCode::SUCCESS)
        }
    }
}

/// Returns `true` when nothing would change, `false` when at least one
/// file or the stdin input would be rewritten.
fn check_with_io<R: Read>(args: CheckArgs, stdin: R) -> anyhow::Result<bool> {
    let pipeline = load_pipeline()?;

    if args.stdin {
        let (_, changed) = run_pipeline_on_stdin(stdin, &pipeline)?;
        return Ok(!changed);
    }

    let any_changed = walker::walk(&args.paths)
        .par_bridge()
        .map(|entry| -> anyhow::Result<bool> {
            let path = entry.context("walking input paths")?;
            let (_, changed) = process_path(&path, &pipeline)?;
            Ok(changed)
        })
        .try_reduce(|| false, |a, b| Ok(a || b))?;

    Ok(!any_changed)
}

fn format_with_io<R: Read, W: Write>(
    args: FormatArgs,
    stdin: R,
    mut stdout: W,
) -> anyhow::Result<()> {
    let pipeline = load_pipeline()?;

    if args.stdin {
        let (formatted, changed) = run_pipeline_on_stdin(stdin, &pipeline)?;
        if args.diff {
            if changed {
                write_diff(&mut stdout, "<stdin>")?;
            }
        } else {
            stdout
                .write_all(formatted.text().as_bytes())
                .context("writing stdout")?;
        }
        return Ok(());
    }

    let changed: Vec<PathBuf> = walker::walk(&args.paths)
        .par_bridge()
        .map(|entry| -> anyhow::Result<Option<PathBuf>> {
            let path = entry.context("walking input paths")?;
            let (formatted, changed) = process_path(&path, &pipeline)?;
            if changed && !args.diff {
                fs_err::write(&path, formatted.text())?;
            }
            Ok((changed && args.diff).then_some(path))
        })
        .filter_map(Result::transpose)
        .collect::<anyhow::Result<Vec<_>>>()?;

    for path in changed {
        write_diff(&mut stdout, path.display())?;
    }
    Ok(())
}

fn load_pipeline() -> anyhow::Result<Pipeline> {
    let cwd = std::env::current_dir().context("reading current working directory")?;
    let config = Config::load(&cwd).context("loading [tool.prose] config")?;
    Ok(Pipeline::with_defaults(&config))
}

/// Loads a single Python file from disk and runs the pipeline over it.
fn process_path(path: &Path, pipeline: &Pipeline) -> anyhow::Result<(Source, bool)> {
    let source =
        Source::from_path(path).with_context(|| format!("loading `{}`", path.display()))?;
    Ok(pipeline.run(source)?)
}

/// Reads a Python source from `stdin`, parses it, and runs the pipeline.
fn run_pipeline_on_stdin<R: Read>(stdin: R, pipeline: &Pipeline) -> anyhow::Result<(Source, bool)> {
    let text = io::read_to_string(stdin).context("reading stdin")?;
    let source: Source = text.parse().context("parsing stdin as Python")?;
    Ok(pipeline.run(source)?)
}

/// Placeholder diff emitter for `--diff` mode.
///
/// Prints a two-line banner for now. A real unified diff lands when
/// the first rule PR gives the `--diff` path something non-empty to
/// print.
fn write_diff<W: Write>(writer: &mut W, name: impl std::fmt::Display) -> anyhow::Result<()> {
    writeln!(writer, "--- {name}\n+++ {name}").context("writing diff")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use clap::{error::ErrorKind, CommandFactory};
    use tempfile::TempDir;

    use super::*;

    fn check_args(paths: Vec<PathBuf>, stdin: bool) -> CheckArgs {
        CheckArgs { paths, stdin }
    }

    fn format_args(paths: Vec<PathBuf>, stdin: bool, diff: bool) -> FormatArgs {
        FormatArgs { diff, paths, stdin }
    }

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
    fn check_paths_reports_clean_when_pipeline_is_empty() {
        let tmp = TempDir::new().expect("tempdir");
        let file = tmp.path().join("a.py");
        std::fs::write(&file, "x = 1\n").expect("writes");

        let clean = check_with_io(
            check_args(vec![tmp.path().to_path_buf()], false),
            Cursor::new(Vec::<u8>::new()),
        )
        .expect("runs successfully");

        assert!(clean);
    }

    #[test]
    fn check_rejects_paths_with_stdin() {
        let err = Cli::try_parse_from(["prose", "check", "--stdin", "a.py"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
    }

    #[test]
    fn check_stdin_empty_pipeline_reports_clean() {
        let stdin = Cursor::new(b"x = 1\n".to_vec());
        let clean = check_with_io(check_args(Vec::new(), true), stdin).expect("runs successfully");
        assert!(clean);
    }

    #[test]
    fn check_stdin_surfaces_parse_error() {
        let stdin = Cursor::new(b"def foo(".to_vec());
        let err = check_with_io(check_args(Vec::new(), true), stdin).unwrap_err();
        assert!(err.to_string().contains("stdin"));
    }

    #[test]
    fn command_is_well_formed() {
        Cli::command().debug_assert();
    }

    #[test]
    fn command_version_matches_crate() {
        assert_eq!(
            Cli::command().get_version(),
            Some(env!("CARGO_PKG_VERSION"))
        );
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
    fn format_paths_does_not_rewrite_when_pipeline_is_empty() {
        let tmp = TempDir::new().expect("tempdir");
        let file = tmp.path().join("a.py");
        std::fs::write(&file, "x = 1\n").expect("writes");

        let mut stdout = Vec::new();
        format_with_io(
            format_args(vec![tmp.path().to_path_buf()], false, false),
            Cursor::new(Vec::<u8>::new()),
            &mut stdout,
        )
        .expect("runs successfully");

        let contents = std::fs::read_to_string(&file).expect("reads");
        assert_eq!(contents, "x = 1\n");
        assert!(stdout.is_empty());
    }

    #[test]
    fn format_rejects_paths_with_stdin() {
        let err = Cli::try_parse_from(["prose", "format", "--stdin", "a.py"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
    }

    #[test]
    fn format_stdin_prints_input_verbatim_when_pipeline_is_empty() {
        let stdin = Cursor::new(b"x = 1\n".to_vec());
        let mut stdout = Vec::new();
        format_with_io(format_args(Vec::new(), true, false), stdin, &mut stdout)
            .expect("runs successfully");
        assert_eq!(stdout, b"x = 1\n");
    }

    #[test]
    fn format_stdin_with_diff_emits_nothing_when_pipeline_is_empty() {
        let stdin = Cursor::new(b"x = 1\n".to_vec());
        let mut stdout = Vec::new();
        format_with_io(format_args(Vec::new(), true, true), stdin, &mut stdout)
            .expect("runs successfully");
        assert!(stdout.is_empty());
    }
}
