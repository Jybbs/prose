//! The per-rule config sub-tables, the alignment-config and
//! rule-toggle macros, and the shared policy enums.

use std::num::NonZeroUsize;

use regex_lite::Regex;
use serde::{Deserialize, Serialize};

use super::de::{
    deserialize_max_atomics_per_line, deserialize_max_inline_args,
    deserialize_max_inline_dict_entries, deserialize_max_inline_params, deserialize_regex,
    serialize_regex,
};

/// Stamps an alignment-rule config struct whose `max_shift` defaults
/// to `$shift`. Each alignment rule binds one of these, so a rule
/// warranting a wider starting cap seeds its own default off the same
/// shape rather than a hand-copied parallel struct.
macro_rules! alignment_config {
    ($name:ident, $shift:literal) => {
        #[doc = concat!("Alignment config seeding `max_shift` to `", stringify!($shift), "`.")]
        #[derive(Debug, Deserialize, Serialize)]
        #[serde(default, rename_all = "kebab-case")]
        pub struct $name {
            pub enabled: bool,
            pub max_shift: NonZeroUsize,
            pub max_shift_policy: MaxAlignShiftPolicy,
        }

        impl Default for $name {
            fn default() -> Self {
                Self {
                    enabled: true,
                    max_shift: NonZeroUsize::new($shift)
                        .expect("alignment max-shift seed is non-zero"),
                    max_shift_policy: MaxAlignShiftPolicy::default(),
                }
            }
        }

        impl RuleToggle for $name {
            fn with_enabled(enabled: bool) -> Self {
                Self {
                    enabled,
                    ..Self::default()
                }
            }
        }
    };
}

alignment_config!(AlignImportsConfig, 16);

alignment_config!(AlignmentConfig, 8);

/// Configuration for the `alphabetize` rule. `docstring_entries`
/// gates the Google-style entry-section reorder pass, leaving the
/// AST-level sorts to apply on their own when set `false`.
#[derive(Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct AlphabetizeConfig {
    pub docstring_entries: bool,
    pub enabled: bool,
}

impl Default for AlphabetizeConfig {
    fn default() -> Self {
        Self {
            docstring_entries: true,
            enabled: true,
        }
    }
}

/// Configuration for the `bare_imports` rule.
#[derive(Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct BareImportsConfig {
    pub allow: Vec<String>,
    pub allow_aliased: bool,
    pub enabled: bool,
    pub max_attributes: usize,
}

impl Default for BareImportsConfig {
    fn default() -> Self {
        Self {
            allow: Vec::new(),
            allow_aliased: true,
            enabled: true,
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
/// `max_inline_args` caps the count threshold. A positive integer
/// enforces the cap. `false` disables the count trigger.
#[derive(Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct CallLayoutConfig {
    pub enabled: bool,
    #[serde(deserialize_with = "deserialize_max_inline_args")]
    pub max_inline_args: Option<NonZeroUsize>,
}

impl Default for CallLayoutConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_inline_args: NonZeroUsize::new(3),
        }
    }
}

/// Configuration for the `collection_layout` rule.
///
/// `max_atomics_per_line` and `max_inline_dict_entries` each take a
/// positive integer or `false`. The integer sets the cap, and `false`
/// disables it, leaving width as the only gate.
#[derive(Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct CollectionLayoutConfig {
    pub enabled: bool,
    #[serde(deserialize_with = "deserialize_max_atomics_per_line")]
    pub max_atomics_per_line: Option<NonZeroUsize>,
    #[serde(deserialize_with = "deserialize_max_inline_dict_entries")]
    pub max_inline_dict_entries: Option<NonZeroUsize>,
}

impl Default for CollectionLayoutConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_atomics_per_line: NonZeroUsize::new(8),
            max_inline_dict_entries: NonZeroUsize::new(3),
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

/// What to do when an alignment group's widest padding exceeds the
/// rule's `max-shift`.
///
/// `Split` greedily partitions the group so each contiguous
/// sub-group satisfies the cap, and each sub-group of size `>= 2`
/// aligns independently. `Drop` excludes the widest member(s) from
/// the padding calculation until the cap is satisfied, leaving those
/// members at their original spacing while neighbors align around
/// them.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MaxAlignShiftPolicy {
    Drop,
    #[default]
    Split,
}

/// Configuration for the `signature_layout` rule.
///
/// `max_inline_params` caps the count threshold. A positive integer
/// enforces the cap. `false` disables the count trigger.
#[derive(Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct SignatureLayoutConfig {
    pub enabled: bool,
    #[serde(deserialize_with = "deserialize_max_inline_params")]
    pub max_inline_params: Option<NonZeroUsize>,
}

impl Default for SignatureLayoutConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_inline_params: NonZeroUsize::new(4),
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
    AlphabetizeConfig,
    BareImportsConfig,
    CallLayoutConfig,
    CollectionLayoutConfig,
    ReassignedConstantsConfig,
    SignatureLayoutConfig,
    SingleUseVariablesConfig,
);
