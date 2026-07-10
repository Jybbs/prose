//! Schema-shape tests for the `JsonSchema` derive across the `Config`
//! tree and the hand-written impls for the custom-serde spellings.

use rstest::{fixture, rstest};
use schemars::schema_for;
use serde_json::{Value, json};

use super::*;
use crate::pipeline::Pipeline;

#[fixture]
fn schema() -> Value {
    serde_json::to_value(schema_for!(Config)).expect("schema serializes")
}

#[rstest]
fn allow_pattern_reads_a_regex_string_with_the_config_default(schema: Value) {
    assert_eq!(
        schema["$defs"]["SingleUseVariablesConfig"]["properties"]["allow-pattern"],
        json!({ "type": "string", "format": "regex", "default": "^_" }),
    );
}

#[rstest]
#[case::code_line_length("code-line-length", json!(88))]
#[case::docstring_line_length("docstring-line-length", json!(76))]
#[case::import_line_length("import-line-length", json!(120))]
fn defaults_mirror_config_default(schema: Value, #[case] key: &str, #[case] expected: Value) {
    assert_eq!(schema["properties"][key]["default"], expected);
}

#[rstest]
fn docstring_structured_policy_enumerates_kebab_variants(schema: Value) {
    assert_eq!(
        schema["$defs"]["DocstringStructuredPolicy"]["enum"],
        json!(["code-line-length", "docstring-line-length"]),
    );
}

#[rstest]
fn import_line_length_accepts_a_positive_integer_or_false(schema: Value) {
    let any_of = &schema["properties"]["import-line-length"]["anyOf"];

    assert_eq!(any_of[0]["minimum"], json!(1));
    assert_eq!(any_of[1], json!({ "const": false }));
}

#[rstest]
fn inline_budget_accepts_a_positive_integer_or_false(schema: Value) {
    let any_of = &schema["$defs"]["InlineBudget"]["anyOf"];

    assert_eq!(any_of[0]["minimum"], json!(1));
    assert_eq!(any_of[1], json!({ "const": false }));
}

#[rstest]
fn max_shift_accepts_a_non_negative_integer_or_false(schema: Value) {
    let any_of = &schema["$defs"]["MaxShift"]["anyOf"];

    assert_eq!(any_of[0]["minimum"], json!(0));
    assert_eq!(any_of[1], json!({ "const": false }));
}

#[rstest]
fn rule_entry_accepts_a_bool_toggle_or_a_sub_table(schema: Value) {
    let entry = &schema["$defs"]["RuleConfigs"]["properties"]["align-equals"];

    assert_eq!(entry["anyOf"][0], json!({ "type": "boolean" }));
    assert_eq!(
        entry["anyOf"][1],
        json!({ "$ref": "#/$defs/AlignmentConfig" }),
    );
    assert_eq!(entry["default"]["max-shift"], json!(16));
}

#[rstest]
fn rules_schema_carries_every_registered_slug(schema: Value) {
    let properties = schema["$defs"]["RuleConfigs"]["properties"]
        .as_object()
        .expect("rules schema is an object");

    assert_eq!(properties.len(), Pipeline::known_ids().len());
    for id in Pipeline::known_ids() {
        assert!(
            properties.contains_key(id.as_str()),
            "missing rule entry `{id}`",
        );
    }
}

#[rstest]
fn schema_document_snapshot(schema: Value) {
    insta::assert_snapshot!(serde_json::to_string_pretty(&schema).expect("renders"));
}

#[rstest]
fn target_version_reads_the_upstream_python_version_schema(schema: Value) {
    assert_eq!(
        schema["properties"]["target-version"]["anyOf"][0],
        json!({ "$ref": "#/$defs/PythonVersion" }),
    );
    assert!(schema["$defs"]["PythonVersion"]["anyOf"].is_array());
}

#[rstest]
fn top_level_properties_are_the_serialized_config_keys(schema: Value) {
    let config = serde_json::to_value(Config::default()).expect("Config serializes");

    assert_eq!(
        schema["properties"]
            .as_object()
            .expect("an object")
            .keys()
            .collect::<Vec<_>>(),
        config
            .as_object()
            .expect("an object")
            .keys()
            .collect::<Vec<_>>(),
    );
}
