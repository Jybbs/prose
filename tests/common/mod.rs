//! Shared snapshot harness for integration test binaries.

#![allow(dead_code)]

use std::ffi::OsStr;
use std::path::Path;

use serde::Deserialize;
use similar::TextDiff;

/// Per-fixture flags read from the sidecar TOML's `[harness]` table,
/// independent of the `[tool.prose]` config the rule itself consumes.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct HarnessOptions {
    pub skip_ruff_coexistence: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Sidecar {
    harness: HarnessOptions,
}

pub fn case_stem(path: &Path) -> &str {
    path.file_stem()
        .and_then(OsStr::to_str)
        .expect("fixture path has a stem")
}

pub fn in_snapshot_dir(directory: &str, f: impl FnOnce()) {
    insta::with_settings!({
        snapshot_path => format!("snapshots/{directory}"),
        prepend_module_to_snapshot => false,
        snapshot_suffix => "",
    }, {
        f();
    });
}

pub fn parse_harness_options(contents: &str) -> HarnessOptions {
    toml::from_str::<Sidecar>(contents)
        .unwrap_or_else(|e| panic!("parse sidecar harness section: {e}"))
        .harness
}

pub fn unified_diff(expected: &str, actual: &str) -> String {
    TextDiff::from_lines(expected, actual)
        .unified_diff()
        .header("expected", "actual")
        .to_string()
}
