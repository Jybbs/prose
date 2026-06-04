//! Post-parse normalization and clap-error to exit-status routing.

use std::process::ExitCode;

use clap::{CommandFactory, error::ErrorKind};

use crate::cli::exit_status::ExitStatus;

use super::{Cli, Command};

/// Resolves a `-` positional into stdin mode, surfacing a clap
/// error when `-` appears alongside other paths.
pub(crate) fn normalize_stdin_dash(cli: &mut Cli) -> Option<clap::Error> {
    let (paths, stdin) = match &mut cli.command {
        Command::Cache { .. } | Command::Completions { .. } | Command::Server(_) => return None,
        Command::Check(args) => (&mut args.paths, &mut args.stdin),
        Command::Format(args) => (&mut args.paths, &mut args.stdin),
    };
    if !paths.iter().any(|p| p.as_os_str() == "-") {
        return None;
    }
    if paths.len() > 1 {
        return Some(Cli::command().error(
            ErrorKind::ArgumentConflict,
            "`-` cannot appear alongside other paths",
        ));
    }
    paths.clear();
    *stdin = true;
    None
}

/// Prints a clap parse failure and resolves the exit code.
pub(crate) fn report_clap_error(err: clap::Error) -> ExitCode {
    let _ = err.print();
    clap_error_status(err.kind()).into()
}

/// Returns a config error when `--diff` pairs with a non-text
/// `--output-format`. Routed through [`report_clap_error`] so the
/// exit code lands at 4 alongside other config errors.
pub(crate) fn validate_diff_format_combination(cli: &Cli) -> Option<clap::Error> {
    let Command::Format(args) = &cli.command else {
        return None;
    };
    (args.diff && !args.output_format.is_text()).then(|| {
        Cli::command().error(
            ErrorKind::InvalidValue,
            "`--diff` requires `--output-format text`",
        )
    })
}

/// Help / version land at `Clean`, every other clap error at `ConfigError`.
pub(super) fn clap_error_status(kind: ErrorKind) -> ExitStatus {
    match kind {
        ErrorKind::DisplayHelp
        | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
        | ErrorKind::DisplayVersion => ExitStatus::Clean,
        _ => ExitStatus::ConfigError,
    }
}
