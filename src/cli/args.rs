//! Clap-derived argument types and parse-time validation.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::builder::{PossibleValuesParser, TypedValueParser};
use clap::{error::ErrorKind, ColorChoice, CommandFactory, Parser, Subcommand};
use clap_complete::Shell;

use super::exit_status::ExitStatus;
use crate::pipeline::Pipeline;
use crate::rule::RuleId;

/// Matrix appended to `prose --help` via `after_long_help`.
const EXIT_CODE_TABLE: &str = "\
Exit codes:
  0    Clean: no diagnostics, no rewrites pending
  1    Format would change: at least one Severity::Format diagnostic
  2    Lint violation: at least one Severity::Lint diagnostic
  3    Parse error: input could not be parsed as Python
  4    Config error: pyproject.toml, --select / --ignore, or arg validation";

#[derive(Debug, Default, clap::Args)]
pub(crate) struct CheckArgs {
    /// Bypass the user-level cache for this invocation.
    #[arg(long)]
    pub(crate) no_cache: bool,

    /// Output format for diagnostics.
    #[arg(long, value_enum, default_value_t)]
    pub(crate) output_format: OutputFormat,

    /// One or more files or directories to check. Omit when using `--stdin`.
    #[arg(conflicts_with = "stdin", value_name = "PATH")]
    pub(crate) paths: Vec<PathBuf>,

    #[command(flatten)]
    pub(crate) rules: RuleFilter,

    /// Read source from stdin instead of the filesystem.
    #[arg(long)]
    pub(crate) stdin: bool,
}

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

#[derive(Debug, Default, clap::Args)]
pub(crate) struct FormatArgs {
    /// Show a unified diff instead of writing changes.
    #[arg(long)]
    pub(crate) diff: bool,

    /// Bypass the user-level cache for this invocation.
    #[arg(long)]
    pub(crate) no_cache: bool,

    /// Output format for diagnostics.
    #[arg(long, value_enum, default_value_t)]
    pub(crate) output_format: OutputFormat,

    /// One or more files or directories to format. Omit when using `--stdin`.
    #[arg(conflicts_with = "stdin", value_name = "PATH")]
    pub(crate) paths: Vec<PathBuf>,

    #[command(flatten)]
    pub(crate) rules: RuleFilter,

    /// Read source from stdin instead of the filesystem.
    #[arg(long)]
    pub(crate) stdin: bool,
}

#[derive(Clone, Copy, Debug, Default, clap::ValueEnum)]
pub(crate) enum OutputFormat {
    Github,
    Json,
    Sarif,
    #[default]
    Text,
}

impl OutputFormat {
    pub(crate) fn is_text(self) -> bool {
        matches!(self, Self::Text)
    }
}

/// Subset of registered rules to run, applied as `select - ignore`.
#[derive(Debug, Default, clap::Args)]
pub(crate) struct RuleFilter {
    /// Comma-separated rule slugs to skip, subtracted from
    /// whichever set would otherwise have run.
    #[arg(long, value_delimiter = ',', value_name = "RULES", value_parser = rule_id_parser())]
    pub(crate) ignore: Vec<RuleId>,

    /// Comma-separated rule slugs to run, replacing the
    /// configured-enabled set.
    #[arg(long, value_delimiter = ',', value_name = "RULES", value_parser = rule_id_parser())]
    pub(crate) select: Vec<RuleId>,
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
fn clap_error_status(kind: ErrorKind) -> ExitStatus {
    match kind {
        ErrorKind::DisplayHelp
        | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
        | ErrorKind::DisplayVersion => ExitStatus::Clean,
        _ => ExitStatus::ConfigError,
    }
}

/// Returns a value parser that accepts any registered rule slug and
/// produces a [`RuleId`]. Errors render with clap's `[possible
/// values: ...]` suffix listing every known slug.
fn rule_id_parser() -> impl TypedValueParser<Value = RuleId> {
    PossibleValuesParser::new(Pipeline::known_ids().iter().map(RuleId::as_str))
        .try_map(|s| s.parse::<RuleId>())
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn parse_err(args: &[&str]) -> clap::Error {
        Cli::try_parse_from(args).expect_err("expected parse failure")
    }

    #[test]
    fn after_long_help_documents_the_exit_code_matrix() {
        let table = Cli::command()
            .get_after_long_help()
            .expect("after_long_help is set")
            .to_string();
        for code in 0..=4 {
            let needle = format!("  {code}    ");
            assert!(
                table.contains(&needle),
                "after_long_help missing row for code {code}: {table}",
            );
        }
        for label in ["Clean", "Format", "Lint", "Parse", "Config"] {
            assert!(
                table.contains(label),
                "after_long_help missing label `{label}`: {table}",
            );
        }
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
        assert!(matches!(args.output_format, OutputFormat::Github));
    }

    #[test]
    fn check_parses_with_output_format_json() {
        let cli =
            Cli::try_parse_from(["prose", "check", "--output-format", "json"]).expect("parses");
        let args = check_command(cli);
        assert!(matches!(args.output_format, OutputFormat::Json));
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

    #[test]
    fn completions_parses_each_supported_shell() {
        for shell in ["bash", "elvish", "fish", "powershell", "zsh"] {
            let cli = Cli::try_parse_from(["prose", "completions", shell]).expect("parses shell");
            assert!(matches!(cli.command, Command::Completions { .. }));
        }
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
}
