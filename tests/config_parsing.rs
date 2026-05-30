//! Snapshot tests for `prose.toml` config parsing.
//!
//! Each `tests/fixtures/config/*/input.toml` file is a self-contained
//! `prose.toml` document. The harness parses it through
//! `Config::from_prose_toml_str` and snapshots the resulting `Config`
//! debug representation, so a regression in any default, rename, or
//! field addition surfaces as a snapshot diff rather than slipping
//! past spot-checked assertions.

mod common;

use prose::config::Config;

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
