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
/// unambiguous. Shared by the `InlineBudget` layout caps and the
/// top-level `import-line-length` key.
pub(super) fn deserialize_optional_cap<'de, D>(
    deserializer: D,
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
        Value::Off(true) => Err(serde::de::Error::custom(
            "expected a positive integer or `false`, not `true`",
        )),
    }
}

pub(super) fn deserialize_prose<F>(
    table: toml::Table,
    on_notice: &mut F,
) -> Result<Config, ConfigError>
where
    F: FnMut(ConfigNotice<'_>),
{
    Ok(serde_ignored::deserialize(
        toml::Value::Table(table).into_deserializer(),
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
