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
    /// shared column, `0` forbidding any shift and `false` folding a run
    /// into one column regardless of spread.
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
    /// Keys methods on the dunder-property-private-public grouping before
    /// name, dropping to name alone when `false`.
    pub group_methods: bool,
    /// Reorders class and function definitions, freezing them in source
    /// order when `false`.
    pub sort_definitions: bool,
    /// Reorders Google-style docstring entry sections.
    pub sort_docstring_entries: bool,
    /// Reorders the `__all__` and `__slots__` string lists.
    pub sort_dunder_lists: bool,
}

impl Default for AlphabetizeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            group_methods: true,
            sort_definitions: true,
            sort_docstring_entries: true,
            sort_dunder_lists: true,
        }
    }
}

/// Configuration for the `bare_imports` rule.
#[derive(Debug, Deserialize, JsonSchema, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct BareImportsConfig {
    /// Module names left as bare imports regardless of attribute reach.
    pub allow: Vec<String>,
    pub enabled: bool,
    /// Exempts an aliased import from the rewrite.
    pub exempt_aliased: bool,
    /// The attribute-reach count above which a bare import stays as-is.
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
    pub enabled: bool,
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
    /// The inline argument count above which a call explodes one-per-line,
    /// `false` leaving width as the only trigger.
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
    /// Joins a fitting multi-line literal, subscript, or dict key back to
    /// one line.
    pub collapse: bool,
    pub enabled: bool,
    /// Drives every expansion, the width spread and the entry-count cap
    /// alike, so `false` leaves the count cap inert.
    pub explode: bool,
    /// The inline element count above which a collection explodes,
    /// `false` leaving width as the only trigger.
    pub max_atomics: InlineBudget,
    /// The dict entry count above which the dict explodes, `false`
    /// leaving width as the only trigger.
    pub max_dict_entries: InlineBudget,
    /// Breaks an over-wide `key: value` at its `:` and hangs the value
    /// beneath.
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

/// Settings parsed from `[tool.prose.imports]`. `first_party` lists
/// the package names whose imports group with relative imports as
/// local-package, keyed kebab-case under `first-party`.
#[derive(Debug, Default, Deserialize, JsonSchema, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ImportsConfig {
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

/// Configuration for the `reassigned_constants` rule.
#[derive(Debug, Deserialize, JsonSchema, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ReassignedConstantsConfig {
    /// Constant names exempt from the reassignment flag.
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

/// Configuration for the `signature_layout` rule.
#[derive(Debug, Deserialize, JsonSchema, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct SignatureLayoutConfig {
    pub enabled: bool,
    /// The parameter count above which a signature explodes one-per-line,
    /// `false` leaving width as the only trigger.
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
    /// A regex whose matching variable names are left un-inlined.
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

/// A per-rule config a bare bool can toggle. `with_enabled` is the
/// shorthand for the `{ enabled = <bool> }` table under
/// `[tool.prose.rules]`, leaving every other knob at its default.
pub(crate) trait RuleToggle: Default {
    fn with_enabled(enabled: bool) -> Self;
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
    BareImportsConfig,
    CallLayoutConfig,
    CollectionLayoutConfig,
    ReassignedConstantsConfig,
    SignatureLayoutConfig,
    SingleUseVariablesConfig,
);
