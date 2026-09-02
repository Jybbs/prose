//! Tests over the registry constructors `Pipeline::with_defaults` and
//! `Pipeline::with_filters`.

use super::*;

#[test]
fn known_ids_matches_with_defaults_registration() {
    let config = Config::default();
    let pipeline = Pipeline::with_defaults(&config);
    let mut registered = registered_slugs(&pipeline);
    registered.sort_unstable();
    let mut known: Vec<&'static str> = Pipeline::known_ids().iter().map(RuleId::as_str).collect();
    known.sort_unstable();
    assert_eq!(registered, known);
}

#[test]
fn pipeline_is_send_and_sync() {
    assert_send_sync::<Pipeline>();
}

#[test]
fn with_defaults_registers_enabled_rules() {
    let config = Config::default();
    let pipeline = Pipeline::with_defaults(&config);
    assert_eq!(pipeline.len(), Pipeline::known_ids().len());
}

#[test]
fn with_defaults_respects_rule_toggles() {
    let disabled = Pipeline::known_ids()
        .iter()
        .map(|id| format!("{id} = false"))
        .join("\n");
    let config: Config = toml::from_str(&format!("[rules]\n{disabled}\n"))
        .expect("every registered slug parses as a rule toggle");

    assert!(Pipeline::with_defaults(&config).is_empty());
}

#[test]
fn with_filters_ignore_subtracts_from_configured_set() {
    let ignore = [AlignEquals::SLUG, AlphabetizeSiblings::SLUG];
    let pipeline = Pipeline::with_filters(&Config::default(), &[], &ignore);
    let slugs = registered_slugs(&pipeline);
    assert_eq!(slugs.len(), Pipeline::known_ids().len() - ignore.len());
    assert!(!slugs.contains(&AlignEquals::SLUG.as_str()));
    assert!(!slugs.contains(&AlphabetizeSiblings::SLUG.as_str()));
}

#[test]
fn with_filters_select_minus_ignore_drops_overlap() {
    let pipeline = Pipeline::with_filters(
        &Config::default(),
        &[AlignEquals::SLUG, AlignColons::SLUG],
        &[AlignEquals::SLUG],
    );
    assert_eq!(registered_slugs(&pipeline), ["align-colons"]);
}

#[test]
fn with_filters_select_overrides_disabled_config() {
    let mut config = Config::default();
    config.rules.align_equals.enabled = false;

    let pipeline = Pipeline::with_filters(&config, &[AlignEquals::SLUG], &[]);
    assert_eq!(registered_slugs(&pipeline), ["align-equals"]);
}

#[test]
fn with_filters_select_with_default_config_restricts_to_listed_rules() {
    let pipeline = Pipeline::with_filters(&Config::default(), &[AlignEquals::SLUG], &[]);
    assert_eq!(registered_slugs(&pipeline), ["align-equals"]);
}
