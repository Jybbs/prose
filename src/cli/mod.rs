//! Command-line interface.
//!
//! Exposes three subcommands: `check` reports violations without
//! modifying files, `format` rewrites in place (or prints a unified
//! diff with `--diff`), and `completions` emits a shell-completion
//! script. `check` and `format` accept positional paths and a
//! `--stdin` flag for pipeline use, mutually exclusive via clap's
//! `conflicts_with`.
//!
//! Path mode parallelizes across files via `rayon`. Set
//! `RAYON_NUM_THREADS=1` to force single-threaded execution when
//! debugging a rule. Stdin mode is single-threaded by construction.
//!
//! Layout: `args` houses every clap-derived type and parse-time
//! validation. `runner` houses the pipeline-orchestration helpers
//! that translate parsed args into source loading, emitter dispatch,
//! and diff rendering. `exit_status` carries the matrix every
//! subcommand resolves into.

use std::io::{self, Write};
use std::process::ExitCode;

use anstream::AutoStream;
use clap::{ColorChoice, CommandFactory, Parser};
use clap_complete::generate;

mod args;
mod exit_status;
mod runner;

use args::{report_clap_error, validate_diff_format_combination, Cli, Command};
use exit_status::ExitStatus;

pub(super) fn log_error_chain(err: &anyhow::Error) {
    let mut stderr = io::stderr().lock();
    for cause in err.chain() {
        let _ = writeln!(stderr, "error: {cause}");
    }
}

pub fn run() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => return report_clap_error(err),
    };
    if let Some(err) = validate_diff_format_combination(&cli) {
        return report_clap_error(err);
    }
    let stdout = stdout_with_color(cli.color);
    let result = match cli.command {
        Command::Check(args) => runner::check_with_io(args, io::stdin(), stdout),
        Command::Completions { shell } => {
            generate(shell, &mut Cli::command(), "prose", &mut io::stdout());
            Ok(ExitStatus::Clean)
        }
        Command::Format(args) => runner::format_with_io(args, io::stdin(), stdout),
    };
    match result {
        Ok(status) => status.into(),
        Err(err) if is_broken_pipe(&err) => ExitCode::SUCCESS,
        Err(err) => {
            log_error_chain(&err);
            ExitStatus::ConfigError.into()
        }
    }
}

fn is_broken_pipe(err: &anyhow::Error) -> bool {
    err.downcast_ref::<io::Error>()
        .is_some_and(|e| e.kind() == io::ErrorKind::BrokenPipe)
}

fn stdout_with_color(choice: ColorChoice) -> AutoStream<io::StdoutLock<'static>> {
    let lock = io::stdout().lock();
    match choice {
        ColorChoice::Always => AutoStream::always(lock),
        ColorChoice::Auto => AutoStream::auto(lock),
        ColorChoice::Never => AutoStream::never(lock),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_broken_pipe_detects_io_error_in_chain() {
        let err = anyhow::Error::from(io::Error::new(io::ErrorKind::BrokenPipe, "x"));
        assert!(is_broken_pipe(&err));
    }

    #[test]
    fn is_broken_pipe_returns_false_for_other_io_errors() {
        let err = anyhow::Error::from(io::Error::other("x"));
        assert!(!is_broken_pipe(&err));
    }

    #[test]
    fn is_broken_pipe_unwraps_through_anyhow_context() {
        let err = anyhow::Error::from(io::Error::new(io::ErrorKind::BrokenPipe, "x"))
            .context("writing stdout");
        assert!(is_broken_pipe(&err));
    }
}
