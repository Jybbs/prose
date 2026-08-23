//! Subset settle probe: every rule alone and every ordered rule pair
//! rewrites a corpus file to a buffer no second pass touches, and every
//! seating that result rests on is declared in the registry.
//!
//! A rule that settles alone and never un-settles an earlier one leaves
//! every larger subset settled, so the singles and the ordered pairs
//! carry the guarantee between them. `PROSE_SETTLE_CORPUS` points the
//! sweep at a directory other than the fixture tree.

use std::{collections::BTreeMap, collections::BTreeSet, path::Path};

use itertools::Itertools;
use prose::{
    config::Config,
    pipeline::{Pipeline, PipelineError},
    rule::{RuleId, render_slugs, runs_behind},
    source::Source,
};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use common::{Tally, corpus};

mod common;

/// The corpus defects one sweep found, each keyed by its own wording so
/// the same shape across many files reports once.
#[derive(Default)]
struct Findings {
    /// Seatings the settling rests on, absent from the dependency column.
    undeclared: Tally,
    /// Subsets leaving a rule still editing their own output.
    unsettled: Tally,
}

impl Findings {
    fn absorb(&mut self, other: Self) {
        self.undeclared.absorb(other.undeclared);
        self.unsettled.absorb(other.unsettled);
    }

    fn total(&self) -> usize {
        self.undeclared.len() + self.unsettled.len()
    }
}

/// Every pipeline the sweep runs, built once and shared across the
/// corpus rather than rebuilt per file.
struct Probes {
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
            pairs,
            solo: Pipeline::known_ids()
                .iter()
                .map(|&rule| (rule, subset(config, &[rule])))
                .collect(),
            touching,
        }
    }
}

/// Runs `first` then `second` over `text`, chaining a single-rule
/// pipeline apiece so the seating is the caller's rather than the
/// registry's. The first stage reads `alone`, which already holds every
/// active rule's own run over `text`. A rule outside `active` leaves
/// `text` alone, so its stage is skipped. `None` when a stage declines
/// the source.
fn in_order(
    probes: &Probes,
    alone: &BTreeMap<RuleId, Source>,
    active: &BTreeSet<RuleId>,
    [first, second]: [RuleId; 2],
    text: &str,
) -> Option<Source> {
    let once = if active.contains(&first) {
        Some(alone.get(&first)?)
    } else {
        None
    };
    settled(&probes.solo[&second], once.map_or(text, Source::text))?.ok()
}

/// Sweeps one corpus file across every subset its active rules reach.
/// A rule counts active when its own solo pipeline would edit the
/// file, the same instance the solo and pair probes run.
fn probe(probes: &Probes, path: &Path) -> Findings {
    let mut findings = Findings::default();
    let Ok(source) = Source::from_path(path) else {
        return findings;
    };
    let text = source.text();
    let active: BTreeSet<RuleId> = probes
        .solo
        .iter()
        .filter_map(|(&rule, solo)| (!solo.unsettled(&source).is_empty()).then_some(rule))
        .collect();
    if active.is_empty() {
        return findings;
    }

    let mut alone: BTreeMap<RuleId, Source> = BTreeMap::new();
    for &rule in &active {
        let solo = &probes.solo[&rule];
        let label = format!("`{rule}` alone");
        let Some(once) = ran(solo, text, &label, &mut findings.unsettled, path) else {
            continue;
        };
        reports_left(solo, &once, &label, &mut findings.unsettled, path);
        alone.insert(rule, once);
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
        let Some(reversed) = in_order(probes, &alone, &active, [later, earlier], text) else {
            continue;
        };
        if pair.unsettled(&reversed).is_empty() || runs_behind(later.as_str(), earlier.as_str()) {
            continue;
        }
        findings
            .undeclared
            .record(format!("`{later}` settles only behind `{earlier}`"), path);
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
    into: &mut Tally,
    path: &Path,
) -> Option<Source> {
    match settled(pipeline, text)? {
        Ok(source) => Some(source),
        Err(error) => {
            into.record(format!("{label} was rejected: {error}"), path);
            None
        }
    }
}

/// Files a `label`-keyed defect for the rules still editing `output`,
/// true when any were.
fn reports_left(
    pipeline: &Pipeline,
    output: &Source,
    label: &str,
    into: &mut Tally,
    path: &Path,
) -> bool {
    let left = pipeline.unsettled(output);
    if left.is_empty() {
        return false;
    }
    into.record(
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
        findings.unsettled.render("unsettled subsets"),
        findings.undeclared.render("undeclared seatings"),
    );
    assert!(
        report.is_empty(),
        "{} distinct defects across the corpus's {} files:{report}",
        findings.total(),
        files.len(),
    );
}
