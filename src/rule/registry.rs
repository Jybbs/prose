//! The `register_rules!` macro and the registry table binding each
//! rule slug to its config sub-table, struct, and message.

use serde::{Deserialize, Serialize};

use super::id::RuleId;
use super::slug::{is_valid_slug, slug_bytes_equal};
use super::trait_::Rule;
use crate::{
    config::{
        AlignImportsConfig, AlignmentConfig, AlphabetizeConfig, BareImportsConfig,
        CallLayoutConfig, CollectionLayoutConfig, Config, ReassignedConstantsConfig,
        SignatureLayoutConfig, SingleUseVariablesConfig, ToggleOnly,
    },
    pipeline::Pipeline,
    rules::{
        align_colons::AlignColons, align_comparisons::AlignComparisons, align_equals::AlignEquals,
        align_imports::AlignImports, align_match_case::AlignMatchCase, alphabetize::Alphabetize,
        bare_imports::BareImports, blank_lines::BlankLines, call_layout::CallLayout,
        collection_layout::CollectionLayout, docstring_expand::DocstringExpand,
        docstring_frame::DocstringFrame, docstring_wrap::DocstringWrap,
        import_layout::ImportLayout, legacy_union_syntax::LegacyUnionSyntax,
        reassigned_constants::ReassignedConstants, signature_layout::SignatureLayout,
        single_use_variables::SingleUseVariables, step_narration::StepNarration,
        strip_align_padding::StripAlignPadding, strip_trailing_commas::StripTrailingCommas,
        unused_future_annotations::UnusedFutureAnnotations,
    },
};

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
        pub(super) fn message_for_id(id: RuleId) -> &'static str {
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
    "signature-layout":          signature_layout:          SignatureLayoutConfig     => SignatureLayout         => "normalize function signature to one-line or one-per-line shape",
    "import-layout":             import_layout:             ToggleOnly                => ImportLayout            => "split an over-long `from` import into repeated-prefix lines",
    "align-imports":             align_imports:             AlignImportsConfig        => AlignImports            => "align consecutive `import`s",
    "align-colons":              align_colons:              AlignmentConfig           => AlignColons             => "align consecutive `:` separators",
    "docstring-wrap":            docstring_wrap:            ToggleOnly                => DocstringWrap           => "wrap docstring prose to the configured budget",
    "align-equals":              align_equals:              AlignmentConfig           => AlignEquals             => "align consecutive `=` operators",
    "align-comparisons":         align_comparisons:         AlignmentConfig           => AlignComparisons        => "align consecutive comparison operators",
    "strip-align-padding":       strip_align_padding:       ToggleOnly                => StripAlignPadding       => "drop padding from a group with no column to align to",
    "reassigned-constants":      reassigned_constants:      ReassignedConstantsConfig => ReassignedConstants     => "SCREAMING_CASE name is reassigned despite its constant casing. Rename it lowercase or keep it write-once",
    "step-narration":            step_narration:            ToggleOnly                => StepNarration           => "numbered-step comment found. Consider extracting each step as a named function",
    "legacy-union-syntax":       legacy_union_syntax:       ToggleOnly                => LegacyUnionSyntax       => "rewrite legacy `Optional`/`Union` to PEP 604 union syntax",
    "single-use-variables":      single_use_variables:      SingleUseVariablesConfig  => SingleUseVariables      => "binding is assigned and used once. Consider inlining",
}
