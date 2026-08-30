//! Subset settle probe: every rule alone and every ordered rule pair
//! rewrites a corpus file to a buffer no second pass touches, and every
//! seating that result rests on is declared in the registry.
//!
//! A rule that settles alone and never un-settles an earlier one leaves
//! every larger subset settled, so the singles and the ordered pairs
//! carry the guarantee between them. Each pair also runs with both
//! rules splicing into one buffer and with a reparse between them, so
//! a pair the registry declares independent fails on any file where
//! the two differ, and a pointed sweep reports which undeclared pairs
//! agree on every file they edit together. The fixture tree runs the
//! sweep at every line length the harness carries, because a subset
//! that settles at one `code-line-length` can still edit its own
//! output at another.
//! `PROSE_SETTLE_CORPUS` points the sweep at a directory other than the
//! fixture tree and drops it to the shipped default budget, and
//! `PROSE_SETTLE_WIDTHS` names the set outright either way.

use std::{
    collections::{BTreeMap, BTreeSet},
    num::NonZeroUsize,
    path::{Path, PathBuf},
};

use itertools::Itertools;
use prose::{
    config::Config,
    pipeline::{Pipeline, PipelineError, Sharing},
    rule::{RuleId, independent, render_slugs, runs_behind},
    source::Source,
};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use common::{Tally, WIDTHS, corpus, pointed_corpus, widths_or};

mod common;

/// How often one undeclared pair's batched splice matched its fold
/// over the files both rules edited.
#[derive(Default)]
struct Agreement {
    agreed: usize,
    diverged: usize,
    /// The first file the two differed on.
    example: Option<String>,
}

impl Agreement {
    fn absorb(&mut self, other: Self) {
        self.agreed += other.agreed;
        self.diverged += other.diverged;
        if self.example.is_none() {
            self.example = other.example;
        }
    }
}

/// The corpus defects one sweep found, each keyed by its own wording so
/// the same shape across many files reports once, beside the agreement
/// each undeclared pair showed.
#[derive(Default)]
struct Findings {
    /// Declared-independent pairs whose batched splice differs from
    /// their fold.
    divergent: Tally,
    sharing: BTreeMap<[RuleId; 2], Agreement>,
    /// Seatings the settling rests on, absent from the dependency column.
    undeclared: Tally,
    /// Subsets leaving a rule still editing their own output.
    unsettled: Tally,
}

impl Findings {
    fn absorb(&mut self, other: Self) {
        self.divergent.absorb(other.divergent);
        for (pair, agreement) in other.sharing {
            self.sharing.entry(pair).or_default().absorb(agreement);
        }
        self.undeclared.absorb(other.undeclared);
        self.unsettled.absorb(other.unsettled);
    }

    /// The undeclared pairs by how they agreed, one line apiece, for a
    /// pointed sweep to print.
    fn render_sharing(&self) -> String {
        let (agreeing, diverging): (Vec<_>, Vec<_>) = self
            .sharing
            .iter()
            .partition(|(_, agreement)| agreement.diverged == 0);
        let line = |(pair, agreement): &(&[RuleId; 2], &Agreement)| {
            let [earlier, later] = pair;
            let example = agreement
                .example
                .as_deref()
                .map_or_else(String::new, |file| format!(", e.g. {file}"));
            format!(
                "  `{earlier}` then `{later}`: {} of {} runs agree{example}",
                agreement.agreed,
                agreement.agreed + agreement.diverged,
            )
        };
        format!(
            "\nundeclared pairs whose batched splice matches their fold ({}):\n{}\n\nundeclared pairs whose batched splice diverges from their fold ({}):\n{}\n",
            agreeing.len(),
            agreeing.iter().map(line).format("\n"),
            diverging.len(),
            diverging.iter().map(line).format("\n"),
        )
    }

    fn total(&self) -> usize {
        self.divergent.len() + self.undeclared.len() + self.unsettled.len()
    }
}

/// One ordered rule pair's probes.
struct Pair {
    /// Whether the registry declares the pair independent, so the
    /// pipeline splices both rules into one buffer.
    declared: bool,
    /// The pair under the sharing the pipeline does not take, the fold
    /// for a declared pair and the batched splice otherwise.
    other: Pipeline,
    /// The pair as the pipeline runs it.
    production: Pipeline,
    rules: [RuleId; 2],
}

/// Every pipeline one budget's sweep runs, built once and shared across
/// the corpus rather than rebuilt per file.
struct Probes {
    /// The `code-line-length` clause every defect this budget files
    /// carries.
    budget: String,
    pairs: Vec<Pair>,
    solo: BTreeMap<RuleId, Pipeline>,
    /// The `pairs` slots each rule appears in, on either side.
    touching: BTreeMap<RuleId, Vec<usize>>,
}

impl Probes {
    fn build(length: usize) -> Self {
        let config = Config {
            code_line_length: NonZeroUsize::new(length),
            ..Config::default()
        };
        let pairs: Vec<Pair> = Pipeline::known_ids()
            .iter()
            .array_combinations()
            .map(|[&earlier, &later]| {
                let declared = independent(later.as_str(), earlier.as_str());
                let other = if declared {
                    Sharing::Never
                } else {
                    Sharing::Always
                };
                let pair = || subset(&config, &[earlier, later]);
                Pair {
                    declared,
                    other: pair().sharing(other),
                    production: pair(),
                    rules: [earlier, later],
                }
            })
            .collect();
        let mut touching: BTreeMap<RuleId, Vec<usize>> = BTreeMap::new();
        for (slot, pair) in pairs.iter().enumerate() {
            for rule in pair.rules {
                touching.entry(rule).or_default().push(slot);
            }
        }
        Self {
            budget: format!("at `code-line-length` {length}"),
            pairs,
            solo: Pipeline::known_ids()
                .iter()
                .map(|&rule| (rule, subset(&config, &[rule])))
                .collect(),
            touching,
        }
    }
}

/// True where `other`, the pair under the sharing the pipeline does
/// not take, leaves `text` as anything other than `production` did. A
/// run one side rejects diverges too.
fn diverges(other: &Pipeline, text: &str, production: &Source) -> bool {
    !matches!(settled(other, text), Some(Ok(out)) if out.text() == production.text())
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

/// The budgets this run probes. A pointed corpus takes the shipped
/// default alone, holding its wall clock where it already sits, and
/// `PROSE_SETTLE_WIDTHS` names the set outright either way.
fn lengths() -> Vec<usize> {
    let pointed = pointed_corpus()
        .and(Config::default().code_line_length)
        .map(|length| vec![length.get()]);
    widths_or(pointed.as_deref().unwrap_or(&WIDTHS))
}

/// Sweeps one corpus file across every subset its active rules reach.
/// A rule counts active when its own solo pipeline would edit the
/// file, the same instance the solo and pair probes run. A rule that
/// fails alone is dropped from the pair sweep, whose every slot would
/// otherwise re-report that one defect.
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
    let mut broken: BTreeSet<RuleId> = BTreeSet::new();
    for &rule in &active {
        let solo = &probes.solo[&rule];
        let label = format!("`{rule}` alone {}", probes.budget);
        let Some(once) = ran(solo, text, &label, &mut findings.unsettled, path) else {
            broken.insert(rule);
            continue;
        };
        if reports_left(solo, &once, &label, &mut findings.unsettled, path) {
            broken.insert(rule);
        }
        alone.insert(rule, once);
    }

    let reachable: BTreeSet<usize> = active
        .iter()
        .flat_map(|rule| probes.touching[rule].iter().copied())
        .collect();
    for slot in reachable {
        let Pair {
            declared,
            other,
            production: pair,
            rules: [earlier, later],
        } = &probes.pairs[slot];
        let (earlier, later) = (*earlier, *later);
        if broken.contains(&earlier) || broken.contains(&later) {
            continue;
        }
        let label = format!("`{earlier}` then `{later}` {}", probes.budget);
        let Some(forward) = ran(pair, text, &label, &mut findings.unsettled, path) else {
            continue;
        };
        if forward.text() == text {
            continue;
        }
        if active.contains(&earlier) && active.contains(&later) {
            let diverged = diverges(other, text, &forward);
            if *declared {
                if diverged {
                    findings.divergent.record(
                        format!(
                            "`{later}` spliced beside `{earlier}` differs from its fold {}",
                            probes.budget
                        ),
                        path,
                    );
                }
            } else {
                let agreement = findings.sharing.entry([earlier, later]).or_default();
                if diverged {
                    agreement.diverged += 1;
                    agreement
                        .example
                        .get_or_insert_with(|| path.display().to_string());
                } else {
                    agreement.agreed += 1;
                }
            }
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
        findings.undeclared.record(
            format!(
                "`{later}` settles only behind `{earlier}` {}",
                probes.budget
            ),
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

/// Folds every file's findings under one budget's probes.
fn sweep(probes: &Probes, files: &[PathBuf]) -> Findings {
    files
        .par_iter()
        .map(|path| probe(probes, path))
        .reduce(Findings::default, |mut held, next| {
            held.absorb(next);
            held
        })
}

#[test]
#[cfg_attr(coverage, ignore = "the sweep runs uninstrumented in its own row")]
fn every_rule_subset_settles_and_declares_its_seating() {
    let files = corpus();
    assert!(!files.is_empty(), "the corpus holds no `.py` files");
    let lengths = lengths();
    let mut findings = Findings::default();
    for &length in &lengths {
        findings.absorb(sweep(&Probes::build(length), &files));
    }
    if pointed_corpus().is_some() {
        eprintln!("{}", findings.render_sharing());
    }
    let report = format!(
        "{}{}{}",
        findings.unsettled.render("unsettled subsets"),
        findings.undeclared.render("undeclared seatings"),
        findings
            .divergent
            .render("declared independence the fold contradicts"),
    );
    assert!(
        report.is_empty(),
        "{} distinct defects across the corpus's {} files, swept at `code-line-length` {}:{report}",
        findings.total(),
        files.len(),
        lengths.iter().format(", "),
    );
}
