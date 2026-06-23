//! The per-rule config sub-tables, the rule-toggle macro, and the
//! shared `MaxShift` and docstring-policy enums.

use std::num::NonZeroUsize;

use regex_lite::Regex;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::de::{deserialize_optional_cap, deserialize_regex, serialize_regex};

/// Alignment-rule config shared by every rule that aligns a token
/// across consecutive lines. `max_shift` caps how far a row may shift
/// to reach the column.
#[derive(Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct AlignmentConfig {
    pub enabled: bool,
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

/// Configuration for the `alphabetize` rule. `sort_docstring_entries`
/// gates the Google-style entry-section reorder pass, leaving the
/// AST-level sorts to apply on their own when set `false`.
#[derive(Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct AlphabetizeConfig {
    pub enabled: bool,
    pub sort_docstring_entries: bool,
}

impl Default for AlphabetizeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sort_docstring_entries: true,
        }
    }
}

/// Configuration for the `bare_imports` rule.
#[derive(Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct BareImportsConfig {
    pub allow: Vec<String>,
    pub enabled: bool,
    pub exempt_aliased: bool,
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
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
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
///
/// `max_args` caps the count threshold. A positive integer enforces the
/// cap. `false` disables the count trigger.
#[derive(Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct CallLayoutConfig {
    pub enabled: bool,
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

/// Configuration for the `collection_layout` rule.
///
/// `collapse`, `explode`, and `wrap_dict_entries` each gate one shape
/// move and default `true`. `collapse` joins a fitting multi-line
/// literal, subscript, or dict key back to one line. `explode` drives
/// every expansion, the width-driven spread and the `max_dict_entries`
/// count trigger alike, so `false` leaves the count cap inert.
/// `wrap_dict_entries` breaks an over-wide `key: value` at its `:` and
/// hangs the value beneath.
///
/// `max_atomics` and `max_dict_entries` each take a positive integer or
/// `false`. The integer sets the cap, and `false` disables it, leaving
/// width as the only gate.
#[derive(Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct CollectionLayoutConfig {
    pub collapse: bool,
    pub enabled: bool,
    pub explode: bool,
    pub max_atomics: InlineBudget,
    pub max_dict_entries: InlineBudget,
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
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DocstringStructuredPolicy {
    #[default]
    CodeLineLength,
    DocstringLineLength,
}

/// Settings parsed from `[tool.prose.imports]`. `first_party` lists
/// the package names whose imports group with relative imports as
/// local-package, keyed kebab-case under `first-party`.
#[derive(Debug, Default, Deserialize, Serialize)]
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

impl Serialize for InlineBudget {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            Some(n) => serializer.serialize_u64(n.get() as u64),
            None => serializer.serialize_bool(false),
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
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            Cap(usize),
            Switch(bool),
        }
        match Repr::deserialize(deserializer)? {
            Repr::Cap(n) => Ok(NonZeroUsize::new(n).map_or(Self::NoShift, Self::Cap)),
            Repr::Switch(false) => Ok(Self::Unlimited),
            Repr::Switch(true) => Err(serde::de::Error::custom(
                "`max-shift` accepts a non-negative integer or `false`, not `true`",
            )),
        }
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
#[derive(Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ReassignedConstantsConfig {
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
///
/// `max_params` caps the count threshold. A positive integer enforces
/// the cap. `false` disables the count trigger.
#[derive(Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct SignatureLayoutConfig {
    pub enabled: bool,
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
#[derive(Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct SingleUseVariablesConfig {
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
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
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
