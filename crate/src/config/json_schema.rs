//! `JsonSchema` helpers for the custom-serde config spellings: the
//! bool-or-table rule entry and the optional cap.

use std::num::NonZeroUsize;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::Serialize;

/// Schema for a cap of integer type `T`, or `false` lifting it.
pub(super) fn cap_or_false_schema<T: JsonSchema>(generator: &mut SchemaGenerator) -> Schema {
    json_schema!({
        "anyOf": [generator.subschema_for::<T>(), { "const": false }],
    })
}

/// Schema for an optional cap a positive integer sets and `false`
/// lifts, the shape `deserialize_optional_cap` reads.
pub(super) fn optional_cap_schema(generator: &mut SchemaGenerator) -> Schema {
    cap_or_false_schema::<NonZeroUsize>(generator)
}

/// Schema for one `[tool.prose.rules]` entry, a bare bool toggle or the
/// rule's sub-table, the shape `deserialize_rule` reads.
pub(crate) fn rule_schema<T>(generator: &mut SchemaGenerator) -> Schema
where
    T: Default + JsonSchema + Serialize,
{
    json_schema!({
        "anyOf": [{ "type": "boolean" }, generator.subschema_for::<T>()],
        "default": T::default(),
    })
}
