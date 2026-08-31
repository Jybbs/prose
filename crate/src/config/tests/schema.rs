//! Schema-shape tests for the `JsonSchema` derive across the `Config`
//! tree and the hand-written impls for the custom-serde spellings.

use rstest::{fixture, rstest};
use schemars::schema_for;
use serde_json::{Value, json};

use crate::config::*;
use crate::pipeline::Pipeline;

#[fixture]
fn schema() -> Value {
    schema_for!(Config).to_value()
}

#[rstest]
fn allow_pattern_reads_a_glob_string_with_the_config_default(schema: Value) {
    let allow_pattern = &schema["$defs"]["InlinableBindingsConfig"]["properties"]["allow-pattern"];
    assert_eq!(allow_pattern["type"], json!("string"));
    assert_eq!(allow_pattern["default"], json!("_*"));
}

#[rstest]
#[case::import_line_length("properties", "import-line-length", 1)]
#[case::inline_budget("$defs", "InlineBudget", 1)]
#[case::max_shift("$defs", "MaxShift", 0)]
fn cap_schemas_accept_an_integer_or_false(
    schema: Value,
    #[case] section: &str,
    #[case] key: &str,
    #[case] minimum: u64,
) {
    let any_of = &schema[section][key]["anyOf"];

    assert_eq!(any_of[0]["minimum"], json!(minimum));
    assert_eq!(any_of[1], json!({ "const": false }));
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
