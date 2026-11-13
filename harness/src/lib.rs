//! The test support the `prose` integration binaries share, covering the
//! fixture harness that builds each case's pipeline and snapshot settings,
//! the corpus-sweep surface the settle, subset, and import probes read, and
//! the diff excerpts and defect tallies their reports render.

use std::{ffi::OsStr, io::ErrorKind, path::Path};

use prose::{config::Config, pipeline::Pipeline, rules::RuleId};
use serde::Deserialize;

mod diff;
mod sweep;
mod tally;

pub use diff::{excerpt, unified_diff};
pub use sweep::{
    Absorbing, Slot, TARGETS, WIDTHS, WIDTHS_VAR, corpus, env_list, env_list_of, note_verified,
    pointed_corpus, python_files, report_verified, repro_command, setting, swept, target_name,
    target_names, targets_or, unread, verifying, watch_for_a_runaway, widths_or,
};
use tally::more;
pub use tally::{Hit, SHOWN, Tally, remainder, with_rest};

/// Per-fixture flags read from the sidecar TOML's `[harness]` table,
/// independent of the prose config the rule itself consumes.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct HarnessOptions {
    rules: Vec<RuleId>,
    pub skip_ruff_coexistence: bool,
}

/// Returns the pipeline that exercises a fixture directory.
///
/// `composition` fixtures pin a named subset of rules and the sidecar's
/// `[harness] rules = [...]` field selects exactly that subset, so the
/// snapshot reflects only the listed rules. `notebook`, `suppression`,
/// and `thematic` fixtures exercise the full default pipeline.
/// `binding_analysis` and `identity` run an empty pipeline because their
/// fixtures pin parser and no-op behavior. Every other directory matches
/// a rule slug and runs that rule beside any the sidecar's
/// `[harness] rules` lists, every other rule resolving to off.
pub fn build_pipeline(directory: &str, config: &Config, harness: &HarnessOptions) -> Pipeline {
    match directory {
        "composition" => subset(config, &harness.rules),
        "notebook" | "suppression" | "thematic" => Pipeline::with_defaults(config),
        "binding_analysis" | "identity" => Pipeline::empty(),
        _ => {
            let own: RuleId = directory.replace('_', "-").parse().unwrap_or_else(|_| {
                panic!("no rule registered for fixture directory `{directory}`")
            });
            subset(config, &[&[own], harness.rules.as_slice()].concat())
        }
    }
}

pub fn case_name(path: &Path) -> &str {
    path.parent()
        .and_then(Path::file_name)
        .and_then(OsStr::to_str)
        .expect("fixture path has a case directory")
}

pub fn domain_name(path: &Path) -> &str {
    path.parent()
        .map(case_name)
        .expect("fixture path has a domain directory")
}

/// Reads a fixture's `config.toml` sidecar as a `prose.toml` document, lifting
/// the `[harness]` table out before the remainder deserializes into `Config`,
/// so the prose config sits at the document root the way a real `prose.toml`
/// carries it. A sidecar with no prose keys resolves to `Config::default`.
pub fn fixture_inputs(path: &Path) -> (Config, HarnessOptions) {
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

/// Runs `f` with `insta` keeping each snapshot in the case directory that
/// holds `path`, under the `fixtures/` tree beside the test file making the
/// assertion.
pub fn in_snapshot_dir(path: &Path, f: impl FnOnce()) {
    insta::with_settings!({
        snapshot_path => format!("fixtures/{}/{}", domain_name(path), case_name(path)),
        prepend_module_to_snapshot => false,
        snapshot_suffix => "",
    }, {
        f();
    });
}

/// A pipeline carrying exactly `rules`, bypassing each one's `enabled`
/// flag the way a `--select` run does.
pub fn subset(config: &Config, rules: &[RuleId]) -> Pipeline {
    Pipeline::with_filters(config, rules, &[])
}

fn sidecar_contents(path: &Path) -> Option<String> {
    let sidecar = path.with_file_name("config.toml");
    match fs_err::read_to_string(&sidecar) {
        Ok(c) => Some(c),
        Err(e) if e.kind() == ErrorKind::NotFound => None,
        Err(e) => panic!("read sidecar: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, path::Path};

    use ignore::Walk;

    /// The names a path list opening `rest` brings in, the braced list
    /// where it opens on `{` and the leading identifier otherwise.
    fn listed(rest: &str) -> Vec<&str> {
        match rest.strip_prefix('{') {
            Some(list) => list[..list.find('}').expect("closed path list")]
                .split(',')
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .collect(),
            None => vec![
                rest.split(|c: char| !(c.is_alphanumeric() || c == '_'))
                    .next()
                    .unwrap_or_default(),
            ],
        }
    }

    #[test]
    fn every_export_has_a_caller() {
        let (source, _) = include_str!("lib.rs")
            .split_once("#[cfg(test)]")
            .expect("lib.rs holds its tests module");
        let reexported = source.match_indices("pub use ").flat_map(|(at, prefix)| {
            let (_, rest) = source[at + prefix.len()..]
                .split_once("::")
                .expect("re-export names its module");
            listed(rest)
        });
        let declared = source
            .lines()
            .filter_map(|line| line.strip_prefix("pub "))
            .filter_map(|line| line.split_once(' '))
            .filter(|(kind, _)| *kind != "use")
            .flat_map(|(_, rest)| listed(rest));
        let tests = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("harness sits in the workspace")
            .join("crate/tests");
        let named: BTreeSet<String> = Walk::new(tests)
            .map(|entry| entry.expect("walk crate/tests").into_path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
            .flat_map(|path| {
                let text = fs_err::read_to_string(path).expect("read test source");
                text.match_indices("prose_harness::")
                    .flat_map(|(at, prefix)| listed(&text[at + prefix.len()..]))
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            })
            .collect();
        let uncalled: Vec<&str> = reexported
            .chain(declared)
            .filter(|name| !named.contains(*name))
            .collect();
        assert_eq!(
            uncalled,
            Vec::<&str>::new(),
            "exports no file under crate/tests names"
        );
    }
}
