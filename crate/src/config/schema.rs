//! The per-rule config sub-tables, the rule-toggle macro, and the
//! shared `MaxShift` and docstring-policy enums.

use std::{borrow::Cow, fmt, num::NonZeroUsize, str::FromStr};

use globset::{Glob, GlobMatcher};
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{
    de::{deserialize_cap_or_false, deserialize_optional_cap, serialize_optional_cap},
    json_schema::{cap_or_false_schema, optional_cap_schema},
};

/// Alignment-rule config shared by every rule that aligns a token
/// across consecutive lines.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct AlignmentConfig {
    pub enabled: bool,
    /// The width-spread budget a contiguous run may shift to reach the
    /// shared column. A positive `N` caps the spread, `0` forbids any
    /// shift so every row sits flush, and `false` lifts the cap so a
    /// contiguous run folds into one column. To hold one row out of an
    /// otherwise-aligned group, mark it with `# prose: skip`.
    pub max_shift: MaxShift,
}

impl Default for AlignmentConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_shift: MaxShift::default(),
        }
    }
}

/// A glob a lint reads names against, matched against the whole name,
/// the empty pattern matching no name at all.
#[derive(Clone)]
pub struct AllowPattern(GlobMatcher);

impl AllowPattern {
    /// The pattern as written.
    pub(crate) fn as_str(&self) -> &str {
        self.0.glob().glob()
    }

    /// True when the whole of `name` matches the pattern.
    pub(crate) fn matches(&self, name: &str) -> bool {
        !self.as_str().is_empty() && self.0.is_match(name)
    }
}

impl fmt::Debug for AllowPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("AllowPattern").field(&self.as_str()).finish()
    }
}

impl<'de> Deserialize<'de> for AllowPattern {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

impl FromStr for AllowPattern {
    type Err = globset::Error;

    fn from_str(pattern: &str) -> Result<Self, Self::Err> {
        Glob::new(pattern).map(|glob| Self(glob.compile_matcher()))
    }
}

impl JsonSchema for AllowPattern {
    fn schema_name() -> Cow<'static, str> {
        Cow::Borrowed("AllowPattern")
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        String::json_schema(generator)
    }

    fn inline_schema() -> bool {
        true
    }
}

impl Serialize for AllowPattern {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// Configuration for the `alphabetize-siblings` rule, each facet gating one
/// sort pass and defaulting `true`.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct AlphabetizeSiblingsConfig {
    pub enabled: bool,
    /// Groups methods into dunders, properties, privates, and publics
    /// before sorting within each group. `false` sorts methods by plain
    /// name alone.
    pub group_methods: bool,
    /// Reorders class and function definitions alphabetically, holding
    /// each behind any sibling it names at evaluation time. `false`
    /// freezes definitions in source order while other surfaces still
    /// sort.
    pub sort_definitions: bool,
    /// Reorders the keyed entries of a dict literal, scalar-valued
    /// entries before collection-valued and alphabetical by key within
    /// each. `false` keeps the authored order, which iteration,
    /// `.items()`, and `**` expansion all observe.
    pub sort_dict_keys: bool,
    /// Reorders `name: description` entries within Title-case-headed
    /// docstring sections, parameter entries mirroring the signature as
    /// the rule leaves it and stragglers alphabetizing below. `false`
    /// keeps narrative-curated entry order while still sorting every
    /// other surface.
    pub sort_docstring_entries: bool,
    /// Reorders the string items inside `__all__` and `__slots__`.
    /// `false` keeps a hand-curated public-API order.
    pub sort_dunder_lists: bool,
}

impl Default for AlphabetizeSiblingsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            group_methods: true,
            sort_definitions: true,
            sort_dict_keys: true,
            sort_docstring_entries: true,
            sort_dunder_lists: true,
        }
    }
}

/// Configuration for the `band-constants` rule.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct BandConstantsConfig {
    pub enabled: bool,
    /// Clusters each band by subcategory, the type aliases first, then
    /// the `SCREAMING_CASE` constants, then the remaining module state,
    /// before sorting by name within each. `false` sorts by tier and
    /// name alone.
    pub group_subcategories: bool,
    /// Caps how many evaluation tiers open their own blank-separated
    /// sub-band, merging every deeper tier into the last. `1` holds the
    /// band tight and `false` opens one sub-band per tier.
    pub max_tiers: InlineBudget,
}

impl Default for BandConstantsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            group_subcategories: true,
            max_tiers: InlineBudget(NonZeroUsize::new(2)),
        }
    }
}

/// Configuration for the `bare-imports` rule.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct BareImportsConfig {
    /// Modules whose bare-import form is preserved whatever their
    /// attribute count.
    pub allow: Vec<String>,
    pub enabled: bool,
    /// Exempts every aliased bare import (`import x as y`) from the rule.
    pub exempt_aliased: bool,
    /// The distinct-attribute count at or below which an unaliased bare
    /// import is flagged.
    pub max_attributes: usize,
}

impl Default for BareImportsConfig {
    fn default() -> Self {
        Self {
            allow: Vec::new(),
            enabled: true,
            exempt_aliased: true,
            max_attributes: 4,
        }
    }
}

/// Cache settings parsed from `[tool.prose.cache]`.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct CacheConfig {
    /// Toggles the cache globally.
    pub enabled: bool,
    /// The LRU eviction cap on the cache directory's entry count.
    pub max_entries: u32,
    /// The LRU eviction cap on the cache directory.
    pub max_size_mib: u32,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_entries: 10_000,
            max_size_mib: 100,
        }
    }
}

/// Which budget structured docstring sections wrap to.
///
/// `CodeLineLength` reuses `Config::code_line_length`.
/// `DocstringLineLength` reuses `Config::docstring_line_length`.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DocstringStructuredPolicy {
    #[default]
    CodeLineLength,
    DocstringLineLength,
}

/// Settings parsed from `[tool.prose.imports]`.
#[derive(Clone, Debug, Default, Deserialize, JsonSchema, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ImportsConfig {
    /// Root package names whose imports lift into the local-package
    /// group.
    pub first_party: Vec<String>,
}

/// Configuration for the `inlinable-bindings` rule.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct InlinableBindingsConfig {
    /// Binding names exempted from the lint, a glob matched against the
    /// whole name.
    #[schemars(extend("default" = Self::default().allow_pattern.as_str()))]
    pub allow_pattern: AllowPattern,
    pub enabled: bool,
}

impl Default for InlinableBindingsConfig {
    fn default() -> Self {
        Self {
            allow_pattern: "_*".parse().expect("`_*` parses"),
            enabled: true,
        }
    }
}

/// An inline-element budget read from a `max-<element>` key and shared
/// across the layout rules. `Some(n)` caps the element count a construct
/// holds inline, and `None` lifts the cap so width alone gates the
/// shape.
#[derive(Clone, Copy, Debug)]
pub struct InlineBudget(pub(crate) Option<NonZeroUsize>);

impl InlineBudget {
    /// The cap as a plain count, `None` when the budget is uncapped.
    pub(crate) fn cap(self) -> Option<usize> {
        self.0.map(NonZeroUsize::get)
    }
}

impl<'de> Deserialize<'de> for InlineBudget {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self(deserialize_optional_cap(deserializer)?))
    }
}

impl JsonSchema for InlineBudget {
    fn schema_name() -> Cow<'static, str> {
        Cow::Borrowed("InlineBudget")
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        optional_cap_schema(generator)
    }
}

impl Serialize for InlineBudget {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serialize_optional_cap(&self.0, serializer)
    }
}

/// Configuration for the `line-overflow` rule.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct LineOverflowConfig {
    pub enabled: bool,
    /// Offers the parenthesized adjacent-literal form on an over-budget
    /// line whose overflow sits inside one string literal holding
    /// interior whitespace, as a display-only suggestion `prose format`
    /// never writes. `false` leaves the bare overflow report.
    pub suggest_string_splits: bool,
}

impl Default for LineOverflowConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            suggest_string_splits: true,
        }
    }
}

/// How far a row may shift to align, read from `max-shift`.
/// `Unlimited` lifts the cap so a contiguous run always aligns to its
/// widest member. `NoShift` forbids any shift, collapsing every row to
/// its minimal spacing. `Cap(n)` aligns a run while its width spread
/// stays within `n`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MaxShift {
    Cap(NonZeroUsize),
    NoShift,
    Unlimited,
}

impl Default for MaxShift {
    fn default() -> Self {
        Self::Cap(NonZeroUsize::new(16).expect("16 is non-zero"))
    }
}

impl<'de> Deserialize<'de> for MaxShift {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match deserialize_cap_or_false::<usize, _>(
            deserializer,
            "`max-shift` accepts a non-negative integer or `false`, not `true`",
        )? {
            Some(n) => Ok(NonZeroUsize::new(n).map_or(Self::NoShift, Self::Cap)),
            None => Ok(Self::Unlimited),
        }
    }
}

impl JsonSchema for MaxShift {
    fn schema_name() -> Cow<'static, str> {
        Cow::Borrowed("MaxShift")
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        cap_or_false_schema::<usize>(generator)
    }
}

impl Serialize for MaxShift {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match *self {
            Self::Cap(n) => serializer.serialize_u64(n.get() as u64),
            Self::NoShift => serializer.serialize_u64(0),
            Self::Unlimited => serializer.serialize_bool(false),
        }
    }
}

/// Configuration for the `miscased-constants` rule.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct MiscasedConstantsConfig {
    /// Constant names exempted from the lint, a glob matched against
    /// the whole name, such as old-style bare aliases.
    #[schemars(extend("default" = Self::default().allow_pattern.as_str()))]
    pub allow_pattern: AllowPattern,
    pub enabled: bool,
}

impl Default for MiscasedConstantsConfig {
    fn default() -> Self {
        Self {
            allow_pattern: "".parse().expect("empty pattern parses"),
            enabled: true,
        }
    }
}

/// Configuration for the `modernize-annotations` rule, each facet
/// gating one rewrite and defaulting `true`.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ModernizeAnnotationsConfig {
    pub enabled: bool,
    /// Converts a `typing` generic to the builtin PEP 585 gave it, so
    /// `List[int]` reads as `list[int]`. Runs on `target-version` 3.9
    /// and higher, and `false` leaves every `typing` generic in place.
    pub rewrite_generics: bool,
    /// Rewrites `Optional[X]` and `Union[X, Y]` to the PEP 604 `X | None`
    /// and `X | Y` forms. Runs on `target-version` 3.10 and higher, and
    /// `false` leaves every legacy union in place.
    pub rewrite_unions: bool,
}

impl Default for ModernizeAnnotationsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            rewrite_generics: true,
            rewrite_unions: true,
        }
    }
}

/// Configuration for the `normalize-comparisons` rule, each facet
/// gating one rewrite and defaulting `true`.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct NormalizeComparisonsConfig {
    pub enabled: bool,
    /// Rewrites a `==` or `!=` test against `None` to `is` or `is not`,
    /// and flags a test against `True` or `False` without rewriting it.
    /// `false` leaves every singleton comparison as written.
    pub rewrite_identity: bool,
    /// Folds a leading `not` into the `in` or `is` it negates, so
    /// `not a in b` reads `a not in b`. `false` keeps the outer `not`.
    pub rewrite_negation: bool,
    /// Flips a comparison whose constant side leads, so `42 == n` reads
    /// `n == 42`. `false` keeps the authored operand order.
    pub rewrite_operand_order: bool,
}

impl Default for NormalizeComparisonsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            rewrite_identity: true,
            rewrite_negation: true,
            rewrite_operand_order: true,
        }
    }
}

/// Configuration for the `normalize-literals` rule, each facet gating
/// one spelling axis and defaulting `true`.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct NormalizeLiteralsConfig {
    pub enabled: bool,
    /// Uppercases hex digits and lowercases the `0x`, `0o`, and `0b`
    /// radix markers, the `e` exponent, and the `j` suffix. `false`
    /// keeps every numeric literal spelled as written.
    pub unify_numerics: bool,
    /// Lowercases a string prefix and drops the no-op `u`. `false`
    /// keeps the prefix cased and ordered as written.
    pub unify_prefixes: bool,
    /// Settles a non-docstring string on `"`, falling to `'` only where
    /// that drops an escape, and sheds a backslash the surviving quote
    /// does not need. `false` keeps the literal spelled as written.
    pub unify_quotes: bool,
}

impl Default for NormalizeLiteralsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            unify_numerics: true,
            unify_prefixes: true,
            unify_quotes: true,
        }
    }
}

/// Configuration for the `prefer-fstring` rule, each facet gating one
/// rewrite and defaulting `true`.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct PreferFstringConfig {
    pub enabled: bool,
    /// Converts printf-style `%` interpolation to an f-string, so
    /// `"%s=%s" % (k, v)` reads as `f"{k}={v}"`. `false` leaves every
    /// `%` template in place.
    pub rewrite_percent: bool,
    /// Converts a `str.format()` call to an f-string, so
    /// `"{}={}".format(k, v)` reads as `f"{k}={v}"`. `false` leaves
    /// every `str.format()` call in place.
    pub rewrite_str_format: bool,
}

impl Default for PreferFstringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            rewrite_percent: true,
            rewrite_str_format: true,
        }
    }
}

/// Configuration for the `prune-inert-imports` rule, each facet gating
/// one prune and defaulting `true`.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct PruneInertImportsConfig {
    /// Drops an import rebinding a name an earlier import already bound
    /// to the same source. `false` keeps every repeat.
    pub drop_duplicates: bool,
    /// Drops an import binding a name nothing references, where the
    /// binding is not marked for re-export, read by a `del` or a quoted
    /// annotation, or bound in a package `__init__.py`, where it is
    /// reported instead. `false` leaves every unreferenced import in
    /// place and reports none of them.
    pub drop_unreferenced: bool,
    pub enabled: bool,
}

impl Default for PruneInertImportsConfig {
    fn default() -> Self {
        Self {
            drop_duplicates: true,
            drop_unreferenced: true,
            enabled: true,
        }
    }
}

/// Configuration for the `reassigned-constants` rule.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ReassignedConstantsConfig {
    /// Module-level names exempted from the lint.
    pub allow: Vec<String>,
    pub enabled: bool,
}

impl Default for ReassignedConstantsConfig {
    fn default() -> Self {
        Self {
            allow: Vec::new(),
            enabled: true,
        }
    }
}

/// Configuration for the `reflow-calls` rule.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ReflowCallsConfig {
    pub enabled: bool,
    /// Explodes a call to one keyword argument per line once its argument
    /// count exceeds the cap. `false` disables the count trigger and
    /// leaves every call inline.
    pub max_args: InlineBudget,
}

impl Default for ReflowCallsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_args: InlineBudget(NonZeroUsize::new(3)),
        }
    }
}

/// Configuration for the `reflow-collections` rule, each facet gating
/// one shape decision and defaulting `true`.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ReflowCollectionsConfig {
    pub enabled: bool,
    /// Expands an overflowing or over-count collection to one entry per
    /// line. `false` suppresses every expansion and leaves the count cap
    /// inert.
    pub explode: bool,
    /// Holds a literal the author laid out as a flush bracketed column
    /// of two or more entries. `false` joins one whose single-line form
    /// fits the budget, and every other break rejoins either way.
    pub keep_multiline_literals: bool,
    /// Keeps short collections on one line when each entry is an atomic
    /// literal and the run fits the cap. `false` removes the cap and
    /// packs by width alone.
    pub max_atomics: InlineBudget,
    /// Expands a dict once its entry count exceeds the cap, whatever its
    /// width. `false` disables the count trigger.
    pub max_dict_entries: InlineBudget,
    /// Breaks an over-wide `key: value` at its `:` and hangs the value
    /// beneath. `false` leaves the oversized entry on one line.
    pub wrap_dict_entries: bool,
}

impl Default for ReflowCollectionsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            explode: true,
            keep_multiline_literals: true,
            max_atomics: InlineBudget(NonZeroUsize::new(8)),
            max_dict_entries: InlineBudget(NonZeroUsize::new(3)),
            wrap_dict_entries: true,
        }
    }
}

/// Configuration for the `reflow-imports` rule, each facet gating one
/// statement move and defaulting `true`.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ReflowImportsConfig {
    pub enabled: bool,
    /// Folds repeated `from <module> import …` statements into one
    /// statement carrying each member once, ordered as `alphabetize-siblings`
    /// would leave it. `false` leaves each statement on its own line.
    pub merge_members: bool,
    /// Breaks a comma-joined `import a, b` into one `import` statement
    /// per module. `false` keeps the comma-joined form.
    pub split_multi_module: bool,
}

impl Default for ReflowImportsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            merge_members: true,
            split_multi_module: true,
        }
    }
}

/// Configuration for the `reflow-signatures` rule.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ReflowSignaturesConfig {
    pub enabled: bool,
    /// Expands a signature to one parameter per line once its parameter
    /// count exceeds the cap. `false` disables the count trigger and
    /// leaves only the `code-line-length` budget.
    pub max_params: InlineBudget,
}

impl Default for ReflowSignaturesConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_params: InlineBudget(NonZeroUsize::new(4)),
        }
    }
}

/// A per-rule config a bare bool can toggle. `with_enabled` is the
/// shorthand for the `{ enabled = <bool> }` table under
/// `[tool.prose.rules]`, leaving every other knob at its default.
pub(crate) trait RuleToggle: Default {
    fn with_enabled(enabled: bool) -> Self;
}

/// Configuration for the `stack-method-chains` rule.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct StackMethodChainsConfig {
    pub enabled: bool,
    /// Breaks a method chain to one link per line once its link count
    /// exceeds the cap. `false` disables the count trigger and leaves
    /// only the `code-line-length` budget.
    pub max_links: InlineBudget,
    /// The width a hung link's dot column may sit past the indent the
    /// broken chain opens at. A wider receiver takes the full split
    /// instead, standing alone with every link flush beneath it. `0`
    /// always takes that split, and `false` lifts the cap so every
    /// chain hangs.
    pub max_shift: MaxShift,
}

impl Default for StackMethodChainsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_links: InlineBudget(NonZeroUsize::new(2)),
            max_shift: MaxShift::default(),
        }
    }
}

/// Sub-table shape for rules whose only knob is `enabled`.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ToggleOnly {
    /// Toggles the rule on or off.
    pub enabled: bool,
}

impl Default for ToggleOnly {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl RuleToggle for ToggleOnly {
    fn with_enabled(enabled: bool) -> Self {
        Self { enabled }
    }
}

/// Implements [`RuleToggle`] for configs carrying knobs beyond
/// `enabled`, filling the rest from `Default`.
macro_rules! impl_rule_toggle {
    ($($config:ty),+ $(,)?) => {
        $(impl RuleToggle for $config {
            fn with_enabled(enabled: bool) -> Self {
                Self { enabled, ..Self::default() }
            }
        })+
    };
}

impl_rule_toggle!(
    AlignmentConfig,
    AlphabetizeSiblingsConfig,
    BandConstantsConfig,
    BareImportsConfig,
    InlinableBindingsConfig,
    LineOverflowConfig,
    MiscasedConstantsConfig,
    ModernizeAnnotationsConfig,
    NormalizeComparisonsConfig,
    NormalizeLiteralsConfig,
    PreferFstringConfig,
    PruneInertImportsConfig,
    ReassignedConstantsConfig,
    ReflowCallsConfig,
    ReflowCollectionsConfig,
    ReflowImportsConfig,
    ReflowSignaturesConfig,
    StackMethodChainsConfig,
);
