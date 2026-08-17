//! Subset settle probe: every rule alone and every ordered rule pair
//! rewrites a corpus file to a buffer no second pass touches, and every
//! seating that result rests on is declared in the registry.
//!
//! A rule that settles alone and never un-settles an earlier one leaves
//! every larger subset settled, so the singles and the ordered pairs
//! carry the guarantee between them. `PROSE_SETTLE_CORPUS` points the
//! sweep at a directory other than the fixture tree.

use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    path::{Path, PathBuf},
};

use ignore::Walk;
use itertools::Itertools;
use prose::{
    config::Config,
    pipeline::{Pipeline, PipelineError},
    rule::{RuleId, render_slugs, runs_behind},
    source::Source,
};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

/// How many distinct defects the failure message prints before it
/// reports the remainder as a count.
const SHOWN: usize = 30;

/// The corpus defects one sweep found, each keyed by its own wording so
/// the same shape across many files reports once.
#[derive(Default)]
struct Findings {
    /// Seatings the settling rests on, absent from the dependency column.
    undeclared: BTreeMap<String, Site>,
    /// Subsets leaving a rule still editing their own output.
    unsettled: BTreeMap<String, Site>,
}

impl Findings {
    fn absorb(&mut self, other: Self) {
        merge(&mut self.undeclared, other.undeclared);
        merge(&mut self.unsettled, other.unsettled);
    }

    fn total(&self) -> usize {
        self.undeclared.len() + self.unsettled.len()
    }
}

/// Every pipeline the sweep runs, built once and shared across the
/// corpus rather than rebuilt per file.
struct Probes {
    all: Pipeline,
    pairs: Vec<([RuleId; 2], Pipeline)>,
    solo: BTreeMap<RuleId, Pipeline>,
    /// The `pairs` slots each rule appears in, on either side.
    touching: BTreeMap<RuleId, Vec<usize>>,
}

impl Probes {
    fn build(config: &Config) -> Self {
        let pairs: Vec<([RuleId; 2], Pipeline)> = Pipeline::known_ids()
            .iter()
            .array_combinations()
            .map(|[&earlier, &later]| ([earlier, later], subset(config, &[earlier, later])))
            .collect();
        let mut touching: BTreeMap<RuleId, Vec<usize>> = BTreeMap::new();
        for (slot, ([earlier, later], _)) in pairs.iter().enumerate() {
            touching.entry(*earlier).or_default().push(slot);
            touching.entry(*later).or_default().push(slot);
        }
        Self {
            all: subset(config, Pipeline::known_ids()),
            pairs,
            solo: Pipeline::known_ids()
                .iter()
                .map(|&rule| (rule, subset(config, &[rule])))
                .collect(),
            touching,
        }
    }
}

/// Where a defect first showed and how many corpus files carry it.
struct Site {
    count: usize,
    file: String,
}

/// The `.py` files under the corpus root, sorted so a failure names the
/// same file across runs.
fn corpus() -> Vec<PathBuf> {
    let root = env::var("PROSE_SETTLE_CORPUS").map_or_else(
        |_| Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"),
        PathBuf::from,
    );
    Walk::new(root)
        .flatten()
        .map(ignore::DirEntry::into_path)
        .filter(|path| path.extension().is_some_and(|ext| ext == "py"))
        .sorted()
        .collect()
}

/// Runs `first` then `second` over `text`, chaining a single-rule
/// pipeline apiece so the seating is the caller's rather than the
/// registry's. A rule outside `active` leaves `text` alone, so its
/// stage is skipped. `None` when a stage declines the source.
fn in_order(
    probes: &Probes,
    active: &BTreeSet<RuleId>,
    [first, second]: [RuleId; 2],
    text: &str,
) -> Option<Source> {
    let once = if active.contains(&first) {
        Some(settled(&probes.solo[&first], text)?.ok()?)
    } else {
        None
    };
    settled(
        &probes.solo[&second],
        once.as_ref().map_or(text, Source::text),
    )?
    .ok()
}

/// Folds `other` into `into`, keeping the earlier file for a defect
/// both sides carry and summing how many files reached it.
fn merge(into: &mut BTreeMap<String, Site>, other: BTreeMap<String, Site>) {
    for (defect, site) in other {
        into.entry(defect)
            .and_modify(|held| held.count += site.count)
            .or_insert(site);
    }
}

/// Sweeps one corpus file across every subset its active rules reach.
fn probe(probes: &Probes, path: &Path) -> Findings {
    let mut findings = Findings::default();
    let Ok(source) = Source::from_path(path) else {
        return findings;
    };
    let text = source.text();
    let active: BTreeSet<RuleId> = probes.all.unsettled(&source).into_iter().collect();
    if active.is_empty() {
        return findings;
    }

    for &rule in &active {
        let solo = &probes.solo[&rule];
        let label = format!("`{rule}` alone");
        let Some(once) = ran(solo, text, &label, &mut findings.unsettled, path) else {
            continue;
        };
        reports_left(solo, &once, &label, &mut findings.unsettled, path);
    }

    let reachable: BTreeSet<usize> = active
        .iter()
        .flat_map(|rule| probes.touching[rule].iter().copied())
        .collect();
    for slot in reachable {
        let ([earlier, later], pair) = &probes.pairs[slot];
        let (earlier, later) = (*earlier, *later);
        let label = format!("`{earlier}` then `{later}`");
        let Some(forward) = ran(pair, text, &label, &mut findings.unsettled, path) else {
            continue;
        };
        if forward.text() == text {
            continue;
        }
        if reports_left(pair, &forward, &label, &mut findings.unsettled, path) {
            continue;
        }
        let Some(reversed) = in_order(probes, &active, [later, earlier], text) else {
            continue;
        };
        if pair.unsettled(&reversed).is_empty() || runs_behind(later.as_str(), earlier.as_str()) {
            continue;
        }
        record(
            &mut findings.undeclared,
            format!("`{later}` settles only behind `{earlier}`"),
            path,
        );
    }
    findings
}

/// Runs `pipeline` over `text`, recording a rejected run against
/// `label`'s wording. `None` when the text does not parse or the run
/// was rejected.
fn ran(
    pipeline: &Pipeline,
    text: &str,
    label: &str,
    into: &mut BTreeMap<String, Site>,
    path: &Path,
) -> Option<Source> {
    match settled(pipeline, text)? {
        Ok(source) => Some(source),
        Err(error) => {
            record(into, format!("{label} was rejected: {error}"), path);
            None
        }
    }
}

/// Files a defect under its own wording, counting the corpus files that
/// reach it and holding the first one as the example.
fn record(into: &mut BTreeMap<String, Site>, defect: String, path: &Path) {
    into.entry(defect)
        .and_modify(|held| held.count += 1)
        .or_insert_with(|| Site {
            count: 1,
            file: path.display().to_string(),
        });
}

/// Renders `defects` as one line apiece under `heading`, capped at
/// [`SHOWN`] with the remainder reported as a count.
fn render(heading: &str, defects: &BTreeMap<String, Site>) -> String {
    if defects.is_empty() {
        return String::new();
    }
    let shown = defects
        .iter()
        .take(SHOWN)
        .map(|(defect, site)| format!("  {defect} ({} files, e.g. {})", site.count, site.file))
        .format("\n");
    let rest = defects.len().saturating_sub(SHOWN);
    let tail = if rest > 0 {
        format!("\n  ... and {rest} more")
    } else {
        String::new()
    };
    format!("\n{heading} ({}):\n{shown}{tail}", defects.len())
}

/// Files a `label`-keyed defect for the rules still editing `output`,
/// true when any were.
fn reports_left(
    pipeline: &Pipeline,
    output: &Source,
    label: &str,
    into: &mut BTreeMap<String, Site>,
    path: &Path,
) -> bool {
    let left = pipeline.unsettled(output);
    if left.is_empty() {
        return false;
    }
    record(
        into,
        format!("{label} leaves {} editing", render_slugs(&left)),
        path,
    );
    true
}

/// Runs `pipeline` over a fresh parse of `text`. `None` when `text`
/// itself does not parse, leaving the run's own rejection to surface as
/// the `Err` the caller records.
fn settled(pipeline: &Pipeline, text: &str) -> Option<Result<Source, PipelineError>> {
    let source = text.parse::<Source>().ok()?;
    Some(pipeline.run(source).map(|(formatted, _)| formatted))
}

/// A pipeline carrying exactly `rules`, bypassing each one's `enabled`
/// flag the way a `--select` run does.
fn subset(config: &Config, rules: &[RuleId]) -> Pipeline {
    Pipeline::with_filters(config, rules, &[])
}

#[test]
#[cfg_attr(coverage, ignore = "the sweep runs uninstrumented in its own row")]
fn every_rule_subset_settles_and_declares_its_seating() {
    let files = corpus();
    assert!(!files.is_empty(), "the corpus holds no `.py` files");
    let probes = Probes::build(&Config::default());
    let findings = files.par_iter().map(|path| probe(&probes, path)).reduce(
        Findings::default,
        |mut held, next| {
            held.absorb(next);
            held
        },
    );
    let report = format!(
        "{}{}",
        render("unsettled subsets", &findings.unsettled),
        render("undeclared seatings", &findings.undeclared),
    );
    assert!(
        report.is_empty(),
        "{} distinct defects across the corpus's {} files:{report}",
        findings.total(),
        files.len(),
    );
}
