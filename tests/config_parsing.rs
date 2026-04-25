//! Snapshot tests for `[tool.prose]` config parsing.
//!
//! Each `tests/fixtures/config/*.toml` file is a self-contained
//! `pyproject.toml` snippet. The harness parses it through
//! `Config::from_pyproject_str` and snapshots the resulting `Config`
//! debug representation, so a regression in any default, rename, or
//! field addition surfaces as a snapshot diff rather than slipping
//! past spot-checked assertions.

use std::ffi::OsStr;
use std::path::PathBuf;

use prose::config::Config;

#[test]
fn fixtures() {
    insta::glob!("fixtures/config/*.toml", |path| {
        let case = path
            .file_stem()
            .and_then(OsStr::to_str)
            .expect("fixture path has a stem");
        let toml = fs_err::read_to_string(path).expect("fixture reads");
        let config = Config::from_pyproject_str(&toml).expect("fixture parses");

        insta::with_settings!({
            snapshot_path => PathBuf::from("snapshots").join("config"),
            prepend_module_to_snapshot => false,
            snapshot_suffix => "",
        }, {
            insta::assert_debug_snapshot!(case, config);
        });
    });
}
