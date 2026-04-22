//! Parses the `[tool.prose]` section from `pyproject.toml`.
//!
//! Every rule is independently toggleable. Defaults mirror the
//! enabled-everywhere preset described in the README.

use std::num::NonZeroUsize;

use ruff_python_ast::PythonVersion;
use serde::Deserialize;

/// The `[tool.prose]` section of a user's `pyproject.toml`.
///
/// `None` on a field means the user did not set that key in their
/// `pyproject.toml`; downstream consumers apply their own fallback
/// rather than reading a synthesized default here.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Config {
    pub line_length: Option<NonZeroUsize>,
    pub rules: RuleToggles,
    pub target_version: Option<PythonVersion>,
}

/// Per-rule on/off flags parsed from `[tool.prose.rules]`.
///
/// Every flag defaults to `true`. Users opt a rule out by setting it
/// to `false` in their `pyproject.toml`.
#[derive(Debug, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct RuleToggles {
    pub align_colons: bool,
    pub align_equals: bool,
    pub align_imports: bool,
    pub alphabetize: bool,
    pub match_case_align: bool,
    pub one_per_line_collections: bool,
    pub singleton_rule: bool,
    pub strip_trailing_commas: bool,
}

impl Default for RuleToggles {
    fn default() -> Self {
        Self {
            align_colons: true,
            align_equals: true,
            align_imports: true,
            alphabetize: true,
            match_case_align: true,
            one_per_line_collections: true,
            singleton_rule: true,
            strip_trailing_commas: true,
        }
    }
}
