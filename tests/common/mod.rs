//! Shared snapshot harness for integration test binaries.

#![allow(dead_code)]

use std::ffi::OsStr;
use std::io::ErrorKind;
use std::path::Path;

use prose::config::Config;
use prose::pipeline::Pipeline;
use prose::rule::RuleId;
use serde::Deserialize;
use similar::TextDiff;

/// Per-fixture flags read from the sidecar TOML's `[harness]` table,
/// independent of the prose config the rule itself consumes.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct HarnessOptions {
    rules: Vec<RuleId>,
    pub(crate) skip_ruff_coexistence: bool,
}

/// Returns the pipeline that exercises a fixture directory.
///
/// `composition` fixtures pin a named subset of rules and the sidecar's
/// `[harness] rules = [...]` field selects exactly that subset, so the
/// snapshot reflects only the listed rules. `thematic` and `suppression`
/// fixtures exercise the full default pipeline. `binding_analysis` and
/// `identity` run an empty pipeline because their fixtures pin parser
/// and no-op behavior. Every other directory matches a rule slug and
/// runs that rule in isolation.
pub(crate) fn build_pipeline(
    directory: &str,
    config: &Config,
    harness: &HarnessOptions,
) -> Pipeline {
    match directory {
        "composition" => Pipeline::with_filters(config, &harness.rules, &[]),
        "thematic" | "suppression" => Pipeline::with_defaults(config),
        "binding_analysis" | "identity" => Pipeline::empty(),
        _ => Pipeline::for_rule(directory, config)
            .unwrap_or_else(|| panic!("no rule registered for fixture directory `{directory}`")),
    }
}

pub(crate) fn case_name(path: &Path) -> &str {
    path.parent()
        .and_then(Path::file_name)
        .and_then(OsStr::to_str)
        .expect("fixture path has a case directory")
}

pub(crate) fn domain_name(path: &Path) -> &str {
    path.parent()
        .and_then(Path::parent)
        .and_then(Path::file_name)
        .and_then(OsStr::to_str)
        .expect("fixture path has a domain directory")
}

/// Reads a fixture's `config.toml` sidecar as a `prose.toml` document, lifting
/// the `[harness]` table out before the remainder deserializes into `Config`,
/// so the prose config sits at the document root the way a real `prose.toml`
/// carries it. A sidecar with no prose keys resolves to `Config::default`.
pub(crate) fn fixture_inputs(path: &Path) -> (Config, HarnessOptions) {
    let Some(contents) = sidecar_contents(path) else {
        return Default::default();
    };
    let mut table: toml::Table =
        toml::from_str(&contents).unwrap_or_else(|e| panic!("parse sidecar TOML: {e}"));
    let harness: HarnessOptions = table
        .remove("harness")
        .map(|section| {
            section
                .try_into()
                .unwrap_or_else(|e| panic!("parse sidecar harness section: {e}"))
        })
        .unwrap_or_default();
    let config: Config = toml::Value::Table(table)
        .try_into()
        .unwrap_or_else(|e| panic!("parse sidecar config: {e}"));
    (config, harness)
}

pub(crate) fn in_snapshot_dir(path: &Path, f: impl FnOnce()) {
    insta::with_settings!({
        snapshot_path => format!("fixtures/{}/{}", domain_name(path), case_name(path)),
        prepend_module_to_snapshot => false,
        snapshot_suffix => "",
    }, {
        f();
    });
}

pub(crate) fn unified_diff(expected: &str, actual: &str) -> String {
    TextDiff::from_lines(expected, actual)
        .unified_diff()
        .header("expected", "actual")
        .to_string()
}

fn sidecar_contents(path: &Path) -> Option<String> {
    let sidecar = path.with_file_name("config.toml");
    match fs_err::read_to_string(&sidecar) {
        Ok(c) => Some(c),
        Err(e) if e.kind() == ErrorKind::NotFound => None,
        Err(e) => panic!("read sidecar: {e}"),
    }
}
