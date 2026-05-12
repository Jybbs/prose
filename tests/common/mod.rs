//! Shared snapshot harness for integration test binaries.

#![allow(dead_code)]

use std::ffi::OsStr;
use std::io::ErrorKind;
use std::path::Path;

use prose::config::Config;
use prose::pipeline::Pipeline;
use serde::Deserialize;
use similar::TextDiff;

/// Per-fixture flags read from the sidecar TOML's `[harness]` table,
/// independent of the `[tool.prose]` config the rule itself consumes.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct HarnessOptions {
    pub(crate) skip_ruff_coexistence: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Sidecar {
    harness: HarnessOptions,
}

pub(crate) fn build_pipeline(directory: &str, config: &Config) -> Pipeline {
    match directory {
        "composition" | "suppression" => Pipeline::with_defaults(config),
        "binding_analysis" | "identity" => Pipeline::empty(),
        _ => Pipeline::for_rule(directory, config)
            .unwrap_or_else(|| panic!("no rule registered for fixture directory `{directory}`")),
    }
}

pub(crate) fn case_stem(path: &Path) -> &str {
    path.file_stem()
        .and_then(OsStr::to_str)
        .expect("fixture path has a stem")
}

pub(crate) fn fixture_config(path: &Path) -> Config {
    sidecar_contents(path)
        .map(|c| {
            Config::from_pyproject_str(&c).unwrap_or_else(|e| panic!("parse sidecar config: {e}"))
        })
        .unwrap_or_default()
}

pub(crate) fn in_snapshot_dir(directory: &str, f: impl FnOnce()) {
    insta::with_settings!({
        snapshot_path => format!("snapshots/{directory}"),
        prepend_module_to_snapshot => false,
        snapshot_suffix => "",
    }, {
        f();
    });
}

pub(crate) fn parse_harness_options(contents: &str) -> HarnessOptions {
    toml::from_str::<Sidecar>(contents)
        .unwrap_or_else(|e| panic!("parse sidecar harness section: {e}"))
        .harness
}

pub(crate) fn sidecar_contents(path: &Path) -> Option<String> {
    let stem = case_stem(path)
        .strip_suffix(".input")
        .expect("fixture path ends in .input.py");
    let sidecar = path.with_file_name(format!("{stem}.config.toml"));
    match fs_err::read_to_string(&sidecar) {
        Ok(c) => Some(c),
        Err(e) if e.kind() == ErrorKind::NotFound => None,
        Err(e) => panic!("read sidecar: {e}"),
    }
}

pub(crate) fn unified_diff(expected: &str, actual: &str) -> String {
    TextDiff::from_lines(expected, actual)
        .unified_diff()
        .header("expected", "actual")
        .to_string()
}
