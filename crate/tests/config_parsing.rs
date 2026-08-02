//! Snapshot tests for `prose.toml` config parsing.
//!
//! Each `tests/fixtures/config/*/input.toml` file is a self-contained
//! `prose.toml` document. The harness parses it through
//! `Config::from_prose_toml_str` and snapshots the resulting `Config`
//! debug representation, so a regression in any default, rename, or
//! field addition surfaces as a snapshot diff rather than slipping
//! past spot-checked assertions.

mod common;

use std::path::Path;

use prose::{config::Config, pipeline::Pipeline};

#[test]
fn every_registered_rule_has_a_config_override() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/config/every_knob_overridden/input.toml");
    let document: toml::Table = fs_err::read_to_string(&path)
        .expect("input.toml reads")
        .parse()
        .expect("input.toml parses");
    let rules = document["rules"].as_table().expect("[rules] table");
    let missing: Vec<String> = Pipeline::known_ids()
        .iter()
        .map(ToString::to_string)
        .filter(|id| !rules.contains_key(id))
        .collect();
    assert!(
        missing.is_empty(),
        "`every_knob_overridden` omits a per-rule override for: {}",
        missing.join(", "),
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
