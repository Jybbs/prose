//! Snapshot tests for `[tool.prose]` config parsing.
//!
//! Each `tests/fixtures/config/*.toml` file is a self-contained
//! `pyproject.toml` snippet. The harness parses it through
//! `Config::from_pyproject_str` and snapshots the resulting `Config`
//! debug representation, so a regression in any default, rename, or
//! field addition surfaces as a snapshot diff rather than slipping
//! past spot-checked assertions.

mod common;

use prose::config::Config;

#[test]
fn fixtures() {
    insta::glob!("fixtures/config/*.toml", |path| {
        let case = common::case_stem(path);
        let toml = fs_err::read_to_string(path).expect("fixture reads");
        let config = Config::from_pyproject_str(&toml).expect("fixture parses");

        let reparsed = Config::from_pyproject_str(&toml).expect("fixture re-parses");
        let (a, b) = (format!("{config:#?}"), format!("{reparsed:#?}"));
        assert!(
            a == b,
            "config parsing not deterministic for `{case}`:\n{}",
            common::unified_diff(&a, &b),
        );

        common::in_snapshot_dir("config", || {
            insta::assert_debug_snapshot!(case, config);
        });
    });
}
