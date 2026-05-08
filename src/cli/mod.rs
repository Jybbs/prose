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
//! and diff rendering.

use std::io;
use std::process::ExitCode;

use anstream::AutoStream;
use clap::{ColorChoice, CommandFactory, Parser};
use clap_complete::generate;

mod args;
mod runner;

use args::{report_clap_error, validate_diff_format_combination, Cli, Command};

pub fn run() -> anyhow::Result<ExitCode> {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => return Ok(report_clap_error(err)),
    };
    if let Some(err) = validate_diff_format_combination(&cli) {
        return Ok(report_clap_error(err));
    }
    match cli.command {
        Command::Check(args) => {
            let stdout = stdout_with_color(cli.color);
            Ok(if runner::check_with_io(args, io::stdin(), stdout)? {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            })
        }
        Command::Completions { shell } => {
            generate(shell, &mut Cli::command(), "prose", &mut io::stdout());
            Ok(ExitCode::SUCCESS)
        }
        Command::Format(args) => {
            let stdout = stdout_with_color(cli.color);
            runner::format_with_io(args, io::stdin(), stdout)?;
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn stdout_with_color(choice: ColorChoice) -> AutoStream<io::StdoutLock<'static>> {
    let lock = io::stdout().lock();
    match choice {
        ColorChoice::Always => AutoStream::always(lock),
        ColorChoice::Auto => AutoStream::auto(lock),
        ColorChoice::Never => AutoStream::never(lock),
    }
}
