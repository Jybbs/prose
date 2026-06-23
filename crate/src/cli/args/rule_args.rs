//! `check` / `format` shared args, the rule-filter, and the output
//! format value-enum.

use std::path::PathBuf;

use clap::builder::{PossibleValuesParser, TypedValueParser};

use crate::{pipeline::Pipeline, rule::RuleId};

#[derive(Debug, Default, clap::Args)]
pub(crate) struct CheckArgs {
    /// Bypass the user-level cache for this invocation.
    #[arg(long)]
    pub(crate) no_cache: bool,

    /// Output format for diagnostics.
    #[arg(long, value_enum, default_value_t)]
    pub(crate) output_format: OutputFormat,

    /// Files or directories to check, or `-` to read source from
    /// stdin. Omit when using `--stdin`.
    #[arg(conflicts_with = "stdin", value_name = "PATH")]
    pub(crate) paths: Vec<PathBuf>,

    /// Reduce the summary to a bare count line, dropping the section
    /// anchors and color.
    #[arg(short, long)]
    pub(crate) quiet: bool,

    #[command(flatten)]
    pub(crate) rules: RuleFilter,

    /// Read source from stdin instead of the filesystem. Equivalent
    /// to passing `-` as the sole path.
    #[arg(long)]
    pub(crate) stdin: bool,

    /// Treat stdin as this path, its extension selecting the source
    /// type. A `.ipynb` name reads stdin as a notebook.
    #[arg(long, value_name = "PATH")]
    pub(crate) stdin_filename: Option<PathBuf>,

    /// Confirm each file's would-be rewrite re-parses, surfacing an
    /// unparseable rule output as a failure. Off by default.
    #[arg(long)]
    pub(crate) validate: bool,
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

    /// Files or directories to format, or `-` to read source from
    /// stdin. Omit when using `--stdin`.
    #[arg(conflicts_with = "stdin", value_name = "PATH")]
    pub(crate) paths: Vec<PathBuf>,

    /// Reduce the summary to a bare count line, dropping the section
    /// anchors and color.
    #[arg(short, long)]
    pub(crate) quiet: bool,

    #[command(flatten)]
    pub(crate) rules: RuleFilter,

    /// Read source from stdin instead of the filesystem. Equivalent
    /// to passing `-` as the sole path.
    #[arg(long)]
    pub(crate) stdin: bool,

    /// Treat stdin as this path, its extension selecting the source
    /// type. A `.ipynb` name reads stdin as a notebook.
    #[arg(long, value_name = "PATH")]
    pub(crate) stdin_filename: Option<PathBuf>,
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

/// Returns a value parser that accepts any registered rule slug and
/// produces a [`RuleId`]. Errors render with clap's `[possible
/// values: ...]` suffix listing every known slug.
fn rule_id_parser() -> impl TypedValueParser<Value = RuleId> {
    PossibleValuesParser::new(Pipeline::known_ids().iter().map(RuleId::as_str))
        .try_map(|s| s.parse::<RuleId>())
}
