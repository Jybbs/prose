//! Command-line interface.
//!
//! Subcommands: `check` reports violations without modifying files,
//! `format` rewrites in place (or prints a unified diff with
//! `--diff`), `cache` manages the user-level content cache, and
//! `completions` emits a shell-completion script. `check` and
//! `format` accept positional paths, a `-` positional alias for
//! stdin, and a `--stdin` flag, all mutually exclusive.
//!
//! Path mode parallelizes across files via `rayon`. Set
//! `RAYON_NUM_THREADS=1` to force single-threaded execution when
//! debugging a rule. Stdin mode is single-threaded by construction.
//!
//! Layout: `args` houses every clap-derived type and parse-time
//! validation. `cache` houses the `prose cache` subcommand handlers.
//! `runner` houses the pipeline-orchestration helpers that translate
//! parsed args into source loading, emitter dispatch, and diff
//! rendering. `exit_status` carries the matrix every subcommand
//! resolves into.

use std::{
    io::{self, IsTerminal, Write},
    path::PathBuf,
    process::ExitCode,
};

use anstream::{AutoStream, stream::RawStream};
use anyhow::Context;
use clap::{ColorChoice, CommandFactory, Parser};
use clap_complete::generate;

pub(crate) mod args;
mod cache;
pub(crate) mod exit_status;
mod output;
mod runner;

use args::{
    CacheAction, Cli, Command, normalize_stdin_dash, report_clap_error,
    validate_diff_format_combination,
};
use exit_status::ExitStatus;
use output::Presentation;

use crate::config::Config;

pub fn run() -> ExitCode {
    let mut cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => return report_clap_error(err),
    };
    if let Some(err) = normalize_stdin_dash(&mut cli) {
        return report_clap_error(err);
    }
    if let Some(err) = validate_diff_format_combination(&cli) {
        return report_clap_error(err);
    }
    // The server owns stdin and stdout end to end, so it dispatches
    // before the shared stdout lock below, which its writer thread would
    // otherwise deadlock against.
    if let Command::Server(args) = cli.command {
        return finalize(crate::server::run(args)).into();
    }
    let present = Presentation {
        quiet: command_quiet(&cli.command),
        stdout_tty: io::stdout().is_terminal(),
    };
    let stdout = with_color(io::stdout().lock(), cli.color);
    let stderr = with_color(io::stderr(), cli.color);
    let verbose = cli.verbose;
    let result = match cli.command {
        Command::Cache { action } => match action {
            CacheAction::Clean => cache::clean(stdout),
            CacheAction::Compact => cache::compact(stdout),
            CacheAction::Info => cache::info(stdout),
        },
        Command::Check(args) => {
            runner::check_with_io(args, verbose, &present, io::stdin(), stdout, stderr)
        }
        Command::Completions { shell } => {
            generate(shell, &mut Cli::command(), "prose", &mut io::stdout());
            Ok(ExitStatus::Clean)
        }
        Command::Format(args) => {
            runner::format_with_io(args, verbose, &present, io::stdin(), stdout, stderr)
        }
        Command::Server(_) => unreachable!("Server dispatched before the stdout lock"),
    };
    finalize(result).into()
}

fn command_quiet(command: &Command) -> bool {
    match command {
        Command::Check(args) => args.quiet,
        Command::Format(args) => args.quiet,
        Command::Cache { .. } | Command::Completions { .. } | Command::Server(_) => false,
    }
}

fn finalize(result: anyhow::Result<ExitStatus>) -> ExitStatus {
    match result {
        Ok(status) => status,
        Err(err) if is_broken_pipe(&err) => ExitStatus::Clean,
        Err(err) => {
            log_error_chain(&err);
            ExitStatus::ConfigError
        }
    }
}

fn is_broken_pipe(err: &anyhow::Error) -> bool {
    err.downcast_ref::<io::Error>()
        .is_some_and(|e| e.kind() == io::ErrorKind::BrokenPipe)
}

/// Loads the cwd's own config, returning the cwd beside it.
fn load_config_or_status() -> Result<(PathBuf, Config), ExitStatus> {
    let fail = |e: anyhow::Error| {
        log_error_chain(&e);
        ExitStatus::ConfigError
    };
    let cwd = std::env::current_dir()
        .context("reading current working directory")
        .map_err(fail)?;
    let config = Config::load(&cwd)
        .context("loading [tool.prose] config")
        .map_err(fail)?;
    Ok((cwd, config))
}

fn log_error_chain(err: &anyhow::Error) {
    let mut stderr = io::stderr().lock();
    for cause in err.chain() {
        let _ = writeln!(stderr, "error: {cause}");
    }
}

fn with_color<S: RawStream>(raw: S, choice: ColorChoice) -> AutoStream<S> {
    match choice {
        ColorChoice::Always => AutoStream::always(raw),
        ColorChoice::Auto => AutoStream::auto(raw),
        ColorChoice::Never => AutoStream::never(raw),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finalize_clears_broken_pipe_to_clean() {
        let err = anyhow::Error::from(io::Error::new(io::ErrorKind::BrokenPipe, "x"));
        assert_eq!(finalize(Err(err)), ExitStatus::Clean);
    }

    #[test]
    fn finalize_returns_config_error_for_other_errors() {
        let err = anyhow::Error::msg("simulated");
        assert_eq!(finalize(Err(err)), ExitStatus::ConfigError);
    }

    #[test]
    fn finalize_returns_input_status_on_ok() {
        assert_eq!(finalize(Ok(ExitStatus::Clean)), ExitStatus::Clean);
        assert_eq!(
            finalize(Ok(ExitStatus::FormatChange)),
            ExitStatus::FormatChange,
        );
    }

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

    #[test]
    fn log_error_chain_walks_each_cause() {
        let err = anyhow::Error::msg("root").context("ctx");
        log_error_chain(&err);
    }
}
