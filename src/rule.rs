//! Rule abstraction, identifier types, and the registry that ties
//! concrete rule structs to the pipeline orchestrator.
//!
//! Each concrete rule lives under `crate::rules`. The [`Rule`] trait
//! and the [`RuleId`] newtype defined here are the canonical handles.
//! The `register_rules!` macro emits [`KNOWN_IDS`], [`RuleConfigs`],
//! [`Pipeline::for_rule`], [`Pipeline::with_defaults`], and
//! [`Pipeline::with_filters`] from a registry table.

use std::{fmt, str::FromStr};

use ruff_diagnostics::Edit;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    config::{
        AlignmentConfig, AlphabetizeConfig, BareImportsConfig, CallLayoutConfig,
        CollectionLayoutConfig, Config, ReassignedConstantsConfig, SignatureLayoutConfig,
        SingleUseVariablesConfig, ToggleOnly,
    },
    diagnostics::Diagnostic,
    pipeline::Pipeline,
    rules::{
        align_colons::AlignColons, align_comparisons::AlignComparisons, align_equals::AlignEquals,
        align_imports::AlignImports, align_match_case::AlignMatchCase, alphabetize::Alphabetize,
        bare_imports::BareImports, blank_lines::BlankLines, call_layout::CallLayout,
        collection_layout::CollectionLayout, docstring_expand::DocstringExpand,
        docstring_frame::DocstringFrame, docstring_wrap::DocstringWrap,
        import_layout::ImportLayout, legacy_union_syntax::LegacyUnionSyntax,
        reassigned_constants::ReassignedConstants, signature_annotations::SignatureAnnotations,
        signature_layout::SignatureLayout, single_use_variables::SingleUseVariables,
        step_narration::StepNarration, strip_align_padding::StripAlignPadding,
        strip_none_return::StripNoneReturn, strip_trailing_commas::StripTrailingCommas,
        unused_future_annotations::UnusedFutureAnnotations,
    },
    source::Source,
};

/// Returned when a string fails to match any registered rule slug.
/// Carries the offending input so callers can surface it verbatim.
#[derive(Debug, Error)]
#[error("unknown rule id `{0}`")]
pub struct ParseRuleIdError(pub String);

/// Every rule in Prose implements this trait and nothing more.
///
/// Implementations inspect `source` and return the edits that would
/// bring it into conformance, partitioned into fix groups, the
/// `Severity::Lint` diagnostics they surface without an edit, or both.
/// An empty outer `Vec` from `apply` skips the reparse for that rule.
///
/// Rules must be `Send + Sync` so that the pipeline can run across
/// files in parallel without moving the rule list per worker.
pub(crate) trait Rule: Send + Sync {
    /// Computes the edits this rule would apply to `source`,
    /// partitioned into fix groups. Each inner `Vec` is one fix that
    /// the pipeline maps to a single diagnostic, and the edits across
    /// all groups must not overlap after sorting. The pipeline's
    /// applicator declines an overlapping group rather than splicing it.
    fn apply(&self, _source: &Source) -> Vec<Vec<Edit>> {
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
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
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

impl<'de> Deserialize<'de> for RuleId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
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

impl Serialize for RuleId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.0)
    }
}

/// Returns `true` when `bytes` is a valid kebab-case slug. Non-empty,
/// starts and ends with a lowercase ASCII letter or digit, contains
/// only lowercase ASCII letters, digits, and dashes, and has no `--`
/// substring.
const fn is_valid_slug(bytes: &[u8]) -> bool {
    let mut i = 0;
    let mut prev_was_dash = true;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'-' {
            if prev_was_dash {
                return false;
            }
            prev_was_dash = true;
        } else if b.is_ascii_lowercase() || b.is_ascii_digit() {
            prev_was_dash = false;
        } else {
            return false;
        }
        i += 1;
    }
    !prev_was_dash
}

/// Byte-wise equality on `&[u8]` usable from const contexts.
const fn slug_bytes_equal(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// Generates [`KNOWN_IDS`], [`RuleConfigs`], [`message_for_id`],
/// [`Pipeline::for_rule`], [`Pipeline::with_defaults`], and
/// [`Pipeline::with_filters`] from a registry table. Each row leads
/// with the rule's kebab-case slug, then its `[tool.prose.rules]`
/// field name, config sub-table type, rule struct, and one-line
/// imperative. The slug is the single source consumed by
/// `RuleId::from_str`, the `[tool.prose.rules.<slug>]` section name,
/// the `# prose: ignore[<slug>]` directive, and `--select` / `--ignore`.
///
/// The macro asserts each slug's kebab shape and cross-row uniqueness
/// at compile time, and emits a `pub(crate) const SLUG: RuleId` on
/// each rule type so `id()` collapses to `Self::SLUG`.
macro_rules! register_rules {
    ($($slug:literal: $field:ident: $config:ty => $ty:ident => $msg:literal),* $(,)?) => {
        pub(crate) const KNOWN_IDS: &[RuleId] = &[
            $(RuleId($slug)),*
        ];

        /// Per-rule configuration under `[tool.prose.rules]`.
        ///
        /// Each field accepts a bare bool, where `false` disables the
        /// rule and `true` keeps its defaults, or a sub-table whose
        /// keys carry that rule's knobs. An absent field defaults to
        /// enabled.
        #[derive(Debug, Default, Deserialize, Serialize)]
        #[serde(default, rename_all = "kebab-case")]
        pub struct RuleConfigs {
            $(
                #[serde(deserialize_with = "crate::config::deserialize_rule")]
                pub $field: $config,
            )*
        }

        // Routes a missing-`Default` error to the offending `$config`
        // row instead of the macro-emitted derive site.
        $(const _: fn() -> $config = <$config as Default>::default;)*

        // Exposes each rule's slug as an inherent associated const so
        // the rule's `id()` body collapses to `Self::SLUG`.
        $(
            impl $ty {
                pub(crate) const SLUG: RuleId = RuleId($slug);
            }
        )*

        // Asserts each slug is valid kebab-case at compile time.
        $(const _: () = assert!(is_valid_slug($slug.as_bytes()));)*

        // Asserts cross-row slug uniqueness at compile time.
        const _: () = {
            const SLUGS: &[&str] = &[$($slug),*];
            let mut i = 0;
            while i < SLUGS.len() {
                let mut j = i + 1;
                while j < SLUGS.len() {
                    assert!(
                        !slug_bytes_equal(SLUGS[i].as_bytes(), SLUGS[j].as_bytes()),
                        "duplicate rule slug in register_rules!",
                    );
                    j += 1;
                }
                i += 1;
            }
        };

        /// Default backing for [`Rule::message`]. Matches each
        /// registered slug to its registry-supplied imperative.
        fn message_for_id(id: RuleId) -> &'static str {
            match id.as_str() {
                $($slug => $msg,)*
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
                    $($slug => Box::new($ty::from_config(config)),)*
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
                    let id = RuleId($slug);
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
    "collection-layout":         collection_layout:         CollectionLayoutConfig    => CollectionLayout        => "lay out collection literal against the line budget",
    "alphabetize":               alphabetize:               AlphabetizeConfig         => Alphabetize             => "alphabetize this group",
    "call-layout":               call_layout:               CallLayoutConfig          => CallLayout              => "explode call arguments to one keyword per line",
    "strip-trailing-commas":     strip_trailing_commas:     ToggleOnly                => StripTrailingCommas     => "strip trailing comma",
    "docstring-expand":          docstring_expand:          ToggleOnly                => DocstringExpand         => "expand single-line docstring to multi-line form",
    "docstring-frame":           docstring_frame:           ToggleOnly                => DocstringFrame          => "place docstring opener and closer on their own lines",
    "unused-future-annotations": unused_future_annotations: ToggleOnly                => UnusedFutureAnnotations => "remove unused `from __future__ import annotations`",
    "blank-lines":               blank_lines:               ToggleOnly                => BlankLines              => "normalize blank-line spacing",
    "bare-imports":              bare_imports:              BareImportsConfig         => BareImports             => "flag a bare import a `from` import could replace",
    "align-match-case":          align_match_case:          AlignmentConfig           => AlignMatchCase          => "align match-case colons",
    "strip-none-return":         strip_none_return:         ToggleOnly                => StripNoneReturn         => "drop a redundant `-> None` return annotation",
    "signature-layout":          signature_layout:          SignatureLayoutConfig     => SignatureLayout         => "normalize function signature to one-line or one-per-line shape",
    "import-layout":             import_layout:             ToggleOnly                => ImportLayout            => "split an over-long `from` import into repeated-prefix lines",
    "align-imports":             align_imports:             AlignmentConfig           => AlignImports            => "align consecutive `import`s",
    "align-colons":              align_colons:              AlignmentConfig           => AlignColons             => "align consecutive `:` separators",
    "docstring-wrap":            docstring_wrap:            ToggleOnly                => DocstringWrap           => "wrap docstring prose to the configured budget",
    "align-equals":              align_equals:              AlignmentConfig           => AlignEquals             => "align consecutive `=` operators",
    "align-comparisons":         align_comparisons:         AlignmentConfig           => AlignComparisons        => "align consecutive comparison operators",
    "strip-align-padding":       strip_align_padding:       ToggleOnly                => StripAlignPadding       => "drop padding from a group with no column to align to",
    "reassigned-constants":      reassigned_constants:      ReassignedConstantsConfig => ReassignedConstants     => "SCREAMING_CASE name is reassigned despite its constant casing. Rename it lowercase or keep it write-once",
    "step-narration":            step_narration:            ToggleOnly                => StepNarration           => "numbered-step comment found. Consider extracting each step as a named function",
    "legacy-union-syntax":       legacy_union_syntax:       ToggleOnly                => LegacyUnionSyntax       => "rewrite legacy `Optional`/`Union` to PEP 604 union syntax",
    "single-use-variables":      single_use_variables:      SingleUseVariablesConfig  => SingleUseVariables      => "binding is assigned and used once. Consider inlining",
    "signature-annotations":     signature_annotations:     ToggleOnly                => SignatureAnnotations    => "flag a missing parameter or return type annotation",
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn is_valid_slug_accepts_canonical_kebab_shapes(
        #[values("a", "a-b", "abc123", "single-use-variables")] valid: &str,
    ) {
        assert!(is_valid_slug(valid.as_bytes()));
    }

    #[rstest]
    fn is_valid_slug_rejects_invalid_shapes(
        #[values("", "-foo", "foo-", "a--b", "Foo", "abc!")] invalid: &str,
    ) {
        assert!(!is_valid_slug(invalid.as_bytes()));
    }

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

    #[test]
    fn slug_bytes_equal_matches_only_identical_slices() {
        assert!(slug_bytes_equal(b"foo", b"foo"));
        assert!(!slug_bytes_equal(b"foo", b"food"));
        assert!(!slug_bytes_equal(b"foo", b"bar"));
    }
}
