//! String-parsing-surface tests for `Config::from_prose_toml_str` and
//! `Config::from_pyproject_str`.

use std::assert_matches;
use std::fmt::Debug;

use rstest::rstest;

use crate::config::*;

/// Asserts a parsed config survives a TOML dump and re-parse,
/// comparing through `project`.
fn assert_round_trips<T: Debug + PartialEq>(pyproject: &str, project: impl Fn(&Config) -> T) {
    let config = Config::from_pyproject_str(pyproject).expect("parses");
    let dumped = toml::to_string(&config).expect("Config serializes");
    let reparsed = Config::from_prose_toml_str(&dumped).expect("reparses");

    assert_eq!(project(&reparsed), project(&config));
}

fn assert_toml_error(toml: &str) {
    assert_matches!(Config::from_pyproject_str(toml), Err(ConfigError::Toml(_)));
}

/// Builds a `MaxShift::Cap` from a non-zero literal.
fn cap(n: usize) -> MaxShift {
    MaxShift::Cap(NonZeroUsize::new(n).expect("test cap is non-zero"))
}

fn max_args_cap(config: &Config) -> Option<usize> {
    config.rules.reflow_calls.max_args.cap()
}

fn max_atomics_cap(config: &Config) -> Option<usize> {
    config.rules.reflow_collections.max_atomics.cap()
}

fn max_dict_entries_cap(config: &Config) -> Option<usize> {
    config.rules.reflow_collections.max_dict_entries.cap()
}

fn max_links_cap(config: &Config) -> Option<usize> {
    config.rules.stack_method_chains.max_links.cap()
}

fn max_params_cap(config: &Config) -> Option<usize> {
    config.rules.reflow_signatures.max_params.cap()
}

/// A config parsed from an empty `[tool.prose]` table, every key at its
/// default.
fn parsed_defaults() -> Config {
    Config::from_pyproject_str("[tool.prose]\n").expect("parses")
}

#[test]
fn alphabetize_siblings_facet_false_in_sub_table_leaves_siblings_default() {
    let config = Config::from_pyproject_str(
        "[tool.prose.rules.alphabetize-siblings]\ngroup-methods = false\n",
    )
    .expect("parses");

    let rules = &config.rules.alphabetize_siblings;
    assert!(!rules.group_methods);
    assert!(rules.enabled);
    assert!(rules.sort_definitions);
    assert!(rules.sort_dict_keys);
    assert!(rules.sort_docstring_entries);
    assert!(rules.sort_dunder_lists);
}

#[test]
fn docstring_line_length_defaults_to_76_when_field_absent() {
    let config = parsed_defaults();

    assert_eq!(config.docstring_line_length, NonZeroUsize::new(76));
}

#[test]
fn docstring_line_length_explicit_override_takes_effect() {
    let config =
        Config::from_pyproject_str("[tool.prose]\ndocstring-line-length = 100\n").expect("parses");

    assert_eq!(config.docstring_line_length, NonZeroUsize::new(100));
}

#[test]
fn docstring_structured_policy_defaults_to_code_line_length_when_field_absent() {
    let config = parsed_defaults();

    assert_eq!(
        config.docstring_structured_policy,
        DocstringStructuredPolicy::CodeLineLength
    );
}

#[test]
fn docstring_structured_policy_explicit_override_to_docstring_line_length() {
    let config = Config::from_pyproject_str(
        "[tool.prose]\ndocstring-structured-policy = \"docstring-line-length\"\n",
    )
    .expect("parses");

    assert_eq!(
        config.docstring_structured_policy,
        DocstringStructuredPolicy::DocstringLineLength
    );
}

#[test]
fn from_prose_toml_str_empty_returns_defaults() {
    let config = Config::from_prose_toml_str("").expect("parses");

    assert_eq!(config.code_line_length, NonZeroUsize::new(88));
    assert!(config.rules.align_equals.enabled);
}

#[test]
fn from_prose_toml_str_reads_bare_root_keys() {
    let config = Config::from_prose_toml_str(
        "code-line-length = 120\n[rules]\nalphabetize-siblings = false\n",
    )
    .expect("parses");

    assert_eq!(config.code_line_length, NonZeroUsize::new(120));
    assert!(!config.rules.alphabetize_siblings.enabled);
}

#[test]
fn from_pyproject_str_with_unknown_key_warns_and_returns_config() {
    let config = Config::from_pyproject_str(
        "[tool.prose]\ncode-line-length = 100\nunknown-future-key = 1\n",
    )
    .expect("parses");

    assert_eq!(config.code_line_length, NonZeroUsize::new(100));
}

#[test]
fn import_line_length_defaults_to_120_when_field_absent() {
    let config = parsed_defaults();

    assert_eq!(config.import_line_length, NonZeroUsize::new(120));
}

#[test]
fn import_line_length_explicit_override_takes_effect() {
    let config =
        Config::from_pyproject_str("[tool.prose]\nimport-line-length = 100\n").expect("parses");

    assert_eq!(config.import_line_length, NonZeroUsize::new(100));
}

#[test]
fn import_line_length_false_falls_back_to_code_line_length() {
    let config =
        Config::from_pyproject_str("[tool.prose]\nimport-line-length = false\n").expect("parses");

    assert!(config.import_line_length.is_none());
    assert_eq!(config.import_width(), config.code_width());
}

#[rstest]
#[case("100")]
#[case("false")]
fn import_line_length_round_trips_through_toml(#[case] value: &str) {
    assert_round_trips(
        &format!("[tool.prose]\nimport-line-length = {value}\n"),
        |c| c.import_line_length,
    );
}

#[test]
fn import_width_uses_import_line_length_when_set() {
    let config =
        Config::from_pyproject_str("[tool.prose]\nimport-line-length = 100\n").expect("parses");

    assert_eq!(config.import_width(), 100);
}

#[test]
fn imports_first_party_defaults_to_empty_when_absent() {
    let config = parsed_defaults();

    assert!(config.imports.first_party.is_empty());
}

#[test]
fn imports_first_party_reads_kebab_case_list() {
    let config =
        Config::from_pyproject_str("[tool.prose.imports]\nfirst-party = [\"myapp\", \"acme\"]\n")
            .expect("parses");

    assert_eq!(config.imports.first_party, ["myapp", "acme"]);
}

#[test]
fn inlinable_bindings_explicit_allow_pattern_takes_effect() {
    let config = Config::from_pyproject_str(
        "[tool.prose.rules.inlinable-bindings]\nallow-pattern = \"tmp_*\"\n",
    )
    .expect("parses");

    assert!(
        config
            .rules
            .inlinable_bindings
            .allow_pattern
            .matches("tmp_x")
    );
    assert!(
        !config
            .rules
            .inlinable_bindings
            .allow_pattern
            .matches("xtmp_")
    );
}

#[rstest]
#[case::max_args("reflow-calls", "max-args", max_args_cap)]
#[case::max_atomics("reflow-collections", "max-atomics", max_atomics_cap)]
#[case::max_dict_entries("reflow-collections", "max-dict-entries", max_dict_entries_cap)]
#[case::max_links("stack-method-chains", "max-links", max_links_cap)]
#[case::max_params("reflow-signatures", "max-params", max_params_cap)]
fn inline_budget_reads_integer_and_false(
    #[case] table: &str,
    #[case] key: &str,
    #[case] cap_of: fn(&Config) -> Option<usize>,
) {
    let set = Config::from_pyproject_str(&format!("[tool.prose.rules.{table}]\n{key} = 5\n"))
        .expect("parses");
    assert_eq!(cap_of(&set), Some(5));

    let off = Config::from_pyproject_str(&format!("[tool.prose.rules.{table}]\n{key} = false\n"))
        .expect("parses");
    assert_eq!(cap_of(&off), None);
}

#[rstest]
#[case::max_args("reflow-calls", "max-args")]
#[case::max_atomics("reflow-collections", "max-atomics")]
#[case::max_dict_entries("reflow-collections", "max-dict-entries")]
#[case::max_links("stack-method-chains", "max-links")]
#[case::max_params("reflow-signatures", "max-params")]
fn inline_budget_rejects_non_cap_value(
    #[case] table: &str,
    #[case] key: &str,
    #[values("true", "0", "\"off\"")] bad: &str,
) {
    assert_toml_error(&format!("[tool.prose.rules.{table}]\n{key} = {bad}\n"));
}

#[rstest]
#[case::max_args("reflow-calls", "max-args", max_args_cap)]
#[case::max_atomics("reflow-collections", "max-atomics", max_atomics_cap)]
#[case::max_dict_entries("reflow-collections", "max-dict-entries", max_dict_entries_cap)]
#[case::max_links("stack-method-chains", "max-links", max_links_cap)]
#[case::max_params("reflow-signatures", "max-params", max_params_cap)]
fn inline_budget_round_trips_through_toml(
    #[case] table: &str,
    #[case] key: &str,
    #[case] cap_of: fn(&Config) -> Option<usize>,
    #[values("5", "false")] value: &str,
) {
    assert_round_trips(
        &format!("[tool.prose.rules.{table}]\n{key} = {value}\n"),
        cap_of,
    );
}

#[rstest]
#[case::docstring_line_length_negative("[tool.prose]\ndocstring-line-length = -1\n")]
#[case::docstring_line_length_zero("[tool.prose]\ndocstring-line-length = 0\n")]
#[case::docstring_structured_policy("[tool.prose]\ndocstring-structured-policy = \"nonsense\"\n")]
#[case::import_line_length_negative("[tool.prose]\nimport-line-length = -1\n")]
#[case::import_line_length_true("[tool.prose]\nimport-line-length = true\n")]
#[case::import_line_length_zero("[tool.prose]\nimport-line-length = 0\n")]
#[case::inlinable_bindings_allow_pattern(
    "[tool.prose.rules.inlinable-bindings]\nallow-pattern = \"[unclosed\"\n"
)]
#[case::max_shift_negative("[tool.prose.rules.align-equals]\nmax-shift = -1\n")]
#[case::max_shift_true("[tool.prose.rules.align-equals]\nmax-shift = true\n")]
#[case::rules_non_bool_non_table("[tool.prose.rules]\nalign-equals = 5\n")]
#[case::target_version_extra_period("[tool.prose]\ntarget-version = \"3.14.0\"\n")]
#[case::target_version_invalid("[tool.prose]\ntarget-version = \"py310\"\n")]
fn invalid_value_returns_toml_error(#[case] toml: &str) {
    assert_toml_error(toml);
}

#[test]
fn max_shift_default_is_sixteen() {
    let config = parsed_defaults();

    assert_eq!(config.rules.align_equals.max_shift, cap(16));
}

#[rstest]
#[case("4", cap(4))]
#[case("false", MaxShift::Unlimited)]
#[case("0", MaxShift::NoShift)]
fn max_shift_reads_each_value_form(#[case] value: &str, #[case] expected: MaxShift) {
    let config = Config::from_pyproject_str(&format!(
        "[tool.prose.rules.align-equals]\nmax-shift = {value}\n"
    ))
    .expect("parses");

    assert_eq!(config.rules.align_equals.max_shift, expected);
}

#[rstest]
#[case("0")]
#[case("4")]
#[case("false")]
fn max_shift_round_trips_through_toml(#[case] value: &str) {
    assert_round_trips(
        &format!("[tool.prose.rules.align-equals]\nmax-shift = {value}\n"),
        |c| c.rules.align_equals.max_shift,
    );
}

#[test]
fn reflow_collections_facet_false_in_sub_table_leaves_siblings_default() {
    let config = Config::from_pyproject_str(
        "[tool.prose.rules.reflow-collections]\nkeep-multiline-literals = false\n",
    )
    .expect("parses");

    let rules = &config.rules.reflow_collections;
    assert!(!rules.keep_multiline_literals);
    assert!(rules.enabled);
    assert!(rules.explode);
    assert!(rules.wrap_dict_entries);
    assert_eq!(rules.max_dict_entries.cap(), Some(3));
}

#[test]
fn rules_bare_bool_false_leaves_other_knobs_default() {
    let config = Config::from_pyproject_str("[tool.prose.rules]\nalphabetize-siblings = false\n")
        .expect("parses");

    assert!(!config.rules.alphabetize_siblings.enabled);
    assert!(config.rules.alphabetize_siblings.group_methods);
    assert!(config.rules.alphabetize_siblings.sort_definitions);
    assert!(config.rules.alphabetize_siblings.sort_dict_keys);
    assert!(config.rules.alphabetize_siblings.sort_docstring_entries);
    assert!(config.rules.alphabetize_siblings.sort_dunder_lists);
}

#[rstest]
#[case("false", false)]
#[case("true", true)]
fn rules_bare_bool_sets_enabled(#[case] literal: &str, #[case] expected: bool) {
    let config = Config::from_pyproject_str(&format!(
        "[tool.prose.rules]\nalphabetize-siblings = {literal}\n"
    ))
    .expect("parses");

    assert_eq!(config.rules.alphabetize_siblings.enabled, expected);
    assert!(config.rules.align_equals.enabled);
}

#[test]
fn rules_inline_table_compiles_regex_knob() {
    let config = Config::from_pyproject_str(
        "[tool.prose.rules]\ninlinable-bindings = { allow-pattern = \"tmp_*\" }\n",
    )
    .expect("parses");

    assert!(
        config
            .rules
            .inlinable_bindings
            .allow_pattern
            .matches("tmp_x")
    );
    assert!(config.rules.inlinable_bindings.enabled);
}

#[test]
fn rules_inline_table_resolves_nested_max_params() {
    let config = Config::from_pyproject_str(
        "[tool.prose.rules]\nreflow-signatures = { max-params = false }\n",
    )
    .expect("parses");

    assert!(config.rules.reflow_signatures.enabled);
    assert!(config.rules.reflow_signatures.max_params.cap().is_none());
}

#[test]
fn rules_inline_table_sets_knob_and_stays_enabled() {
    let config =
        Config::from_pyproject_str("[tool.prose.rules]\nalign-equals = { max-shift = 4 }\n")
            .expect("parses");

    assert!(config.rules.align_equals.enabled);
    assert_eq!(config.rules.align_equals.max_shift, cap(4));
}

#[test]
fn rules_subtable_form_still_parses() {
    let config = Config::from_pyproject_str(
        "[tool.prose.rules.align-equals]\nenabled = false\nmax-shift = 4\n",
    )
    .expect("parses");

    assert!(!config.rules.align_equals.enabled);
    assert_eq!(config.rules.align_equals.max_shift, cap(4));
}

#[test]
fn target_version_accepts_unrecognized_minor() {
    let config =
        Config::from_pyproject_str("[tool.prose]\ntarget-version = \"3.99\"\n").expect("parses");

    assert_eq!(
        config.target_version,
        Some(PythonVersion {
            major: 3,
            minor: 99
        })
    );
}

#[test]
fn target_version_defaults_to_none_when_field_absent() {
    let config = parsed_defaults();

    assert_eq!(config.target_version, None);
}

#[test]
fn target_version_every_variant_round_trips_through_serde() {
    for version in PythonVersion::iter() {
        let toml = format!("[tool.prose]\ntarget-version = \"{version}\"\n");
        let config = Config::from_pyproject_str(&toml).expect("parses");

        assert_eq!(config.target_version, Some(version));
    }
}

#[test]
fn to_changed_toml_is_empty_for_a_config_on_the_defaults() {
    assert_eq!(Config::default().to_changed_toml(), "");
}

#[test]
fn to_changed_toml_keeps_a_nested_key_set_away_from_its_default() {
    let config = Config::from_prose_toml_str("[rules]\nalign-equals = false\n").expect("parses");

    assert_eq!(
        config.to_changed_toml(),
        "[rules.align-equals]\nenabled = false\n",
    );
}

#[test]
fn to_changed_toml_keeps_a_top_level_key_set_away_from_its_default() {
    let config = Config::from_prose_toml_str("code-line-length = 100\n").expect("parses");

    assert_eq!(config.to_changed_toml(), "code-line-length = 100\n");
}

#[test]
fn uncapped_import_line_length_serializes_as_false() {
    let config =
        Config::from_pyproject_str("[tool.prose]\nimport-line-length = false\n").expect("parses");

    assert!(config.to_toml().contains("import-line-length = false"));
}
