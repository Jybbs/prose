//! Clap-derived argument types and parse-time validation.

use clap::{ColorChoice, Parser, Subcommand};
use clap_complete::Shell;

mod rule_args;
mod server_args;
mod validation;

pub(crate) use rule_args::{CheckArgs, FormatArgs, OutputFormat, RuleFilter};
pub(crate) use server_args::{ServerArgs, Transport};
pub(crate) use validation::{
    normalize_stdin_dash, report_clap_error, validate_diff_format_combination,
};

/// Matrix appended to `prose --help` via `after_long_help`.
const EXIT_CODE_TABLE: &str = "\
Exit codes:
  0    Clean: no diagnostics, no rewrites pending
  1    Format would change: at least one Severity::Format diagnostic
  2    Lint violation: at least one Severity::Lint diagnostic
  3    Parse error: input could not be parsed as Python
  4    Config error: pyproject.toml, --select / --ignore, or arg validation";

#[derive(Debug, Parser)]
#[command(
    about,
    after_long_help = EXIT_CODE_TABLE,
    arg_required_else_help = true,
    name = "prose",
    propagate_version = true,
    version
)]
pub(crate) struct Cli {
    /// When to use colored output.
    #[arg(long, value_enum, default_value_t, global = true, value_name = "WHEN")]
    pub(crate) color: ColorChoice,

    #[command(subcommand)]
    pub(crate) command: Command,

    /// Print extra diagnostic information to stderr.
    #[arg(long, global = true)]
    pub(crate) verbose: bool,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Manage the user-level cache.
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },

    /// Check files for formatting violations without rewriting.
    Check(CheckArgs),

    /// Print a shell-completion script to stdout.
    Completions {
        /// Shell to generate completions for.
        shell: Shell,
    },

    /// Rewrite files to conform to the Prose style.
    Format(FormatArgs),

    /// Run the language server over stdio.
    Server(ServerArgs),
}

#[derive(Debug, Subcommand)]
pub(crate) enum CacheAction {
    /// Clear every cached entry and report the freed bytes.
    Clean,

    /// Evict oldest entries until the configured size cap is met.
    Compact,

    /// Print the cache directory, entry count, byte total, and mtimes.
    Info,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use assert_matches::assert_matches;
    use clap::{CommandFactory, error::ErrorKind};
    use rstest::{fixture, rstest};

    use super::validation::clap_error_status;
    use super::*;
    use crate::cli::exit_status::ExitStatus;
    use crate::rule::RuleId;

    fn check_command(cli: Cli) -> CheckArgs {
        let Command::Check(args) = cli.command else {
            panic!("expected Check variant");
        };
        args
    }

    fn format_command(cli: Cli) -> FormatArgs {
        let Command::Format(args) = cli.command else {
            panic!("expected Format variant");
        };
        args
    }

    #[fixture]
    fn long_help_table() -> String {
        Cli::command()
            .get_after_long_help()
            .expect("after_long_help is set")
            .to_string()
    }

    fn parse_err(args: &[&str]) -> clap::Error {
        Cli::try_parse_from(args).expect_err("expected parse failure")
    }

    #[rstest]
    fn after_long_help_documents_each_exit_code_label(
        long_help_table: String,
        #[values("Clean", "Config", "Format", "Lint", "Parse")] label: &str,
    ) {
        assert!(
            long_help_table.contains(label),
            "missing label `{label}`: {long_help_table}",
        );
    }

    #[rstest]
    fn after_long_help_documents_each_exit_code_row(
        long_help_table: String,
        #[values(0, 1, 2, 3, 4)] code: u8,
    ) {
        let needle = format!("  {code}    ");
        assert!(
            long_help_table.contains(&needle),
            "missing row for code {code}: {long_help_table}",
        );
    }

    #[test]
    fn check_dash_routes_to_stdin() {
        let mut cli = Cli::try_parse_from(["prose", "check", "-"]).expect("parses");
        assert!(normalize_stdin_dash(&mut cli).is_none());
        let args = check_command(cli);
        assert!(args.paths.is_empty());
        assert!(args.stdin);
    }

    #[test]
    fn check_parses_with_no_input_source() {
        let cli = Cli::try_parse_from(["prose", "check"]).expect("parses");
        let args = check_command(cli);
        assert!(args.paths.is_empty());
        assert!(!args.stdin);
    }

    #[test]
    fn check_parses_with_output_format_github() {
        let cli =
            Cli::try_parse_from(["prose", "check", "--output-format", "github"]).expect("parses");
        let args = check_command(cli);
        assert_matches!(args.output_format, OutputFormat::Github);
    }

    #[test]
    fn check_parses_with_output_format_json() {
        let cli =
            Cli::try_parse_from(["prose", "check", "--output-format", "json"]).expect("parses");
        let args = check_command(cli);
        assert_matches!(args.output_format, OutputFormat::Json);
    }

    #[test]
    fn check_parses_with_paths() {
        let cli = Cli::try_parse_from(["prose", "check", "a.py", "b/"]).expect("parses");
        let args = check_command(cli);
        assert_eq!(args.paths, [PathBuf::from("a.py"), PathBuf::from("b/")]);
        assert!(!args.stdin);
    }

    #[test]
    fn check_parses_with_select_and_ignore_lists() {
        let cli = Cli::try_parse_from([
            "prose",
            "check",
            "--select",
            "align-equals,align-colons",
            "--ignore",
            "alphabetize",
        ])
        .expect("parses");
        let args = check_command(cli);
        assert_eq!(
            args.rules.select,
            [RuleId::from("align-equals"), RuleId::from("align-colons")],
        );
        assert_eq!(args.rules.ignore, [RuleId::from("alphabetize")]);
    }

    #[test]
    fn check_parses_with_stdin() {
        let cli = Cli::try_parse_from(["prose", "check", "--stdin"]).expect("parses");
        let args = check_command(cli);
        assert!(args.paths.is_empty());
        assert!(args.stdin);
    }

    #[test]
    fn check_rejects_dash_alongside_other_paths() {
        let mut cli = Cli::try_parse_from(["prose", "check", "-", "a.py"]).expect("parses");
        let err = normalize_stdin_dash(&mut cli).expect("dash + path surfaces error");
        assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
    }

    #[test]
    fn check_rejects_dash_with_stdin_flag() {
        let err = Cli::try_parse_from(["prose", "check", "-", "--stdin"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
    }

    #[test]
    fn check_rejects_empty_output_format_value() {
        let err = Cli::try_parse_from(["prose", "check", "--output-format="]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidValue);
    }

    #[test]
    fn check_rejects_empty_select_value() {
        let err = Cli::try_parse_from(["prose", "check", "--select="]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidValue);
    }

    #[test]
    fn check_rejects_paths_with_stdin() {
        let err = Cli::try_parse_from(["prose", "check", "--stdin", "a.py"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
    }

    #[test]
    fn check_rejects_unknown_ignore_slug_with_known_list() {
        let err = Cli::try_parse_from(["prose", "check", "--ignore", "not-a-rule"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidValue);
        let rendered = err.to_string();
        assert!(rendered.contains("not-a-rule"));
        assert!(rendered.contains("align-equals"));
    }

    #[test]
    fn check_rejects_unknown_select_slug_with_known_list() {
        let err = Cli::try_parse_from(["prose", "check", "--select", "not-a-rule"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidValue);
        let rendered = err.to_string();
        assert!(rendered.contains("not-a-rule"));
        assert!(rendered.contains("align-equals"));
    }

    #[test]
    fn check_validate_defaults_off_and_parses_when_set() {
        assert!(!check_command(Cli::try_parse_from(["prose", "check"]).expect("parses")).validate);
        let on = Cli::try_parse_from(["prose", "check", "--validate"]).expect("parses");
        assert!(check_command(on).validate);
    }

    #[test]
    fn clap_error_status_routes_argument_conflict_to_config_error() {
        let err = parse_err(&["prose", "format", "--stdin", "a.py"]);
        assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
        assert_eq!(clap_error_status(err.kind()), ExitStatus::ConfigError);
    }

    #[test]
    fn clap_error_status_routes_diff_with_non_text_format_to_config_error() {
        let cli = Cli::try_parse_from(["prose", "format", "--diff", "--output-format", "json"])
            .expect("parses");
        let err = validate_diff_format_combination(&cli).expect("validation surfaces error");
        assert_eq!(err.kind(), ErrorKind::InvalidValue);
        assert_eq!(clap_error_status(err.kind()), ExitStatus::ConfigError);
    }

    #[test]
    fn clap_error_status_routes_help_on_missing_subcommand_to_clean() {
        let err = parse_err(&["prose"]);
        assert_eq!(
            err.kind(),
            ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
        );
        assert_eq!(clap_error_status(err.kind()), ExitStatus::Clean);
    }

    #[test]
    fn clap_error_status_routes_help_to_clean() {
        let err = parse_err(&["prose", "--help"]);
        assert_eq!(err.kind(), ErrorKind::DisplayHelp);
        assert_eq!(clap_error_status(err.kind()), ExitStatus::Clean);
    }

    #[test]
    fn clap_error_status_routes_invalid_value_to_config_error() {
        let err = parse_err(&["prose", "check", "--select", "not-a-rule"]);
        assert_eq!(err.kind(), ErrorKind::InvalidValue);
        assert_eq!(clap_error_status(err.kind()), ExitStatus::ConfigError);
    }

    #[test]
    fn clap_error_status_routes_version_to_clean() {
        let err = parse_err(&["prose", "--version"]);
        assert_eq!(err.kind(), ErrorKind::DisplayVersion);
        assert_eq!(clap_error_status(err.kind()), ExitStatus::Clean);
    }

    #[test]
    fn color_defaults_to_auto() {
        let cli = Cli::try_parse_from(["prose", "check"]).expect("parses");
        assert_eq!(cli.color, ColorChoice::Auto);
    }

    #[test]
    fn color_parses_always_before_subcommand() {
        let cli = Cli::try_parse_from(["prose", "--color", "always", "check"]).expect("parses");
        assert_eq!(cli.color, ColorChoice::Always);
    }

    #[test]
    fn color_parses_never_after_subcommand() {
        let cli = Cli::try_parse_from(["prose", "check", "--color", "never"]).expect("parses");
        assert_eq!(cli.color, ColorChoice::Never);
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

    #[rstest]
    fn completions_parses_each_supported_shell(
        #[values("bash", "elvish", "fish", "powershell", "zsh")] shell: &str,
    ) {
        let cli = Cli::try_parse_from(["prose", "completions", shell]).expect("parses shell");
        assert_matches!(cli.command, Command::Completions { .. });
    }

    #[test]
    fn format_dash_routes_to_stdin() {
        let mut cli = Cli::try_parse_from(["prose", "format", "-"]).expect("parses");
        assert!(normalize_stdin_dash(&mut cli).is_none());
        let args = format_command(cli);
        assert!(args.paths.is_empty());
        assert!(args.stdin);
    }

    #[test]
    fn format_parses_with_diff_and_paths() {
        let cli = Cli::try_parse_from(["prose", "format", "--diff", "a.py"]).expect("parses");
        let args = format_command(cli);
        assert!(args.diff);
        assert_eq!(args.paths, [PathBuf::from("a.py")]);
        assert!(!args.stdin);
    }

    #[test]
    fn format_parses_with_select_and_ignore_lists() {
        let cli = Cli::try_parse_from([
            "prose",
            "format",
            "--select",
            "align-equals",
            "--ignore",
            "alphabetize",
        ])
        .expect("parses");
        let args = format_command(cli);
        assert_eq!(args.rules.select, [RuleId::from("align-equals")]);
        assert_eq!(args.rules.ignore, [RuleId::from("alphabetize")]);
    }

    #[test]
    fn format_parses_with_stdin() {
        let cli = Cli::try_parse_from(["prose", "format", "--stdin"]).expect("parses");
        let args = format_command(cli);
        assert!(args.paths.is_empty());
        assert!(args.stdin);
    }

    #[test]
    fn format_rejects_dash_alongside_other_paths() {
        let mut cli = Cli::try_parse_from(["prose", "format", "-", "a.py"]).expect("parses");
        let err = normalize_stdin_dash(&mut cli).expect("dash + path surfaces error");
        assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
    }

    #[test]
    fn format_rejects_dash_with_stdin_flag() {
        let err = Cli::try_parse_from(["prose", "format", "-", "--stdin"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
    }

    #[test]
    fn format_rejects_diff_with_output_format_github() {
        let cli = Cli::try_parse_from(["prose", "format", "--diff", "--output-format", "github"])
            .expect("parses");
        let err = validate_diff_format_combination(&cli)
            .expect("conflict between --diff and --output-format github fires");
        assert_eq!(err.kind(), ErrorKind::InvalidValue);
    }

    #[test]
    fn format_rejects_diff_with_output_format_json() {
        let cli = Cli::try_parse_from(["prose", "format", "--diff", "--output-format", "json"])
            .expect("parses");
        let err = validate_diff_format_combination(&cli)
            .expect("conflict between --diff and --output-format json fires");
        assert_eq!(err.kind(), ErrorKind::InvalidValue);
    }

    #[test]
    fn format_rejects_paths_with_stdin() {
        let err = Cli::try_parse_from(["prose", "format", "--stdin", "a.py"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
    }

    #[test]
    fn normalize_stdin_dash_is_noop_for_completions() {
        let mut cli = Cli::try_parse_from(["prose", "completions", "bash"]).expect("parses");
        assert!(normalize_stdin_dash(&mut cli).is_none());
    }

    #[test]
    fn normalize_stdin_dash_leaves_dashless_paths_untouched() {
        let mut cli = Cli::try_parse_from(["prose", "check", "a.py", "b/"]).expect("parses");
        assert!(normalize_stdin_dash(&mut cli).is_none());
        let args = check_command(cli);
        assert_eq!(args.paths, [PathBuf::from("a.py"), PathBuf::from("b/")]);
        assert!(!args.stdin);
    }

    #[test]
    fn server_parses_with_default_stdio_transport() {
        let cli = Cli::try_parse_from(["prose", "server"]).expect("parses");
        let Command::Server(args) = cli.command else {
            panic!("expected Server variant");
        };
        assert_matches!(args.transport, Transport::Stdio);
    }
}
