//! Deserializes the prose table and the shapes serde cannot read on its
//! own, covering the bool-or-table rule reader, the optional-cap
//! round-trips, and the notices the table raises.

use std::{fmt, marker::PhantomData, num::NonZeroUsize};

use ruff_python_ast::PythonVersion;
use serde::{
    Deserialize, Deserializer, Serializer,
    de::{IntoDeserializer, MapAccess, Visitor, value::MapAccessDeserializer},
};

use super::{Config, ConfigError, schema::RuleToggle};

/// A notice the prose table raises while it deserializes.
pub(crate) enum TableNotice<'a> {
    /// A `target-version` outside the versions the parser models.
    UnmodeledTarget(PythonVersion),
    /// An unrecognized key, carrying its dotted path.
    UnknownKey(&'a str),
}

impl fmt::Display for TableNotice<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnmodeledTarget(version) => write!(
                f,
                "warning: `target-version` {version} in [tool.prose] names no Python version \
                 the parser models, which run from {} through {}",
                PythonVersion::lowest(),
                PythonVersion::latest(),
            ),
            Self::UnknownKey(key) => write!(f, "warning: unknown key `{key}` in [tool.prose]"),
        }
    }
}

/// Deserializes a cap of integer type `T`, or `false` lifting it to
/// `None`. `true` is rejected with `on_true`.
pub(super) fn deserialize_cap_or_false<'de, T, D>(
    deserializer: D,
    on_true: &'static str,
) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Value<T> {
        Cap(T),
        Off(bool),
    }
    match Value::deserialize(deserializer)? {
        Value::Cap(n) => Ok(Some(n)),
        Value::Off(false) => Ok(None),
        Value::Off(true) => Err(serde::de::Error::custom(on_true)),
    }
}

/// Deserializes an optional cap a positive integer sets and `false`
/// disables. Shared by the `InlineBudget` layout caps and the
/// top-level `import-line-length` key.
pub(super) fn deserialize_optional_cap<'de, D>(
    deserializer: D,
) -> Result<Option<NonZeroUsize>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_cap_or_false(
        deserializer,
        "expected a positive integer or `false`, not `true`",
    )
}

/// Deserializes `table` into a config, raising a notice through
/// `on_notice` for each key it does not recognize and for a
/// `target-version` outside the versions the parser models.
pub(super) fn deserialize_prose(
    table: toml::Table,
    on_notice: &mut dyn FnMut(TableNotice<'_>),
) -> Result<Config, ConfigError> {
    let config: Config =
        serde_ignored::deserialize(toml::Value::Table(table).into_deserializer(), |path| {
            on_notice(TableNotice::UnknownKey(&path.to_string()))
        })?;
    if let Some(version) = config.target_version
        && !PythonVersion::iter().any(|modeled| modeled == version)
    {
        on_notice(TableNotice::UnmodeledTarget(version));
    }
    Ok(config)
}

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

/// Serializes an optional cap as its integer, or `false` when uncapped.
pub(super) fn serialize_optional_cap<S: Serializer>(
    cap: &Option<NonZeroUsize>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match cap {
        Some(n) => serializer.serialize_u64(n.get() as u64),
        None => serializer.serialize_bool(false),
    }
}
