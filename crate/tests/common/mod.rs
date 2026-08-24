//! Shared snapshot harness for integration test binaries.

#![allow(dead_code)]

use std::{
    collections::BTreeMap,
    env,
    ffi::OsStr,
    io::ErrorKind,
    num::NonZeroUsize,
    path::{Path, PathBuf},
};

use ignore::WalkBuilder;
use itertools::Itertools;
use prose::{config::Config, pipeline::Pipeline, rule::RuleId};
use serde::Deserialize;
use similar::TextDiff;

/// The environment variable aiming a sweep at a directory other than
/// the fixture tree.
pub(crate) const CORPUS: &str = "PROSE_SETTLE_CORPUS";

/// How many distinct defects a rendered tally prints before it reports
/// the remainder as a count.
const SHOWN: usize = 30;

/// The line lengths a sweep covers absent [`WIDTHS_VAR`], the shipped
/// default flanked by the narrow and wide settings a project sets it
/// to.
pub(crate) const WIDTHS: [usize; 6] = [40, 50, 60, 79, 88, 100];

/// The environment variable naming the line lengths a sweep covers.
pub(crate) const WIDTHS_VAR: &str = "PROSE_SETTLE_WIDTHS";

/// Per-fixture flags read from the sidecar TOML's `[harness]` table,
/// independent of the prose config the rule itself consumes.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct HarnessOptions {
    rules: Vec<RuleId>,
    pub(crate) skip_ruff_coexistence: bool,
}

/// The defects one corpus sweep found, each keyed by its own wording so
/// the same shape across many files reports once.
#[derive(Default)]
pub(crate) struct Tally(BTreeMap<String, Site>);

impl Tally {
    /// Folds `other` in, keeping the earlier file for a defect both
    /// sides carry and summing how many files reached it.
    pub(crate) fn absorb(&mut self, other: Self) {
        for (defect, site) in other.0 {
            self.0
                .entry(defect)
                .and_modify(|held| held.count += site.count)
                .or_insert(site);
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    /// Files a defect under its own wording, counting the corpus files
    /// that reach it and holding the first one as the example.
    pub(crate) fn record(&mut self, defect: String, path: &Path) {
        self.record_at(defect, path, None);
    }

    /// Records `defect` against `path`, keeping `repro` as the command
    /// that sweeps this defect alone.
    pub(crate) fn record_at(&mut self, defect: String, path: &Path, repro: Option<&str>) {
        self.0
            .entry(defect)
            .and_modify(|held| held.count += 1)
            .or_insert_with(|| Site {
                count: 1,
                file: path.display().to_string(),
                repro: repro.map(str::to_owned),
            });
    }

    /// Renders the defects as one line apiece under `heading`, capped at
    /// [`SHOWN`] with the remainder reported as a count. Empty for an
    /// empty tally, so a caller concatenates several and tests the
    /// result for emptiness.
    pub(crate) fn render(&self, heading: &str) -> String {
        if self.0.is_empty() {
            return String::new();
        }
        let shown = self
            .0
            .iter()
            .take(SHOWN)
            .map(|(defect, site)| {
                let repro = site
                    .repro
                    .as_ref()
                    .map_or_else(String::new, |cmd| format!("\n    reproduce with {cmd}"));
                format!(
                    "  {defect} ({} files, e.g. {}){repro}",
                    site.count, site.file
                )
            })
            .format("\n");
        let rest = self.0.len().saturating_sub(SHOWN);
        let tail = if rest > 0 {
            format!("\n  ... and {rest} more")
        } else {
            String::new()
        };
        format!("\n{heading} ({}):\n{shown}{tail}", self.0.len())
    }
}

/// Where a defect first showed and how many corpus files carry it.
struct Site {
    count: usize,
    file: String,
    /// The command narrowing a sweep to this defect alone.
    repro: Option<String>,
}

/// Returns the pipeline that exercises a fixture directory.
///
/// `composition` fixtures pin a named subset of rules and the sidecar's
/// `[harness] rules = [...]` field selects exactly that subset, so the
/// snapshot reflects only the listed rules. `notebook`, `suppression`,
/// and `thematic` fixtures exercise the full default pipeline.
/// `binding_analysis` and `identity` run an empty pipeline because their
/// fixtures pin parser and no-op behavior. Every other directory matches
/// a rule slug and runs that rule in isolation.
pub(crate) fn build_pipeline(
    directory: &str,
    config: &Config,
    harness: &HarnessOptions,
) -> Pipeline {
    match directory {
        "composition" => Pipeline::with_filters(config, &harness.rules, &[]),
        "notebook" | "suppression" | "thematic" => Pipeline::with_defaults(config),
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

/// The `.py` files under the corpus root, sorted so a failure names the
/// same file across runs. [`CORPUS`] points a sweep at a directory
/// other than the fixture tree. The walk carries no standard filter, so
/// a hidden directory and an ignored one both enter the sweep rather
/// than leaving it short without saying so.
pub(crate) fn corpus() -> Vec<PathBuf> {
    let root = pointed_corpus()
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"));
    WalkBuilder::new(root)
        .standard_filters(false)
        .build()
        .flatten()
        .map(ignore::DirEntry::into_path)
        .filter(|path| path.extension().is_some_and(|ext| ext == "py"))
        .sorted()
        .collect()
}

pub(crate) fn domain_name(path: &Path) -> &str {
    path.parent()
        .and_then(Path::parent)
        .and_then(Path::file_name)
        .and_then(OsStr::to_str)
        .expect("fixture path has a domain directory")
}

/// The values `var` carries as a space-separated list, `defaults` where
/// it is unset.
pub(crate) fn env_list<T: Clone>(var: &str, defaults: &[T], parse: impl Fn(&str) -> T) -> Vec<T> {
    env::var(var).map_or_else(
        |_| defaults.to_vec(),
        |set| set.split_whitespace().map(&parse).collect(),
    )
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

/// The directory [`CORPUS`] aims a sweep at, `None` for the fixture
/// tree.
pub(crate) fn pointed_corpus() -> Option<PathBuf> {
    env::var_os(CORPUS).map(PathBuf::from)
}

pub(crate) fn unified_diff(expected: &str, actual: &str) -> String {
    TextDiff::from_lines(expected, actual)
        .unified_diff()
        .header("expected", "actual")
        .to_string()
}

/// The line lengths this run sweeps, [`WIDTHS_VAR`] overriding
/// `defaults` as a space-separated list.
pub(crate) fn widths_or(defaults: &[usize]) -> Vec<usize> {
    env_list(WIDTHS_VAR, defaults, |width| {
        width
            .parse::<NonZeroUsize>()
            .unwrap_or_else(|_| panic!("every `{WIDTHS_VAR}` entry is a nonzero number"))
            .get()
    })
}

fn sidecar_contents(path: &Path) -> Option<String> {
    let sidecar = path.with_file_name("config.toml");
    match fs_err::read_to_string(&sidecar) {
        Ok(c) => Some(c),
        Err(e) if e.kind() == ErrorKind::NotFound => None,
        Err(e) => panic!("read sidecar: {e}"),
    }
}
