//! Rule abstraction, identifier types, and the registry that ties
//! concrete rule structs to the pipeline orchestrator.
//!
//! Each concrete rule lives under `crate::rules`. The [`Rule`] trait
//! and the [`RuleId`] newtype defined here are the canonical handles.
//! The `register_rules!` macro emits [`KNOWN_IDS`], [`RuleConfigs`],
//! [`Pipeline::for_rule`], [`Pipeline::with_defaults`], and
//! [`Pipeline::with_filters`] from a registry table.

use std::{
    borrow::Cow,
    collections::BTreeSet,
    fmt::{self, Display},
    str::FromStr,
};

use itertools::Itertools;
use ruff_diagnostics::Edit;
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};
use serde_json::Map;
use thiserror::Error;

use crate::{
    config::{
        AlignmentConfig, AlphabetizeSiblingsConfig, BandConstantsConfig, BareImportsConfig, Config,
        InlinableBindingsConfig, LineOverflowConfig, MiscasedConstantsConfig,
        ModernizeAnnotationsConfig, NormalizeComparisonsConfig, NormalizeLiteralsConfig,
        PreferFstringConfig, PruneInertImportsConfig, ReassignedConstantsConfig, ReflowCallsConfig,
        ReflowCollectionsConfig, ReflowImportsConfig, ReflowSignaturesConfig,
        StackMethodChainsConfig, ToggleOnly, rule_schema,
    },
    diagnostics::Diagnostic,
    pipeline::Pipeline,
    rules::{
        align_colons::AlignColons, align_comments::AlignComments,
        align_comparisons::AlignComparisons, align_equals::AlignEquals,
        align_imports::AlignImports, align_match_case::AlignMatchCase,
        alphabetize_siblings::AlphabetizeSiblings, band_constants::BandConstants,
        bare_imports::BareImports, expand_docstrings::ExpandDocstrings,
        frame_docstrings::FrameDocstrings, group_imports::GroupImports,
        inlinable_bindings::InlinableBindings, line_overflow::LineOverflow,
        miscased_constants::MiscasedConstants, modernize_annotations::ModernizeAnnotations,
        normalize_comment_spacing::NormalizeCommentSpacing,
        normalize_comparisons::NormalizeComparisons, normalize_literals::NormalizeLiterals,
        prefer_fstring::PreferFstring, prune_inert_imports::PruneInertImports,
        reassigned_constants::ReassignedConstants, reflow_calls::ReflowCalls,
        reflow_collections::ReflowCollections, reflow_imports::ReflowImports,
        reflow_parentheses::ReflowParentheses, reflow_signatures::ReflowSignatures,
        restated_types::RestatedTypes, shed_backslash_continuations::ShedBackslashContinuations,
        shed_redundant_base::ShedRedundantBase, shed_super_args::ShedSuperArgs,
        signature_annotations::SignatureAnnotations,
        simplify_comprehensions::SimplifyComprehensions, space_statements::SpaceStatements,
        stack_adjacent_strings::StackAdjacentStrings, stack_method_chains::StackMethodChains,
        step_narration::StepNarration, strip_none_return::StripNoneReturn,
        strip_stranded_padding::StripStrandedPadding, strip_trailing_commas::StripTrailingCommas,
        unsorted_positionals::UnsortedPositionals, wrap_docstrings::WrapDocstrings,
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
pub(crate) trait Rule: fmt::Debug + Send + Sync {
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
    /// to the `MESSAGE` const on the rule registered under `self.id()`.
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

/// The slugs the rule named `slug` must run behind, empty for a rule
/// that settles wherever it sits and for an unknown slug.
pub fn dependencies_of(slug: &str) -> &'static [&'static str] {
    slug_index(slug).map_or(&[], |i| PIPELINE_DEPENDENCIES[i])
}

/// Renders `rules` as a comma-separated list of backticked slugs.
pub fn render_slugs(rules: &[RuleId]) -> impl Display + '_ {
    rules
        .iter()
        .format_with(", ", |rule, f| f(&format_args!("`{rule}`")))
}

/// Whether `later`'s dependency column reaches `earlier`, directly or
/// through the column of a rule it already names. `false` for an
/// unknown slug on either side. Takes its pair in the opposite order
/// from [`precedes`], which asks about registration rather than the
/// declared column.
pub fn runs_behind(later: &str, earlier: &str) -> bool {
    let mut seen = BTreeSet::new();
    let mut pending = vec![later];
    while let Some(slug) = pending.pop() {
        for dependency in dependencies_of(slug) {
            if *dependency == earlier {
                return true;
            }
            if seen.insert(*dependency) {
                pending.push(dependency);
            }
        }
    }
    false
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
/// `false` when either is absent from the registry. Answers about the
/// registry's own order rather than the declared column [`runs_behind`]
/// walks, and takes its pair in the opposite order.
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
/// rule struct, and the slugs it must run behind. The slug is the
/// single source consumed by `RuleId::from_str`, the
/// `[tool.prose.rules.<slug>]` section name, the
/// `# prose: ignore[<slug>]` directive, and `--select` / `--ignore`.
/// Each rule's one-line imperative lives on its own type as
/// `MESSAGE`, which [`message_for_id`] reads back per slug.
///
/// Row order is pipeline order.
///
/// The macro asserts each slug's kebab shape and cross-row uniqueness
/// at compile time, holds every dependency to a rule seated earlier,
/// and emits a `pub(crate) const SLUG: RuleId` on each rule type so
/// `id()` collapses to `Self::SLUG`.
macro_rules! register_rules {
    ($($slug:literal: $field:ident: $config:ty => $ty:ident
        => [$($after:literal),*]),* $(,)?) => {
        pub(crate) const KNOWN_IDS: &[RuleId] = &[
            $(RuleId($slug)),*
        ];

        /// Each rule's one-line imperative, indexed alongside [`KNOWN_IDS`].
        const MESSAGES: &[&str] = &[$($ty::MESSAGE),*];

        /// The slugs each rule runs behind, indexed alongside [`KNOWN_IDS`].
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
        #[derive(Clone, Debug, Default, Deserialize, Serialize)]
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

        /// Default backing for [`Rule::message`], the `MESSAGE` const
        /// on the rule registered under `id`.
        pub(crate) fn message_for_id(id: RuleId) -> &'static str {
            match slug_index(id.as_str()) {
                Some(index) => MESSAGES[index],
                None => unreachable!("rule id must be registered"),
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
                let id = RuleId::from_str(&name.replace('_', "-")).ok()?;
                Some(Self::with_filters(config, &[id], &[]))
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
            ///
            /// Each rule is built from a config whose `enabled` flags
            /// carry the resolved set, so a rule predicting what a later
            /// rule does to a column reads whether that rule runs in this
            /// pipeline rather than whether the file enables it.
            pub fn with_filters(
                config: &Config,
                select: &[RuleId],
                ignore: &[RuleId],
            ) -> Self {
                let mut resolved = config.clone();
                $({
                    let id = RuleId($slug);
                    let included = if select.is_empty() {
                        config.rules.$field.enabled
                    } else {
                        select.contains(&id)
                    };
                    resolved.rules.$field.enabled = included && !ignore.contains(&id);
                })*
                let mut rules: Vec<Box<dyn Rule>> = Vec::new();
                $({
                    if resolved.rules.$field.enabled {
                        rules.push(Box::new($ty::from_config(&resolved)));
                    }
                })*
                Self::from_rules(rules).targeting(resolved.target_version)
            }
        }
    };
}

register_rules! {
    "shed-backslash-continuations": shed_backslash_continuations: ToggleOnly                 => ShedBackslashContinuations => [],
    "normalize-literals":           normalize_literals:           NormalizeLiteralsConfig    => NormalizeLiterals          => [],
    "prune-inert-imports":          prune_inert_imports:          PruneInertImportsConfig    => PruneInertImports          => ["shed-backslash-continuations", "normalize-literals"],
    "strip-none-return":            strip_none_return:            ToggleOnly                 => StripNoneReturn            => [],
    "modernize-annotations":        modernize_annotations:        ModernizeAnnotationsConfig => ModernizeAnnotations       => [],
    "strip-trailing-commas":        strip_trailing_commas:        ToggleOnly                 => StripTrailingCommas        => [],
    "normalize-comparisons":        normalize_comparisons:        NormalizeComparisonsConfig => NormalizeComparisons       => [],
    "reflow-parentheses":           reflow_parentheses:           ToggleOnly                 => ReflowParentheses          => ["shed-backslash-continuations", "normalize-comparisons"],
    "shed-redundant-base":          shed_redundant_base:          ToggleOnly                 => ShedRedundantBase          => [],
    "simplify-comprehensions":      simplify_comprehensions:      ToggleOnly                 => SimplifyComprehensions     => ["reflow-parentheses"],
    "frame-docstrings":             frame_docstrings:             ToggleOnly                 => FrameDocstrings            => [],
    "expand-docstrings":            expand_docstrings:            ToggleOnly                 => ExpandDocstrings           => ["frame-docstrings"],
    "group-imports":                group_imports:                ToggleOnly                 => GroupImports               => [],
    "shed-super-args":              shed_super_args:              ToggleOnly                 => ShedSuperArgs              => [],
    "stack-method-chains":          stack_method_chains:          StackMethodChainsConfig    => StackMethodChains          => ["reflow-parentheses"],
    "reflow-calls":                 reflow_calls:                 ReflowCallsConfig          => ReflowCalls                => ["shed-backslash-continuations", "reflow-parentheses", "simplify-comprehensions", "shed-super-args", "stack-method-chains"],
    "reflow-signatures":            reflow_signatures:            ReflowSignaturesConfig     => ReflowSignatures           => ["strip-none-return", "reflow-parentheses"],
    "reflow-collections":           reflow_collections:           ReflowCollectionsConfig    => ReflowCollections          => ["simplify-comprehensions", "stack-method-chains", "reflow-calls", "reflow-signatures"],
    "prefer-fstring":               prefer_fstring:               PreferFstringConfig        => PreferFstring              => ["normalize-literals", "reflow-collections"],
    "stack-adjacent-strings":       stack_adjacent_strings:       ToggleOnly                 => StackAdjacentStrings       => ["stack-method-chains", "reflow-collections", "reflow-calls", "reflow-signatures"],
    "align-match-case":             align_match_case:             AlignmentConfig            => AlignMatchCase             => ["reflow-parentheses"],
    "reflow-imports":               reflow_imports:               ReflowImportsConfig        => ReflowImports              => ["shed-backslash-continuations", "prune-inert-imports", "group-imports"],
    "band-constants":               band_constants:               BandConstantsConfig        => BandConstants              => ["simplify-comprehensions", "reflow-imports"],
    "alphabetize-siblings":         alphabetize_siblings:         AlphabetizeSiblingsConfig  => AlphabetizeSiblings        => ["normalize-literals", "strip-trailing-commas", "reflow-parentheses", "frame-docstrings", "expand-docstrings", "stack-method-chains", "reflow-collections", "reflow-calls", "reflow-signatures", "reflow-imports", "band-constants"],
    "space-statements":             space_statements:             ToggleOnly                 => SpaceStatements            => ["prune-inert-imports", "group-imports", "alphabetize-siblings", "band-constants"],
    "align-imports":                align_imports:                AlignmentConfig            => AlignImports               => ["reflow-imports", "alphabetize-siblings", "band-constants", "space-statements"],
    "align-colons":                 align_colons:                 AlignmentConfig            => AlignColons                => ["strip-trailing-commas", "reflow-parentheses", "reflow-collections", "reflow-signatures", "stack-adjacent-strings", "alphabetize-siblings", "band-constants"],
    "wrap-docstrings":              wrap_docstrings:              ToggleOnly                 => WrapDocstrings             => ["frame-docstrings", "expand-docstrings", "align-colons"],
    "align-equals":                 align_equals:                 AlignmentConfig            => AlignEquals                => ["strip-trailing-commas", "reflow-parentheses", "reflow-collections", "alphabetize-siblings", "band-constants", "align-colons"],
    "align-comparisons":            align_comparisons:            AlignmentConfig            => AlignComparisons           => ["reflow-parentheses", "normalize-comparisons", "reflow-calls", "reflow-collections"],
    "strip-stranded-padding":       strip_stranded_padding:       ToggleOnly                 => StripStrandedPadding       => ["reflow-parentheses", "align-match-case", "align-imports", "align-colons", "align-equals", "align-comparisons"],
    "normalize-comment-spacing":    normalize_comment_spacing:    ToggleOnly                 => NormalizeCommentSpacing    => [],
    "align-comments":               align_comments:               AlignmentConfig            => AlignComments              => ["strip-trailing-commas", "strip-stranded-padding", "normalize-comment-spacing"],
    "bare-imports":                 bare_imports:                 BareImportsConfig          => BareImports                => [],
    "miscased-constants":           miscased_constants:           MiscasedConstantsConfig    => MiscasedConstants          => [],
    "reassigned-constants":         reassigned_constants:         ReassignedConstantsConfig  => ReassignedConstants        => [],
    "step-narration":               step_narration:               ToggleOnly                 => StepNarration              => [],
    "inlinable-bindings":           inlinable_bindings:           InlinableBindingsConfig    => InlinableBindings          => [],
    "unsorted-positionals":         unsorted_positionals:         ToggleOnly                 => UnsortedPositionals        => [],
    "signature-annotations":        signature_annotations:        ToggleOnly                 => SignatureAnnotations       => [],
    "restated-types":               restated_types:               ToggleOnly                 => RestatedTypes              => ["frame-docstrings", "expand-docstrings", "wrap-docstrings"],
    "line-overflow":                line_overflow:                LineOverflowConfig         => LineOverflow               => ["strip-stranded-padding", "normalize-comment-spacing", "align-comments"],
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn dependencies_of_returns_empty_for_a_rule_without_predecessors(
        #[values("strip-none-return", "shed-super-args", "not-a-rule")] slug: &str,
    ) {
        assert!(dependencies_of(slug).is_empty());
    }

    #[test]
    fn dependencies_of_returns_the_declared_predecessors() {
        assert_eq!(
            dependencies_of("align-equals"),
            [
                "strip-trailing-commas",
                "reflow-parentheses",
                "reflow-collections",
                "alphabetize-siblings",
                "band-constants",
                "align-colons",
            ],
        );
    }

    #[rstest]
    fn is_valid_slug_accepts_canonical_kebab_shapes(
        #[values("a", "a-b", "abc123", "inlinable-bindings")] valid: &str,
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
    #[case("reflow-collections", "align-equals", true)]
    #[case("align-equals", "reflow-collections", false)]
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
    fn rule_id_from_str_rejects_a_retired_slug(
        #[values(
            "alphabetize",
            "blank-lines",
            "call-layout",
            "chain-layout",
            "collection-layout",
            "comment-spacing",
            "docstring-expand",
            "docstring-frame",
            "docstring-wrap",
            "import-layout",
            "legacy-union-syntax",
            "shed-parentheses",
            "signature-layout",
            "strip-align-padding",
            "unused-future-annotations"
        )]
        retired: &str,
    ) {
        let err = retired
            .parse::<RuleId>()
            .expect_err("a retired slug resolves against nothing");
        assert_eq!(err.0, retired);
        assert!(Pipeline::for_rule(retired, &Config::default()).is_none());
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
    fn runs_behind_reaches_a_dependency_through_another_rules_column() {
        assert!(
            !dependencies_of("align-comments").contains(&"align-colons"),
            "the direct column does not name it",
        );
        assert!(runs_behind("align-comments", "align-colons"));
    }

    #[rstest]
    #[case("bare-imports", "align-colons")]
    #[case("reflow-parentheses", "align-comments")]
    #[case("not-a-rule", "align-colons")]
    #[case("align-colons", "not-a-rule")]
    fn runs_behind_rejects_a_slug_no_column_reaches(#[case] later: &str, #[case] earlier: &str) {
        assert!(!runs_behind(later, earlier));
    }

    #[test]
    fn slug_bytes_equal_matches_only_identical_slices() {
        assert!(slug_bytes_equal(b"foo", b"foo"));
        assert!(!slug_bytes_equal(b"foo", b"food"));
        assert!(!slug_bytes_equal(b"foo", b"bar"));
    }
}
