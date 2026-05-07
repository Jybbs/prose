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

use crate::config::{AlignmentConfig, CollectionLayoutConfig, Config, ToggleOnly};
use crate::pipeline::Pipeline;
use crate::rules::align_colons::AlignColons;
use crate::rules::align_equals::AlignEquals;
use crate::rules::align_imports::AlignImports;
use crate::rules::alphabetize::Alphabetize;
use crate::rules::collection_layout::CollectionLayout;
use crate::rules::match_case_align::MatchCaseAlign;
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
/// bring it into conformance. An empty `Vec<Edit>` means the rule has
/// nothing to say, and the pipeline skips the reparse for that rule.
///
/// Rules must be `Send + Sync` so that the pipeline can run across
/// files in parallel without moving the rule list per worker.
pub(crate) trait Rule: Send + Sync {
    /// Computes the edit list this rule would apply to `source`.
    /// Edits must not overlap after sorting, an invariant the
    /// pipeline's applicator debug-asserts.
    fn apply(&self, source: &Source) -> Vec<Edit>;

    /// Stable, kebab-case identifier matching the rule's
    /// `[tool.prose.rules]` key. Surfaces in `--select`,
    /// `# prose: ignore`, and diagnostic output.
    fn id(&self) -> RuleId;
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

/// Generates [`KNOWN_IDS`], [`RuleConfigs`], [`Pipeline::for_rule`],
/// and [`Pipeline::with_defaults`] from a registry table. Each row
/// pairs the rule's `[tool.prose.rules]` field name with its config
/// sub-table type and rule struct. The kebab-case slug is derived
/// from the type identifier via `ruff_macros::kebab_case!`.
macro_rules! register_rules {
    ($($field:ident: $config:ty => $ty:ident),* $(,)?) => {
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

            /// Builds a pipeline registering every rule enabled in
            /// `config`. Rules whose `enabled` flag is `false` are
            /// silently skipped.
            pub fn with_defaults(config: &Config) -> Self {
                let mut rules: Vec<Box<dyn Rule>> = Vec::new();
                $(
                    if config.rules.$field.enabled {
                        rules.push(Box::new($ty::from_config(config)));
                    }
                )*
                Self::from_rules(rules)
            }
        }
    };
}

register_rules! {
    collection_layout:     CollectionLayoutConfig => CollectionLayout,
    alphabetize:           ToggleOnly             => Alphabetize,
    strip_trailing_commas: ToggleOnly             => StripTrailingCommas,
    match_case_align:      AlignmentConfig        => MatchCaseAlign,
    align_imports:         AlignmentConfig        => AlignImports,
    align_colons:          AlignmentConfig        => AlignColons,
    align_equals:          AlignmentConfig        => AlignEquals,
    singleton_rule:        ToggleOnly             => SingletonRule,
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
