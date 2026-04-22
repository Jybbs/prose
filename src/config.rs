//! Parses the `[tool.prose]` section from `pyproject.toml`.
//!
//! Every rule is independently toggleable. Defaults mirror the
//! enabled-everywhere preset described in the README.

use ruff_python_ast::PythonVersion;
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Config {
    pub line_length: Option<usize>,
    pub target_version: Option<PythonVersion>,
    pub rules: RuleToggles,
}

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
