//! Serde `deserialize_with` helpers: the bool-or-table rule reader,
//! the optional-cap parser, and the regex round-trip.

use std::{fmt, marker::PhantomData, num::NonZeroUsize};

use regex_lite::Regex;
use serde::{
    Deserialize, Deserializer, Serializer,
    de::{IntoDeserializer, MapAccess, Visitor, value::MapAccessDeserializer},
};

use super::load::ConfigNotice;
use super::schema::RuleToggle;
use super::{Config, ConfigError};

/// Resolves a rule's config from either a bare bool toggle or a
/// sub-table. `deserialize_any` dispatches on the TOML value so the
/// sub-table arm forwards a live map, preserving `serde_ignored`'s
/// unknown-key tracking inside the table.
pub(crate) fn deserialize_rule<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: RuleToggle + Deserialize<'de>,
{
    struct RuleVisitor<T>(PhantomData<T>);

    impl<'de, T: RuleToggle + Deserialize<'de>> Visitor<'de> for RuleVisitor<T> {
        type Value = T;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a boolean toggle or a rule sub-table")
        }

        fn visit_bool<E: serde::de::Error>(self, enabled: bool) -> Result<T, E> {
            Ok(T::with_enabled(enabled))
        }

        fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<T, A::Error> {
            T::deserialize(MapAccessDeserializer::new(map))
        }
    }

    deserializer.deserialize_any(RuleVisitor(PhantomData))
}

/// Deserializes an optional cap a positive integer sets and `false`
/// disables. `true` is rejected so the disable spelling stays
/// unambiguous.
fn deserialize_optional_cap<'de, D>(
    deserializer: D,
    knob: &str,
) -> Result<Option<NonZeroUsize>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Value {
        Cap(NonZeroUsize),
        Off(bool),
    }
    match Value::deserialize(deserializer)? {
        Value::Cap(n) => Ok(Some(n)),
        Value::Off(false) => Ok(None),
        Value::Off(true) => Err(serde::de::Error::custom(format!(
            "`{knob}` accepts a positive integer or `false`, not `true`"
        ))),
    }
}

/// Generates a named `deserialize_with` target forwarding to
/// [`deserialize_optional_cap`] with the knob's kebab-case name.
macro_rules! optional_cap {
    ($fn_name:ident, $knob:literal) => {
        pub(super) fn $fn_name<'de, D>(deserializer: D) -> Result<Option<NonZeroUsize>, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserialize_optional_cap(deserializer, $knob)
        }
    };
}

optional_cap!(deserialize_import_line_length, "import-line-length");

optional_cap!(deserialize_max_atomics_per_line, "max-atomics-per-line");

optional_cap!(deserialize_max_inline_args, "max-inline-args");

optional_cap!(
    deserialize_max_inline_dict_entries,
    "max-inline-dict-entries"
);

optional_cap!(deserialize_max_inline_params, "max-inline-params");

pub(super) fn deserialize_prose<F>(
    value: toml::Value,
    on_notice: &mut F,
) -> Result<Config, ConfigError>
where
    F: FnMut(ConfigNotice<'_>),
{
    Ok(serde_ignored::deserialize(
        value.into_deserializer(),
        |path| {
            on_notice(ConfigNotice::UnknownKey(&path.to_string()));
        },
    )?)
}

pub(super) fn deserialize_regex<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Regex, D::Error> {
    let pattern = String::deserialize(deserializer)?;
    Regex::new(&pattern).map_err(serde::de::Error::custom)
}

pub(super) fn serialize_regex<S: Serializer>(
    regex: &Regex,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(regex.as_str())
}
