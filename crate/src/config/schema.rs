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
    /// How far apart the widest and narrowest rows of a run may be for the
    /// run to still align on one column. A positive `N` caps that gap, `0`
    /// forbids any padding so every row sits flush, and `false` lifts the
    /// cap so a run of any width aligns on one column. A row marked
    /// `# prose: skip` stays out of its group.
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

/// A glob a lint matches names against as a whole, where the empty
/// pattern matches no name.
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
    /// Sorts class and function definitions alphabetically, keeping each
    /// below any sibling it names at evaluation time. `false` keeps
    /// definitions in source order while everything else still sorts.
    pub sort_definitions: bool,
    /// Sorts the entries of a dict literal, scalar values before
    /// collection values and alphabetical by key within each. `false`
    /// keeps the order as written, which iteration, `.items()`, and `**`
    /// unpacking all follow.
    pub sort_dict_keys: bool,
    /// Sorts the `name: description` entries of a Title-case-headed
    /// docstring section, parameter entries in the signature's order as
    /// the rule leaves it and every other entry alphabetical below them.
    /// `false` keeps the entries in the order written while everything
    /// else still sorts.
    pub sort_docstring_entries: bool,
    /// Sorts the string items inside `__all__` and `__slots__`. `false`
    /// keeps the order written, for a hand-ordered public API.
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
    /// Caps how many evaluation tiers get their own blank-line-separated
    /// sub-band, merging every deeper tier into the last. `1` keeps the
    /// whole band together and `false` gives every tier its own sub-band.
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
    /// Modules whose bare `import` form is kept whatever their attribute
    /// count.
    pub allow: Vec<String>,
    pub enabled: bool,
    /// Exempts every aliased bare import (`import x as y`) from the rule.
    pub exempt_aliased: bool,
    /// The number of distinct attributes at or below which an unaliased
    /// bare import is reported.
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
    /// Turns the cache on or off.
    pub enabled: bool,
    /// The entry count LRU eviction reduces the cache directory to.
    pub max_entries: u32,
    /// The size in MiB LRU eviction reduces the cache directory to.
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
    /// Root package names whose imports sort into the local-package
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
/// across the layout rules. `Some(n)` caps how many elements a construct
/// keeps on one line, and `None` lifts the cap so width alone decides the
/// layout.
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
    /// Suggests the parenthesized adjacent-literal form for an over-budget
    /// line whose overflow sits inside one string literal containing
    /// whitespace, as a display-only fix `prose format` never writes.
    /// `false` reports the overflow alone.
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

/// How much padding a row may take to align, read from `max-shift`.
/// `Unlimited` lifts the cap so a run always aligns on its widest
/// member. `NoShift` forbids any padding, so every row sits flush.
/// `Cap(n)` aligns a run while the gap between its widest and narrowest
/// rows stays within `n`.
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
    /// Swaps the operands of a comparison whose constant side comes first,
    /// so `42 == n` reads `n == 42`. `false` keeps the operand order as
    /// written.
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
    /// Rewrites a non-docstring string to `"` quotes, using `'` only where
    /// that avoids an escape, and removes a backslash the chosen quote
    /// makes unnecessary. `false` keeps the literal as written.
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
    /// Drops an import binding a name nothing references, unless the
    /// binding is marked for re-export, read by a `del` or a quoted
    /// annotation, or bound in a package `__init__.py`, in which case it
    /// is reported instead. `false` keeps every unreferenced import and
    /// reports none.
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
    /// Explodes a call whose every argument can be written as a keyword
    /// to one `name=value` per line once its argument count exceeds the
    /// cap. `false` turns the count trigger off and leaves only the
    /// `code-line-length` budget.
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
    /// Explodes a collection that overflows the line budget or exceeds its
    /// entry cap to one entry per line. `false` turns every explosion off,
    /// leaving the count cap with no effect.
    pub explode: bool,
    /// Keeps a literal the author wrote as a bracketed column of two or
    /// more entries, one per line. `false` joins one back onto a single
    /// line where it fits the budget, and any other multi-line layout
    /// rejoins either way.
    pub keep_multiline_literals: bool,
    /// Caps how many atomic entries, meaning ints, floats, strings, and
    /// single names, one packed row of an expanded collection carries.
    /// `false` removes the cap and packs each row by width alone.
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
    /// Merges repeated `from <module> import …` statements into one
    /// statement naming each member once, in the order
    /// `alphabetize-siblings` gives them. `false` keeps each statement as
    /// written.
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
    /// Explodes a signature to one parameter per line once its parameter
    /// count exceeds the cap. `false` turns the count trigger off and
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
/// `[tool.prose.rules]`, leaving every other facet at its default.
pub(crate) trait RuleToggle: Default {
    fn with_enabled(enabled: bool) -> Self;
}

/// Configuration for the `stack-method-chains` rule.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct StackMethodChainsConfig {
    pub enabled: bool,
    /// Breaks a method chain to one link per line once its link count
    /// exceeds the cap. `false` turns the count trigger off and leaves
    /// only the `code-line-length` budget.
    pub max_links: InlineBudget,
    /// How far past the broken chain's indent a hanging link's dot column
    /// may sit. A receiver wider than that takes the full split instead,
    /// standing alone with every link flush beneath it. `0` always takes
    /// the full split, and `false` lifts the cap so every chain hangs.
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

/// Sub-table shape for rules whose only facet is `enabled`.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ToggleOnly {
    /// Turns the rule on or off.
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

/// Implements [`RuleToggle`] for configs carrying facets beyond
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
