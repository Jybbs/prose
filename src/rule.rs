//! Rule abstraction, identifier types, and the registry that ties
//! concrete rule structs to the pipeline orchestrator.
//!
//! Each concrete rule lives under `crate::rules`. The [`Rule`] trait
//! and the [`RuleId`] newtype defined here are the canonical handles.
//! The `register_rules!` macro emits [`KNOWN_IDS`], [`RuleConfigs`],
//! [`Pipeline::for_rule`], and [`Pipeline::with_defaults`] from a
//! registry table.

use std::fmt;
use std::str::FromStr;

use ruff_diagnostics::Edit;
use serde::Deserialize;
use thiserror::Error;

use crate::config::{
    AlignmentConfig, BareImportAllowlistConfig, CollectionLayoutConfig, Config,
    LooseConstantsConfig, ToggleOnly,
};
use crate::diagnostics::Diagnostic;
use crate::pipeline::Pipeline;
use crate::rules::align_colons::AlignColons;
use crate::rules::align_equals::AlignEquals;
use crate::rules::align_imports::AlignImports;
use crate::rules::alphabetize::Alphabetize;
use crate::rules::bare_import_allowlist::BareImportAllowlist;
use crate::rules::blank_lines::BlankLines;
use crate::rules::collection_layout::CollectionLayout;
use crate::rules::docstring_wrap::DocstringWrap;
use crate::rules::loose_constants::LooseConstants;
use crate::rules::match_case_align::MatchCaseAlign;
use crate::rules::multi_line_docstrings::MultiLineDocstrings;
use crate::rules::no_single_line_docstrings::NoSingleLineDocstrings;
use crate::rules::no_step_narration::NoStepNarration;
use crate::rules::singleton_rule::SingletonRule;
use crate::rules::strip_trailing_commas::StripTrailingCommas;
use crate::source::Source;

/// Returned when a string fails to match any registered rule slug.
/// Carries the offending input so callers can surface it verbatim.
#[derive(Debug, Error)]
#[error("unknown rule id `{0}`")]
pub struct ParseRuleIdError(pub String);

/// Every rule in Prose implements this trait and nothing more.
///
/// Implementations inspect `source` and return the edits that would
/// bring it into conformance, the `Severity::Lint` diagnostics they
/// surface without an edit, or both. An empty `Vec<Edit>` from
/// `apply` skips the reparse for that rule.
///
/// Rules must be `Send + Sync` so that the pipeline can run across
/// files in parallel without moving the rule list per worker.
pub(crate) trait Rule: Send + Sync {
    /// Computes the edit list this rule would apply to `source`.
    /// Edits must not overlap after sorting, an invariant the
    /// pipeline's applicator debug-asserts.
    fn apply(&self, _source: &Source) -> Vec<Edit> {
        Vec::new()
    }

    /// Stable, kebab-case identifier matching the rule's
    /// `[tool.prose.rules]` key. Surfaces in `--select`,
    /// `# prose: ignore`, and diagnostic output.
    fn id(&self) -> RuleId;

    /// Lint-only side channel emitting `Severity::Lint` diagnostics
    /// the pipeline cannot derive from an edit. The default returns
    /// no diagnostics, so auto-fix rules need not override.
    fn lint(&self, _source: &Source) -> Vec<Diagnostic> {
        Vec::new()
    }

    /// One-line imperative carried as `Diagnostic.message`. Defaults
    /// to the registry-supplied string for `self.id()`.
    fn message(&self) -> &'static str {
        message_for_id(self.id())
    }
}

/// Stable, parseable rule identifier wrapping a kebab-case slug.
/// Returned by [`Rule::id`] and parsed from CLI / pragma input via
/// [`FromStr`]. The canonical handle in `--select` / `--ignore`,
/// `# prose: ignore[...]`, JSON `"rule"` fields, and `github`
/// annotations.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct RuleId(&'static str);

impl RuleId {
    pub const fn as_str(&self) -> &'static str {
        self.0
    }
}

impl fmt::Debug for RuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl fmt::Display for RuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl From<&'static str> for RuleId {
    fn from(slug: &'static str) -> Self {
        Self(slug)
    }
}

impl FromStr for RuleId {
    type Err = ParseRuleIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        KNOWN_IDS
            .iter()
            .copied()
            .find(|id| id.0 == s)
            .ok_or_else(|| ParseRuleIdError(s.to_owned()))
    }
}

/// Generates [`KNOWN_IDS`], [`RuleConfigs`], [`message_for_id`],
/// [`Pipeline::for_rule`], [`Pipeline::with_defaults`], and
/// [`Pipeline::with_filters`] from a registry table. Each row pairs
/// the rule's `[tool.prose.rules]` field name with its config
/// sub-table type, rule struct, and one-line imperative. The
/// kebab-case slug is derived from the type identifier via
/// `ruff_macros::kebab_case!`.
macro_rules! register_rules {
    ($($field:ident: $config:ty => $ty:ident => $msg:literal),* $(,)?) => {
        pub(crate) const KNOWN_IDS: &[RuleId] = &[
            $(RuleId(ruff_macros::kebab_case!($ty))),*
        ];

        /// Per-rule configuration parsed from `[tool.prose.rules.<name>]`.
        ///
        /// Each field is a sub-table whose `enabled` key (defaulting
        /// to `true`) toggles the rule and whose remaining keys carry
        /// that rule's knobs.
        #[derive(Debug, Default, Deserialize)]
        #[serde(default, rename_all = "kebab-case")]
        pub struct RuleConfigs {
            $(pub $field: $config,)*
        }

        // Routes a missing-`Default` error to the offending `$config`
        // row instead of the macro-emitted derive site.
        $(const _: fn() -> $config = <$config as Default>::default;)*

        /// Default backing for [`Rule::message`]. Matches each
        /// registered slug to its registry-supplied imperative.
        pub(crate) fn message_for_id(id: RuleId) -> &'static str {
            match id.as_str() {
                $(ruff_macros::kebab_case!($ty) => $msg,)*
                _ => unreachable!("rule id must be registered"),
            }
        }

        impl Pipeline {
            /// Builds a pipeline registering exactly one rule by name.
            ///
            /// Returns `None` when `name` does not match any registered
            /// rule, see [`Pipeline::known_ids`] for the full list.
            /// Bypasses each rule's `enabled` flag. Snake-case input is
            /// normalized to the canonical kebab form.
            pub fn for_rule(name: &str, config: &Config) -> Option<Self> {
                let rule: Box<dyn Rule> = match name.replace('_', "-").as_str() {
                    $(ruff_macros::kebab_case!($ty) => Box::new($ty::from_config(config)),)*
                    _ => return None,
                };
                Some(Self::from_rules(vec![rule]))
            }

            /// Builds a pipeline from every rule whose `enabled`
            /// flag is set in `config`.
            pub fn with_defaults(config: &Config) -> Self {
                Self::with_filters(config, &[], &[])
            }

            /// Builds a pipeline applying `select` and `ignore`
            /// against `config`'s rule toggles.
            ///
            /// A non-empty `select` replaces the configured-enabled
            /// set, whereas an empty `select` falls back to it.
            /// `ignore` then subtracts from the base, yielding
            /// `select - ignore`.
            pub fn with_filters(
                config: &Config,
                select: &[RuleId],
                ignore: &[RuleId],
            ) -> Self {
                let mut rules: Vec<Box<dyn Rule>> = Vec::new();
                $({
                    let id = RuleId(ruff_macros::kebab_case!($ty));
                    let included = if select.is_empty() {
                        config.rules.$field.enabled
                    } else {
                        select.contains(&id)
                    };
                    if included && !ignore.contains(&id) {
                        rules.push(Box::new($ty::from_config(config)));
                    }
                })*
                Self::from_rules(rules)
            }
        }
    };
}

register_rules! {
    collection_layout:          CollectionLayoutConfig    => CollectionLayout       => "expand collection to one entry per line",
    alphabetize:                ToggleOnly                => Alphabetize            => "alphabetize this group",
    strip_trailing_commas:      ToggleOnly                => StripTrailingCommas    => "strip trailing comma",
    no_single_line_docstrings:  ToggleOnly                => NoSingleLineDocstrings => "expand single-line docstring to multi-line form",
    multi_line_docstrings:      ToggleOnly                => MultiLineDocstrings    => "place docstring opener and closer on their own lines",
    blank_lines:                ToggleOnly                => BlankLines             => "normalize blank-line spacing",
    bare_import_allowlist:      BareImportAllowlistConfig => BareImportAllowlist    => "flag bare import outside allowlist",
    match_case_align:           AlignmentConfig           => MatchCaseAlign         => "align match-case arrows",
    align_imports:              AlignmentConfig           => AlignImports           => "align consecutive `import`s",
    align_colons:               AlignmentConfig           => AlignColons            => "align consecutive `:` separators",
    docstring_wrap:             ToggleOnly                => DocstringWrap          => "wrap docstring prose to the configured budget",
    align_equals:               AlignmentConfig           => AlignEquals            => "align consecutive `=` operators",
    singleton_rule:             ToggleOnly                => SingletonRule          => "drop padding from singleton group",
    loose_constants:            LooseConstantsConfig      => LooseConstants         => "consider moving this module-level constant",
    no_step_narration:          ToggleOnly                => NoStepNarration        => "numbered-step comment found. Consider extracting each step as a named function",
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_id_display_and_debug_print_bare_slug() {
        let id = RuleId("align-equals");
        assert_eq!(format!("{id}"), "align-equals");
        assert_eq!(format!("{id:?}"), "align-equals");
    }

    #[test]
    fn rule_id_from_str_rejects_prose_prefixed_slug() {
        let err = "PROSE-align-equals"
            .parse::<RuleId>()
            .expect_err("prefixed form is not the canonical");
        assert_eq!(err.0, "PROSE-align-equals");
    }

    #[test]
    fn rule_id_from_str_rejects_unknown_slug() {
        let err = "not-a-rule"
            .parse::<RuleId>()
            .expect_err("unknown rejected");
        assert_eq!(err.0, "not-a-rule");
    }

    #[test]
    fn rule_id_round_trips_through_display_and_from_str() {
        for id in KNOWN_IDS {
            let parsed: RuleId = id.to_string().parse().expect("known id parses");
            assert_eq!(parsed, *id);
        }
    }
}
