//! Command-line interface.
//!
//! Subcommands: `check` reports violations without modifying files,
//! `format` rewrites in place (or prints a unified diff with
//! `--diff`), `cache` manages the user-level content cache,
//! `completions` emits a shell-completion script, `schema` prints
//! the configuration's JSON Schema, and `rules` lists the
//! registered rules in pipeline order. `check` and `format` accept
//! positional paths, a `-` positional alias for stdin, and a
//! `--stdin` flag, all mutually exclusive.
//!
//! Path mode parallelizes across files via `rayon`. Set
//! `RAYON_NUM_THREADS=1` to force single-threaded execution when
//! debugging a rule. Stdin mode is single-threaded by construction.
//!
//! Layout: `args` houses every clap-derived type and parse-time
//! validation. `cache` houses the `prose cache` subcommand handlers.
//! `emit` houses the diagnostic emitters behind each output format.
//! `rules` houses the `prose rules` listing. `runner` houses the
//! pipeline-orchestration helpers that translate parsed args into
//! source loading, emitter dispatch, and diff rendering. `schema`
//! houses the `prose schema` emission. `output` houses the
//! human-readable run summary and its palette. `exit_status`
//! carries the matrix every subcommand resolves into.

use std::{
    io::{self, IsTerminal, Write},
    process::ExitCode,
};

use anstream::{AutoStream, stream::RawStream};
use anyhow::Context;
use clap::{ColorChoice, CommandFactory, Parser};
use clap_complete::generate;

pub(crate) mod args;
mod cache;
pub mod emit;
pub(crate) mod exit_status;
mod output;
mod rules;
mod runner;
mod schema;

use args::{
    CacheAction, Cli, Command, normalize_stdin_dash, report_clap_error,
    validate_diff_format_combination, validate_stdin_filename,
};
use exit_status::ExitStatus;
use output::Presentation;

use crate::config::{Config, NoticeDedup};

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
    if let Some(err) = validate_stdin_filename(&cli) {
        return report_clap_error(err);
    }
    // The server owns stdin and stdout end to end, so it dispatches
    // before the shared stdout lock below, which its writer thread would
    // otherwise deadlock against.
    if let Command::Server(args) = cli.command {
        return finalize(crate::server::run(args)).into();
    }
    let raw_stdout = io::stdout().lock();
    let stdout_tty = raw_stdout.is_terminal();
    let (mut stdout, color) = stream_for(cli.color, raw_stdout);
    let (stderr, stderr_color) = stream_for(cli.color, io::stderr());
    let present = Presentation {
        color,
        quiet: cli.run_args().is_some_and(|args| args.quiet),
        stderr_color,
        stdout_tty,
    };
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
            let mut script = Vec::new();
            generate(shell, &mut Cli::command(), "prose", &mut script);
            stdout
                .write_all(&script)
                .and_then(|()| stdout.flush())
                .context("writing stdout")
                .map(|()| ExitStatus::Clean)
        }
        Command::Format(args) => {
            runner::format_with_io(args, verbose, &present, io::stdin(), stdout, stderr)
        }
        Command::Rules(args) => rules::list(&args, stdout),
        Command::Schema => schema::print(stdout),
        Command::Server(_) => unreachable!("Server dispatched before the stdout lock"),
    };
    finalize(result).into()
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

/// Loads the config governing the current working directory, the base
/// for stdin input and the run's cache settings. Routes notices through
/// the run's `dedup` so the run warns each key once even when the
/// per-file walk reloads the same config.
fn load_config_or_status(dedup: &NoticeDedup) -> Result<Config, ExitStatus> {
    let fail = |e: anyhow::Error| {
        log_error_chain(&e);
        ExitStatus::ConfigError
    };
    let cwd = std::env::current_dir()
        .context("reading current working directory")
        .map_err(fail)?;
    Config::load_deduped(&cwd, dedup)
        .context("loading [tool.prose] config")
        .map_err(fail)
}

fn log_error_chain(err: &anyhow::Error) {
    let mut stderr = io::stderr().lock();
    for cause in err.chain() {
        let _ = writeln!(stderr, "error: {cause}");
    }
}

/// The color decision `choice` resolves to for `raw`, taken before the
/// stream is wrapped so every writer reads one answer.
fn color_for<S: RawStream>(choice: ColorChoice, raw: &S) -> bool {
    match choice {
        ColorChoice::Always => true,
        ColorChoice::Auto => AutoStream::choice(raw) != anstream::ColorChoice::Never,
        ColorChoice::Never => false,
    }
}

/// `raw` wrapped for the color decision `choice` resolves to on it,
/// beside that decision. A color run keeps the translation a legacy
/// Windows console needs, and a plain run passes its bytes through,
/// because every writer branches on the same decision and emits no
/// escape for the stream to scan for.
fn stream_for<S: RawStream>(choice: ColorChoice, raw: S) -> (AutoStream<S>, bool) {
    let color = color_for(choice, &raw);
    let stream = if color {
        AutoStream::always(raw)
    } else {
        AutoStream::always_ansi(raw)
    };
    (stream, color)
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
