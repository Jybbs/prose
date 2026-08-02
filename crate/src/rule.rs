//! Rule abstraction, identifier types, and the registry that ties
//! concrete rule structs to the pipeline orchestrator.
//!
//! Each concrete rule lives under `crate::rules`. The [`Rule`] trait
//! and the [`RuleId`] newtype defined here are the canonical handles.
//! The `register_rules!` macro emits [`KNOWN_IDS`], [`RuleConfigs`],
//! [`Pipeline::for_rule`], [`Pipeline::with_defaults`], and
//! [`Pipeline::with_filters`] from a registry table.

use std::{borrow::Cow, fmt, str::FromStr};

use ruff_diagnostics::Edit;
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};
use serde_json::Map;
use thiserror::Error;

use crate::{
    config::{
        AlignmentConfig, AlphabetizeConfig, BandConstantsConfig, BareImportsConfig,
        CallLayoutConfig, CollectionLayoutConfig, Config, ImportLayoutConfig,
        MiscasedConstantsConfig, ModernizeAnnotationsConfig, PruneInertImportsConfig,
        ReassignedConstantsConfig, SignatureLayoutConfig, SingleUseVariablesConfig, ToggleOnly,
        rule_schema,
    },
    diagnostics::Diagnostic,
    pipeline::Pipeline,
    rules::{
        align_colons::AlignColons, align_comparisons::AlignComparisons, align_equals::AlignEquals,
        align_imports::AlignImports, align_match_case::AlignMatchCase, alphabetize::Alphabetize,
        band_constants::BandConstants, bare_imports::BareImports, blank_lines::BlankLines,
        call_layout::CallLayout, collection_layout::CollectionLayout,
        docstring_expand::DocstringExpand, docstring_frame::DocstringFrame,
        docstring_wrap::DocstringWrap, group_imports::GroupImports, import_layout::ImportLayout,
        line_overflow::LineOverflow, miscased_constants::MiscasedConstants,
        modernize_annotations::ModernizeAnnotations, prune_inert_imports::PruneInertImports,
        reassigned_constants::ReassignedConstants, shed_parentheses::ShedParentheses,
        signature_annotations::SignatureAnnotations, signature_layout::SignatureLayout,
        single_use_variables::SingleUseVariables, step_narration::StepNarration,
        strip_align_padding::StripAlignPadding, strip_none_return::StripNoneReturn,
        strip_trailing_commas::StripTrailingCommas, unsorted_positionals::UnsortedPositionals,
    },
    source::Source,
};

/// Returned when a string fails to match any registered rule slug.
/// Carries the offending input so callers can surface it verbatim.
#[derive(Debug, Error)]
#[error("unknown rule id `{0}`")]
pub struct ParseRuleIdError(String);

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

#[cfg(test)]
impl From<&'static str> for RuleId {
    fn from(slug: &'static str) -> Self {
        Self(slug)
    }
}

impl FromStr for RuleId {
    type Err = ParseRuleIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        slug_index(s)
            .map(|i| KNOWN_IDS[i])
            .ok_or_else(|| ParseRuleIdError(s.to_owned()))
    }
}

impl Serialize for RuleId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.0)
    }
}

/// The slugs whose output the rule named `slug` reads, empty for a rule
/// that depends on nothing seated ahead of it and for an unknown slug.
pub(crate) fn dependencies_of(slug: &str) -> &'static [&'static str] {
    slug_index(slug).map_or(&[], |i| PIPELINE_DEPENDENCIES[i])
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

/// Returns `true` when `earlier` is registered before `later`, and
/// `false` when either is absent from the registry.
const fn precedes(earlier: &str, later: &str) -> bool {
    match (slug_index(earlier), slug_index(later)) {
        (Some(a), Some(b)) => a < b,
        _ => false,
    }
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

/// The registry index of `slug`, or `None` when it is absent.
const fn slug_index(slug: &str) -> Option<usize> {
    let mut i = 0;
    while i < KNOWN_IDS.len() {
        if slug_bytes_equal(KNOWN_IDS[i].as_str().as_bytes(), slug.as_bytes()) {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Generates [`KNOWN_IDS`], [`RuleConfigs`] with its bool-or-table
/// `JsonSchema` impl, [`message_for_id`], [`Pipeline::for_rule`],
/// [`Pipeline::with_defaults`], and [`Pipeline::with_filters`] from a
/// registry table. Each row leads with the rule's kebab-case slug,
/// then its `[tool.prose.rules]` field name, config sub-table type,
/// rule struct, the slugs whose output it reads, and its one-line
/// imperative. The slug is the single source consumed by
/// `RuleId::from_str`, the `[tool.prose.rules.<slug>]` section name,
/// the `# prose: ignore[<slug>]` directive, and `--select` / `--ignore`.
///
/// Row order is pipeline order.
///
/// The macro asserts each slug's kebab shape and cross-row uniqueness
/// at compile time, holds every dependency to a rule seated earlier,
/// and emits a `pub(crate) const SLUG: RuleId` on each rule type so
/// `id()` collapses to `Self::SLUG`.
macro_rules! register_rules {
    ($($slug:literal: $field:ident: $config:ty => $ty:ident
        => [$($after:literal),*] => $msg:literal),* $(,)?) => {
        pub(crate) const KNOWN_IDS: &[RuleId] = &[
            $(RuleId($slug)),*
        ];

        /// Each rule's dependency slugs, indexed alongside [`KNOWN_IDS`].
        const PIPELINE_DEPENDENCIES: &[&[&str]] = &[$(&[$($after),*]),*];

        // Asserts each declared dependency names a rule seated earlier.
        $($(const _: () = assert!(
            precedes($after, $slug),
            concat!("`", $after, "` must be registered before `", $slug, "`"),
        );)*)*

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

        impl JsonSchema for RuleConfigs {
            fn schema_name() -> Cow<'static, str> {
                Cow::Borrowed("RuleConfigs")
            }

            fn json_schema(generator: &mut SchemaGenerator) -> Schema {
                let properties = Map::from_iter([$(
                    ($slug.to_owned(), rule_schema::<$config>(generator).to_value()),
                )*]);
                json_schema!({
                    "type": "object",
                    "description": "Per-rule configuration under `[tool.prose.rules]`.",
                    "properties": properties,
                })
            }
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
            let mut i = 0;
            while i < KNOWN_IDS.len() {
                let mut j = i + 1;
                while j < KNOWN_IDS.len() {
                    assert!(
                        !slug_bytes_equal(
                            KNOWN_IDS[i].as_str().as_bytes(),
                            KNOWN_IDS[j].as_str().as_bytes(),
                        ),
                        "duplicate rule slug in register_rules!",
                    );
                    j += 1;
                }
                i += 1;
            }
        };

        /// Default backing for [`Rule::message`]. Matches each
        /// registered slug to its registry-supplied imperative.
        pub(crate) fn message_for_id(id: RuleId) -> &'static str {
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
                Some(Self::from_rules(vec![rule]).targeting(config.target_version))
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
                Self::from_rules(rules).targeting(config.target_version)
            }
        }
    };
}

register_rules! {
    "prune-inert-imports":   prune_inert_imports:   PruneInertImportsConfig    => PruneInertImports    => [] => "prune an import binding nothing references or a repeat of one already bound",
    "strip-none-return":     strip_none_return:     ToggleOnly                 => StripNoneReturn      => [] => "drop a redundant `-> None` return annotation",
    "modernize-annotations": modernize_annotations: ModernizeAnnotationsConfig => ModernizeAnnotations => [] => "modernize a legacy `typing` annotation to its builtin or PEP 604 form",
    "strip-trailing-commas": strip_trailing_commas: ToggleOnly                 => StripTrailingCommas  => [] => "strip trailing comma",
    "shed-parentheses":      shed_parentheses:      ToggleOnly                 => ShedParentheses      => [] => "shed a redundant grouping parenthesis pair",
    "docstring-frame":       docstring_frame:       ToggleOnly                 => DocstringFrame       => [] => "canonicalize docstring quotes and frame the opener and closer on their own lines",
    "docstring-expand":      docstring_expand:      ToggleOnly                 => DocstringExpand      => ["docstring-frame"] => "expand single-line docstring to multi-line form",
    "group-imports":         group_imports:         ToggleOnly                 => GroupImports         => [] => "group imports into bare, external, and local sections",
    "collection-layout":     collection_layout:     CollectionLayoutConfig     => CollectionLayout     => [] => "lay out collection literal against the line budget",
    "call-layout":           call_layout:           CallLayoutConfig           => CallLayout           => ["collection-layout"] => "explode call arguments to one keyword per line",
    "signature-layout":      signature_layout:      SignatureLayoutConfig      => SignatureLayout      => [] => "normalize function signature to one-line or one-per-line shape",
    "align-match-case":      align_match_case:      AlignmentConfig            => AlignMatchCase       => [] => "align match-case colons",
    "import-layout":         import_layout:         ImportLayoutConfig         => ImportLayout         => ["group-imports"] => "lay out the import block one module per line with its members gathered behind it",
    "alphabetize":           alphabetize:           AlphabetizeConfig          => Alphabetize          => ["collection-layout", "call-layout", "signature-layout", "import-layout"] => "alphabetize this group",
    "band-constants":        band_constants:        BandConstantsConfig        => BandConstants        => ["alphabetize"] => "band module constants into leading and trailing bands",
    "blank-lines":           blank_lines:           ToggleOnly                 => BlankLines           => ["group-imports", "alphabetize"] => "normalize blank-line spacing",
    "align-imports":         align_imports:         AlignmentConfig            => AlignImports         => ["alphabetize", "blank-lines", "import-layout"] => "align consecutive `import`s",
    "align-colons":          align_colons:          AlignmentConfig            => AlignColons          => [] => "align consecutive `:` separators",
    "docstring-wrap":        docstring_wrap:        ToggleOnly                 => DocstringWrap        => ["docstring-frame", "docstring-expand", "align-colons"] => "wrap docstring prose to the configured budget",
    "align-equals":          align_equals:          AlignmentConfig            => AlignEquals          => ["collection-layout", "alphabetize", "align-colons"] => "align consecutive `=` operators",
    "align-comparisons":     align_comparisons:     AlignmentConfig            => AlignComparisons     => [] => "align consecutive comparison operators",
    "strip-align-padding":   strip_align_padding:   ToggleOnly                 => StripAlignPadding    => ["align-match-case", "align-imports", "align-colons", "align-equals", "align-comparisons"] => "drop padding that lines up with nothing",
    "bare-imports":          bare_imports:          BareImportsConfig          => BareImports          => [] => "Flag a bare import a `from` import could replace",
    "miscased-constants":    miscased_constants:    MiscasedConstantsConfig    => MiscasedConstants    => [] => "Module constant is not SCREAMING_CASE. Rename it to the SCREAMING_CASE form",
    "reassigned-constants":  reassigned_constants:  ReassignedConstantsConfig  => ReassignedConstants  => [] => "SCREAMING_CASE name is reassigned despite its constant casing. Rename it lowercase or keep it write-once",
    "step-narration":        step_narration:        ToggleOnly                 => StepNarration        => [] => "Numbered-step comment found. Consider extracting each step as a named function",
    "single-use-variables":  single_use_variables:  SingleUseVariablesConfig   => SingleUseVariables   => [] => "Binding is assigned and used once. Consider inlining",
    "unsorted-positionals":  unsorted_positionals:  ToggleOnly                 => UnsortedPositionals  => [] => "Positional run is out of alphabetical order. Reordering rebinds every positional call site, so apply it by hand where every caller binds by keyword",
    "signature-annotations": signature_annotations: ToggleOnly                 => SignatureAnnotations => [] => "Flag a missing parameter or return type annotation",
    "line-overflow":         line_overflow:         ToggleOnly                 => LineOverflow         => ["strip-align-padding"] => "Flag a line over its length budget that no reshape can bring within",
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn dependencies_of_returns_empty_for_a_rule_without_predecessors(
        #[values("prune-inert-imports", "collection-layout", "not-a-rule")] slug: &str,
    ) {
        assert!(dependencies_of(slug).is_empty());
    }

    #[test]
    fn dependencies_of_returns_the_declared_predecessors() {
        assert_eq!(
            dependencies_of("align-equals"),
            ["collection-layout", "alphabetize", "align-colons"],
        );
    }

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

    #[rstest]
    #[case("collection-layout", "align-equals", true)]
    #[case("align-equals", "collection-layout", false)]
    #[case("align-equals", "not-a-rule", false)]
    #[case("not-a-rule", "align-equals", false)]
    fn precedes_orders_registered_slugs(
        #[case] earlier: &str,
        #[case] later: &str,
        #[case] expected: bool,
    ) {
        assert_eq!(precedes(earlier, later), expected);
    }

    #[test]
    fn rule_id_display_and_debug_print_bare_slug() {
        let id = RuleId("align-equals");
        assert_eq!(format!("{id}"), "align-equals");
        assert_eq!(format!("{id:?}"), "align-equals");
    }

    #[rstest]
    fn rule_id_from_str_rejects_an_unregistered_slug(
        #[values("not-a-rule", "PROSE-align-equals")] input: &str,
    ) {
        let err = input
            .parse::<RuleId>()
            .expect_err("unregistered slug is rejected");
        assert_eq!(err.0, input);
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
