//! Snapshot tests for `prose.toml` config parsing.
//!
//! Each `tests/fixtures/config/*/input.toml` file is a self-contained
//! `prose.toml` document. The harness parses it through
//! `Config::from_prose_toml_str` and snapshots the resulting `Config`
//! debug representation, so a regression in any default, rename, or
//! field addition surfaces as a snapshot diff rather than slipping
//! past spot-checked assertions.
//!
//! The `every_knob_overridden` case additionally answers to a guard
//! rather than to its snapshot alone, holding the fixture to the name
//! it carries as the config surface grows.

mod common;

use std::collections::BTreeMap;

use prose::config::Config;

const EVERY_KNOB_OVERRIDDEN: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/config/every_knob_overridden/input.toml"
);

#[test]
fn every_knob_overridden_leaves_no_key_at_its_default() {
    let toml = fs_err::read_to_string(EVERY_KNOB_OVERRIDDEN).expect("fixture reads");
    let overridden = leaves(&Config::from_prose_toml_str(&toml).expect("fixture parses"));
    let held: Vec<String> = leaves(&Config::default())
        .into_iter()
        .filter(|(key, value)| overridden.get(key) == Some(value))
        .map(|(key, _)| key)
        .collect();

    assert!(
        held.is_empty(),
        "`every_knob_overridden` still carries the default at {held:#?}",
    );
}

#[test]
fn fixtures() {
    insta::glob!("fixtures/config/*/input.toml", |path| {
        let case = common::case_name(path);
        let toml = fs_err::read_to_string(path).expect("fixture reads");
        let config = Config::from_prose_toml_str(&toml).expect("fixture parses");

        let reparsed = Config::from_prose_toml_str(&toml).expect("fixture re-parses");
        let (a, b) = (format!("{config:#?}"), format!("{reparsed:#?}"));
        assert!(
            a == b,
            "config parsing not deterministic for `{case}`:\n{}",
            common::unified_diff(&a, &b),
        );

        common::in_snapshot_dir(path, || {
            insta::assert_debug_snapshot!("config", config);
        });
    });
}

/// Every scalar `config` resolves to, keyed by the dotted path reaching
/// it. A rule written as a bare bool resolves through its defaults, so
/// the map holds one entry per knob whatever spelling the document took.
fn leaves(config: &Config) -> BTreeMap<String, String> {
    fn walk(value: &toml::Value, path: &str, out: &mut BTreeMap<String, String>) {
        let toml::Value::Table(table) = value else {
            out.insert(path.to_owned(), value.to_string());
            return;
        };
        for (key, nested) in table {
            let reached = if path.is_empty() {
                key.clone()
            } else {
                format!("{path}.{key}")
            };
            walk(nested, &reached, out);
        }
    }

    let mut out = BTreeMap::new();
    walk(
        &toml::Value::try_from(config).expect("config serializes"),
        "",
        &mut out,
    );
    out
}
