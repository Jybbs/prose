//! String-parsing-surface tests for `Config::from_prose_toml_str` and
//! `Config::from_pyproject_str`.

use assert_matches::assert_matches;
use rstest::rstest;

use super::*;

fn assert_toml_error(toml: &str) {
    assert_matches!(Config::from_pyproject_str(toml), Err(ConfigError::Toml(_)));
}

/// Builds a `MaxShift::Cap` from a non-zero literal.
fn cap(n: usize) -> MaxShift {
    MaxShift::Cap(NonZeroUsize::new(n).expect("test cap is non-zero"))
}

#[test]
fn alphabetize_facet_false_in_sub_table_leaves_siblings_default() {
    let config =
        Config::from_pyproject_str("[tool.prose.rules.alphabetize]\ngroup-methods = false\n")
            .expect("parses");

    let rules = &config.rules.alphabetize;
    assert!(!rules.group_methods);
    assert!(rules.enabled);
    assert!(rules.sort_definitions);
    assert!(rules.sort_docstring_entries);
    assert!(rules.sort_dunder_lists);
}

#[test]
fn collection_layout_facet_false_in_sub_table_leaves_siblings_default() {
    let config =
        Config::from_pyproject_str("[tool.prose.rules.collection-layout]\ncollapse = false\n")
            .expect("parses");

    let rules = &config.rules.collection_layout;
    assert!(!rules.collapse);
    assert!(rules.enabled);
    assert!(rules.explode);
    assert!(rules.wrap_dict_entries);
    assert_eq!(rules.max_inline_dict_entries, NonZeroUsize::new(3));
}

#[test]
fn docstring_line_length_defaults_to_76_when_field_absent() {
    let config = Config::from_pyproject_str("[tool.prose]\n").expect("parses");

    assert_eq!(config.docstring_line_length, NonZeroUsize::new(76));
}

#[test]
fn docstring_line_length_explicit_override_takes_effect() {
    let config =
        Config::from_pyproject_str("[tool.prose]\ndocstring-line-length = 100\n").expect("parses");

    assert_eq!(config.docstring_line_length, NonZeroUsize::new(100));
}

#[test]
fn docstring_line_length_negative_returns_toml_error() {
    assert_toml_error("[tool.prose]\ndocstring-line-length = -1\n");
}

#[test]
fn docstring_line_length_zero_returns_toml_error() {
    assert_toml_error("[tool.prose]\ndocstring-line-length = 0\n");
}

#[test]
fn docstring_structured_policy_defaults_to_code_line_length_when_field_absent() {
    let config = Config::from_pyproject_str("[tool.prose]\n").expect("parses");

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
fn docstring_structured_policy_invalid_value_returns_toml_error() {
    assert_toml_error("[tool.prose]\ndocstring-structured-policy = \"nonsense\"\n");
}

#[test]
fn from_prose_toml_str_empty_returns_defaults() {
    let config = Config::from_prose_toml_str("").expect("parses");

    assert_eq!(config.code_line_length, NonZeroUsize::new(88));
    assert!(config.rules.align_equals.enabled);
}

#[test]
fn from_prose_toml_str_reads_bare_root_keys() {
    let config =
        Config::from_prose_toml_str("code-line-length = 120\n[rules]\nalphabetize = false\n")
            .expect("parses");

    assert_eq!(config.code_line_length, NonZeroUsize::new(120));
    assert!(!config.rules.alphabetize.enabled);
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
    let config = Config::from_pyproject_str("[tool.prose]\n").expect("parses");

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

#[test]
fn import_line_length_negative_returns_toml_error() {
    assert_toml_error("[tool.prose]\nimport-line-length = -1\n");
}

#[test]
fn import_line_length_true_returns_toml_error() {
    assert_toml_error("[tool.prose]\nimport-line-length = true\n");
}

#[test]
fn import_line_length_zero_returns_toml_error() {
    assert_toml_error("[tool.prose]\nimport-line-length = 0\n");
}

#[test]
fn import_width_uses_import_line_length_when_set() {
    let config =
        Config::from_pyproject_str("[tool.prose]\nimport-line-length = 100\n").expect("parses");

    assert_eq!(config.import_width(), 100);
}

#[test]
fn imports_first_party_defaults_to_empty_when_absent() {
    let config = Config::from_pyproject_str("[tool.prose]\n").expect("parses");

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
fn max_atomics_per_line_explicit_integer_takes_effect() {
    let config = Config::from_pyproject_str(
        "[tool.prose.rules.collection-layout]\nmax-atomics-per-line = 3\n",
    )
    .expect("parses");

    assert_eq!(
        config.rules.collection_layout.max_atomics_per_line,
        NonZeroUsize::new(3),
    );
}

#[test]
fn max_atomics_per_line_false_disables_cap() {
    let config = Config::from_pyproject_str(
        "[tool.prose.rules.collection-layout]\nmax-atomics-per-line = false\n",
    )
    .expect("parses");

    assert!(
        config
            .rules
            .collection_layout
            .max_atomics_per_line
            .is_none()
    );
}

#[test]
fn max_atomics_per_line_true_returns_toml_error() {
    assert_toml_error("[tool.prose.rules.collection-layout]\nmax-atomics-per-line = true\n");
}

#[test]
fn max_inline_dict_entries_explicit_integer_takes_effect() {
    let config = Config::from_pyproject_str(
        "[tool.prose.rules.collection-layout]\nmax-inline-dict-entries = 5\n",
    )
    .expect("parses");

    assert_eq!(
        config.rules.collection_layout.max_inline_dict_entries,
        NonZeroUsize::new(5),
    );
}

#[test]
fn max_inline_dict_entries_false_disables_count_trigger() {
    let config = Config::from_pyproject_str(
        "[tool.prose.rules.collection-layout]\nmax-inline-dict-entries = false\n",
    )
    .expect("parses");

    assert!(
        config
            .rules
            .collection_layout
            .max_inline_dict_entries
            .is_none()
    );
}

#[test]
fn max_inline_dict_entries_true_returns_toml_error() {
    assert_toml_error("[tool.prose.rules.collection-layout]\nmax-inline-dict-entries = true\n");
}

#[test]
fn max_inline_dict_entries_zero_returns_toml_error() {
    assert_toml_error("[tool.prose.rules.collection-layout]\nmax-inline-dict-entries = 0\n");
}

#[test]
fn max_inline_params_explicit_integer_takes_effect() {
    let config =
        Config::from_pyproject_str("[tool.prose.rules.signature-layout]\nmax-inline-params = 5\n")
            .expect("parses");

    assert_eq!(
        config.rules.signature_layout.max_inline_params,
        NonZeroUsize::new(5),
    );
}

#[test]
fn max_inline_params_false_disables_count_trigger() {
    let config = Config::from_pyproject_str(
        "[tool.prose.rules.signature-layout]\nmax-inline-params = false\n",
    )
    .expect("parses");

    assert!(config.rules.signature_layout.max_inline_params.is_none());
}

#[test]
fn max_inline_params_string_value_returns_toml_error() {
    assert_toml_error("[tool.prose.rules.signature-layout]\nmax-inline-params = \"off\"\n");
}

#[test]
fn max_inline_params_true_returns_toml_error() {
    assert_toml_error("[tool.prose.rules.signature-layout]\nmax-inline-params = true\n");
}

#[test]
fn max_inline_params_zero_returns_toml_error() {
    assert_toml_error("[tool.prose.rules.signature-layout]\nmax-inline-params = 0\n");
}

#[test]
fn max_shift_default_is_sixteen() {
    let config = Config::from_pyproject_str("[tool.prose]\n").expect("parses");

    assert_eq!(config.rules.align_equals.max_shift, cap(16));
}

#[test]
fn max_shift_negative_returns_toml_error() {
    assert_toml_error("[tool.prose.rules.align-equals]\nmax-shift = -1\n");
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
    let config = Config::from_pyproject_str(&format!(
        "[tool.prose.rules.align-equals]\nmax-shift = {value}\n"
    ))
    .expect("parses");
    let dumped = toml::to_string(&config).expect("Config serializes");
    let reparsed = Config::from_prose_toml_str(&dumped).expect("reparses");

    assert_eq!(
        reparsed.rules.align_equals.max_shift,
        config.rules.align_equals.max_shift,
    );
}

#[test]
fn max_shift_true_returns_toml_error() {
    assert_toml_error("[tool.prose.rules.align-equals]\nmax-shift = true\n");
}

#[test]
fn rules_bare_bool_false_leaves_other_knobs_default() {
    let config =
        Config::from_pyproject_str("[tool.prose.rules]\nalphabetize = false\n").expect("parses");

    assert!(!config.rules.alphabetize.enabled);
    assert!(config.rules.alphabetize.group_methods);
    assert!(config.rules.alphabetize.sort_definitions);
    assert!(config.rules.alphabetize.sort_docstring_entries);
    assert!(config.rules.alphabetize.sort_dunder_lists);
}

#[rstest]
#[case("false", false)]
#[case("true", true)]
fn rules_bare_bool_sets_enabled(#[case] literal: &str, #[case] expected: bool) {
    let config =
        Config::from_pyproject_str(&format!("[tool.prose.rules]\nalphabetize = {literal}\n"))
            .expect("parses");

    assert_eq!(config.rules.alphabetize.enabled, expected);
    assert!(config.rules.align_equals.enabled);
}

#[test]
fn rules_inline_table_compiles_regex_knob() {
    let config = Config::from_pyproject_str(
        "[tool.prose.rules]\nsingle-use-variables = { allow-pattern = \"^tmp_\" }\n",
    )
    .expect("parses");

    assert!(
        config
            .rules
            .single_use_variables
            .allow_pattern
            .is_match("tmp_x")
    );
    assert!(config.rules.single_use_variables.enabled);
}

#[test]
fn rules_inline_table_resolves_nested_max_inline_params() {
    let config = Config::from_pyproject_str(
        "[tool.prose.rules]\nsignature-layout = { max-inline-params = false }\n",
    )
    .expect("parses");

    assert!(config.rules.signature_layout.enabled);
    assert!(config.rules.signature_layout.max_inline_params.is_none());
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
fn rules_non_bool_non_table_value_returns_toml_error() {
    assert_toml_error("[tool.prose.rules]\nalign-equals = 5\n");
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
fn single_use_variables_explicit_allow_pattern_takes_effect() {
    let config = Config::from_pyproject_str(
        "[tool.prose.rules.single-use-variables]\nallow-pattern = \"^tmp_\"\n",
    )
    .expect("parses");

    assert!(
        config
            .rules
            .single_use_variables
            .allow_pattern
            .is_match("tmp_x")
    );
    assert!(
        !config
            .rules
            .single_use_variables
            .allow_pattern
            .is_match("xtmp_")
    );
}

#[test]
fn single_use_variables_invalid_allow_pattern_returns_toml_error() {
    assert_toml_error("[tool.prose.rules.single-use-variables]\nallow-pattern = \"[unclosed\"\n");
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
    let config = Config::from_pyproject_str("[tool.prose]\n").expect("parses");

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
fn target_version_extra_period_returns_toml_error() {
    assert_toml_error("[tool.prose]\ntarget-version = \"3.14.0\"\n");
}

#[test]
fn target_version_invalid_value_returns_toml_error() {
    assert_toml_error("[tool.prose]\ntarget-version = \"py310\"\n");
}
