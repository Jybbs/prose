//! The per-rule config sub-tables, the rule-toggle macro, and the
//! shared `MaxShift` and docstring-policy enums.

use std::{borrow::Cow, num::NonZeroUsize};

use regex_lite::Regex;
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{
    de::{
        deserialize_cap_or_false, deserialize_optional_cap, deserialize_regex,
        serialize_optional_cap, serialize_regex,
    },
    json_schema::{cap_or_false_schema, optional_cap_schema},
};

/// Alignment-rule config shared by every rule that aligns a token
/// across consecutive lines.
#[derive(Debug, Deserialize, JsonSchema, Serialize)]
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

/// Configuration for the `alphabetize` rule, each facet gating one
/// sort pass and defaulting `true`.
#[derive(Debug, Deserialize, JsonSchema, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct AlphabetizeConfig {
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
    /// Reorders the keyed entries of a dict literal, single-line entries
    /// before multi-line and alphabetical by key within each. `false`
    /// keeps the authored order, which iteration, `.items()`, and `**`
    /// expansion all observe.
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

impl Default for AlphabetizeConfig {
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

/// Configuration for the `band_constants` rule.
#[derive(Debug, Deserialize, JsonSchema, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct BandConstantsConfig {
    pub enabled: bool,
    /// Clusters each band by subcategory, the type aliases first, then
    /// the `SCREAMING_CASE` constants, then the remaining module state,
    /// before sorting by name within each. `false` sorts by tier and
    /// name alone.
    pub group_constants: bool,
    /// Caps how many evaluation tiers open their own blank-separated
    /// sub-band, merging every deeper tier into the last. `1` holds the
    /// band tight and `false` opens one sub-band per tier.
    pub max_tiers: InlineBudget,
}

impl Default for BandConstantsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            group_constants: true,
            max_tiers: InlineBudget(NonZeroUsize::new(2)),
        }
    }
}

/// Configuration for the `bare_imports` rule.
#[derive(Debug, Deserialize, JsonSchema, Serialize)]
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
    /// The LRU eviction cap on the cache directory.
    pub max_size_mib: u32,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_size_mib: 100,
        }
    }
}

/// Configuration for the `call_layout` rule.
#[derive(Debug, Deserialize, JsonSchema, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct CallLayoutConfig {
    pub enabled: bool,
    /// Explodes a call to one keyword argument per line once its argument
    /// count exceeds the cap. `false` disables the count trigger and
    /// leaves every call inline.
    pub max_args: InlineBudget,
}

impl Default for CallLayoutConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_args: InlineBudget(NonZeroUsize::new(3)),
        }
    }
}

/// Configuration for the `collection_layout` rule, each shape facet
/// gating one move and defaulting `true`.
#[derive(Debug, Deserialize, JsonSchema, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct CollectionLayoutConfig {
    /// Joins a fitting multi-line literal, subscript, comprehension, or
    /// dict key back to one line. `false` freezes those shapes where they
    /// sit.
    pub collapse: bool,
    pub enabled: bool,
    /// Expands an overflowing or over-count collection to one entry per
    /// line. `false` suppresses every expansion and leaves the count cap
    /// inert.
    pub explode: bool,
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

impl Default for CollectionLayoutConfig {
    fn default() -> Self {
        Self {
            collapse: true,
            enabled: true,
            explode: true,
            max_atomics: InlineBudget(NonZeroUsize::new(8)),
            max_dict_entries: InlineBudget(NonZeroUsize::new(3)),
            wrap_dict_entries: true,
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
#[derive(Debug, Default, Deserialize, JsonSchema, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ImportsConfig {
    /// Root package names whose imports lift into the local-package
    /// group.
    pub first_party: Vec<String>,
}

/// An inline-element budget read from a `max-<element>` key and shared
/// across the layout rules. `Some(n)` caps the element count a construct
/// holds inline, and `None` lifts the cap so width alone gates the
/// shape.
#[derive(Clone, Copy, Debug)]
pub struct InlineBudget(Option<NonZeroUsize>);

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

/// Configuration for the `miscased_constants` rule.
#[derive(Debug, Deserialize, JsonSchema, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct MiscasedConstantsConfig {
    /// Constant names exempted from the lint, such as old-style bare
    /// aliases.
    #[schemars(schema_with = "super::json_schema::empty_allow_pattern_schema")]
    #[serde(
        deserialize_with = "deserialize_regex",
        serialize_with = "serialize_regex"
    )]
    pub allow_pattern: Regex,
    pub enabled: bool,
}

impl Default for MiscasedConstantsConfig {
    fn default() -> Self {
        Self {
            allow_pattern: Regex::new("").expect("empty pattern compiles"),
            enabled: true,
        }
    }
}

/// Configuration for the `reassigned_constants` rule.
#[derive(Debug, Deserialize, JsonSchema, Serialize)]
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

/// A per-rule config a bare bool can toggle. `with_enabled` is the
/// shorthand for the `{ enabled = <bool> }` table under
/// `[tool.prose.rules]`, leaving every other knob at its default.
pub(crate) trait RuleToggle: Default {
    fn with_enabled(enabled: bool) -> Self;
}

/// Configuration for the `signature_layout` rule.
#[derive(Debug, Deserialize, JsonSchema, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct SignatureLayoutConfig {
    pub enabled: bool,
    /// Expands a signature to one parameter per line once its parameter
    /// count exceeds the cap. `false` disables the count trigger and
    /// leaves only the `code-line-length` budget.
    pub max_params: InlineBudget,
}

impl Default for SignatureLayoutConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_params: InlineBudget(NonZeroUsize::new(4)),
        }
    }
}

/// Configuration for the `single_use_variables` rule.
#[derive(Debug, Deserialize, JsonSchema, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct SingleUseVariablesConfig {
    /// Binding names exempted from the lint.
    #[schemars(schema_with = "super::json_schema::allow_pattern_schema")]
    #[serde(
        deserialize_with = "deserialize_regex",
        serialize_with = "serialize_regex"
    )]
    pub allow_pattern: Regex,
    pub enabled: bool,
}

impl Default for SingleUseVariablesConfig {
    fn default() -> Self {
        Self {
            allow_pattern: Regex::new("^_").expect("`^_` compiles"),
            enabled: true,
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
    AlphabetizeConfig,
    BandConstantsConfig,
    BareImportsConfig,
    CallLayoutConfig,
    CollectionLayoutConfig,
    MiscasedConstantsConfig,
    ReassignedConstantsConfig,
    SignatureLayoutConfig,
    SingleUseVariablesConfig,
);
