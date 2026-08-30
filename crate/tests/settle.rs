//! Subset settle probe: every rule alone and every ordered rule pair
//! rewrites a corpus file to a buffer no second pass touches, and every
//! seating that result rests on is declared in the registry.
//!
//! A rule that settles alone and never un-settles an earlier one leaves
//! every larger subset settled, so the singles and the ordered pairs
//! carry the guarantee between them. A pair is read off its two rules
//! run one at a time in the caller's order, each constructed against
//! the pair's own selection, and every single-rule run over a buffer is
//! made once per file and shared by each subset whose rule carries the
//! same settings. The fixture tree runs the sweep at every line length
//! the harness carries, because a subset that settles at one
//! `code-line-length` can still edit its own output at another.

use std::{
    collections::{BTreeMap, BTreeSet},
    num::NonZeroUsize,
    path::{Path, PathBuf},
    rc::Rc,
    str::FromStr,
};

use itertools::Itertools;
use prose::{
    config::Config,
    pipeline::Pipeline,
    rule::{RuleId, render_slugs, runs_behind},
    source::Source,
};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rustc_hash::FxHashMap;

use common::{
    CORPUS, Hit, Tally, WIDTHS, WIDTHS_VAR, corpus, note_verified, pointed_corpus, report_verified,
    setting, verifying, widths_or,
};

mod common;

/// The environment variable naming the rules a sweep touches, every
/// subset holding none of them left unprobed.
const RULES_VAR: &str = "PROSE_SETTLE_RULES";

/// The environment variable taking one share of the pairs as `k/n`,
/// the `k`th of `n` counted from one.
const SHARD_VAR: &str = "PROSE_SETTLE_SHARD";

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// One rule's run over one buffer.
#[derive(Clone)]
enum Applied {
    /// The rule rewrote the buffer to this text.
    Changed(Rc<str>),
    /// The pipeline rejected the rule's output.
    Rejected(Rc<str>),
    /// The rule left the buffer as it was.
    Same,
}

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

/// Every single-rule run one file's sweep makes, keyed by the seat of
/// the rule in [`Probes::singles`] and the buffer it ran over.
struct Memo<'p> {
    probes: &'p Probes,
    runs: FxHashMap<(usize, Rc<str>), Applied>,
}

impl Memo<'_> {
    /// The buffer the rule at `seat` leaves `text` as, `text` itself
    /// where the rule made no edit.
    fn after(&mut self, seat: usize, text: &Rc<str>) -> Result<Rc<str>, Rc<str>> {
        match self.apply(seat, text) {
            Applied::Changed(out) => Ok(out),
            Applied::Rejected(error) => Err(error),
            Applied::Same => Ok(Rc::clone(text)),
        }
    }

    fn apply(&mut self, seat: usize, text: &Rc<str>) -> Applied {
        let probes = self.probes;
        self.runs
            .entry((seat, Rc::clone(text)))
            .or_insert_with(|| match text.parse::<Source>() {
                Err(_) => Applied::Rejected(Rc::from("the buffer does not parse")),
                Ok(source) => match probes.singles[seat].pipeline.run(source) {
                    Ok((out, _)) if out.text() == &**text => Applied::Same,
                    Ok((out, _)) => Applied::Changed(Rc::from(out.text())),
                    Err(error) => Applied::Rejected(Rc::from(error.to_string())),
                },
            })
            .clone()
    }

    /// The buffer the rules at `seats` leave `text` as, run first to
    /// second, the first rejection ending the chain.
    fn chain(&mut self, [first, second]: [usize; 2], text: &Rc<str>) -> Result<Rc<str>, Rc<str>> {
        let opened = self.after(first, text)?;
        self.after(second, &opened)
    }

    /// The rules among `seats` still editing `text`, a rejected run
    /// counting as an edit.
    fn editing(&mut self, seats: &[usize], text: &Rc<str>) -> Vec<RuleId> {
        let probes = self.probes;
        seats
            .iter()
            .filter(|&&seat| !matches!(self.apply(seat, text), Applied::Same))
            .map(|&seat| probes.singles[seat].rule)
            .collect()
    }
}

/// Every pipeline one budget's sweep runs, built once and shared across
/// the corpus rather than rebuilt per file.
struct Probes {
    /// The `code-line-length` clause every defect this budget files
    /// carries.
    budget: String,
    /// The configuration every pipeline this run holds resolved
    /// against.
    config: Config,
    /// The pairs this run probes, each in registry order beside the
    /// seats in `singles` of its two rules as the pair constructs them,
    /// narrowed by [`RULES_VAR`] and [`SHARD_VAR`].
    pairs: Vec<([RuleId; 2], [usize; 2])>,
    /// The rules whose solo verdict this run reports, narrowed the same
    /// way.
    reported: BTreeSet<RuleId>,
    /// Every distinct single-rule pipeline, one seat per rule and
    /// settings.
    singles: Vec<Single>,
    /// The seat in `singles` of each rule constructed alone.
    solo: BTreeMap<RuleId, usize>,
    width: usize,
}

/// A rule run on its own, constructed against the selection its
/// pipeline resolved.
struct Single {
    pipeline: Pipeline,
    rule: RuleId,
}

impl Probes {
    fn build(width: usize) -> Self {
        let config = Config {
            code_line_length: NonZeroUsize::new(width),
            ..Config::default()
        };
        let scope = scope();
        let (share, shares) = shard();
        let in_scope = |rule: &RuleId| scope.as_ref().is_none_or(|set| set.contains(rule));
        let taken = |slot: &usize| slot % shares == share;
        let mut singles = Vec::new();
        let mut seats = FxHashMap::default();
        let mut seat = |rule: RuleId, pipeline: Pipeline| -> usize {
            *seats
                .entry((rule, pipeline.fingerprint()))
                .or_insert_with(|| {
                    singles.push(Single { pipeline, rule });
                    singles.len() - 1
                })
        };
        let solo = Pipeline::known_ids()
            .iter()
            .map(|&rule| {
                let [(_, alone)] = seated(&config, [rule]);
                (rule, seat(rule, alone))
            })
            .collect();
        let pairs = Pipeline::known_ids()
            .iter()
            .array_combinations()
            .map(|[&earlier, &later]| [earlier, later])
            .filter(|[earlier, later]| in_scope(earlier) || in_scope(later))
            .enumerate()
            .filter(|(slot, _)| taken(slot))
            .map(|(_, pair)| {
                let [(_, first), (_, second)] = seated(&config, pair);
                (pair, [seat(pair[0], first), seat(pair[1], second)])
            })
            .collect();
        let reported = Pipeline::known_ids()
            .iter()
            .copied()
            .filter(in_scope)
            .enumerate()
            .filter(|(slot, _)| taken(slot))
            .map(|(_, rule)| rule)
            .collect();
        Self {
            budget: format!("at `code-line-length` {width}"),
            config,
            pairs,
            reported,
            singles,
            solo,
            width,
        }
    }

    /// The command sweeping the subsets touching `rules` over `path`
    /// alone at this budget.
    fn hit(&self, path: &Path, rules: &[RuleId]) -> Hit {
        Hit {
            repro: Some(format!(
                "{CORPUS}={} {RULES_VAR}='{}' {WIDTHS_VAR}={} cargo test --test settle",
                path.display(),
                rules.iter().format(" "),
                self.width,
            )),
            ..Hit::default()
        }
    }
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

/// Sweeps one corpus file across every subset it reaches. A rule that
/// fails alone is dropped from the pair sweep, whose every slot would
/// otherwise re-report that one defect, and a pair neither of whose
/// rules edits the file leaves it as it was.
fn probe(probes: &Probes, path: &Path) -> Findings {
    let mut findings = Findings::default();
    let Ok(source) = Source::from_path(path) else {
        return findings;
    };
    let text: Rc<str> = Rc::from(source.text());
    let mut memo = Memo {
        probes,
        runs: FxHashMap::default(),
    };
    let mut broken: BTreeSet<RuleId> = BTreeSet::new();
    for (&rule, &seat) in &probes.solo {
        let label = format!("`{rule}` alone {}", probes.budget);
        let defect = match memo.apply(seat, &text) {
            Applied::Same => continue,
            Applied::Rejected(error) => Some(format!("{label} was rejected: {error}")),
            Applied::Changed(once) => {
                let left = memo.editing(&[seat], &once);
                (!left.is_empty())
                    .then(|| format!("{label} leaves {} editing", render_slugs(&left)))
            }
        };
        if let Some(defect) = defect {
            broken.insert(rule);
            if probes.reported.contains(&rule) {
                findings
                    .unsettled
                    .record_hit(defect, path, probes.hit(path, &[rule]));
            }
        }
    }

    for &(pair @ [earlier, later], seats @ [first, second]) in &probes.pairs {
        if broken.contains(&earlier) || broken.contains(&later) {
            continue;
        }
        let label = format!("`{earlier}` then `{later}` {}", probes.budget);
        let reversed_label = format!("`{later}` then `{earlier}` {}", probes.budget);
        let file = |tally: &mut Tally, defect: String| {
            tally.record_hit(defect, path, probes.hit(path, &pair));
        };
        let chained = memo.chain(seats, &text);
        if verifying() {
            verify_pair(&mut memo, probes, path, &text, pair, seats, &chained);
        }
        let forward = match chained {
            Ok(forward) => forward,
            Err(error) => {
                file(
                    &mut findings.unsettled,
                    format!("{label} was rejected: {error}"),
                );
                continue;
            }
        };
        if forward == text {
            continue;
        }
        let left = memo.editing(&seats, &forward);
        if !left.is_empty() {
            file(
                &mut findings.unsettled,
                format!("{label} leaves {} editing", render_slugs(&left)),
            );
            continue;
        }
        let reversed = match memo.chain([second, first], &text) {
            Ok(reversed) => reversed,
            Err(error) => {
                file(
                    &mut findings.unsettled,
                    format!("{reversed_label} was rejected: {error}"),
                );
                continue;
            }
        };
        if memo.editing(&seats, &reversed).is_empty()
            || runs_behind(later.as_str(), earlier.as_str())
        {
            continue;
        }
        file(
            &mut findings.undeclared,
            format!(
                "`{later}` settles only behind `{earlier}` {}",
                probes.budget
            ),
        );
    }
    findings
}

/// The rules [`RULES_VAR`] names, `None` for every rule when it is
/// absent or blank.
fn scope() -> Option<BTreeSet<RuleId>> {
    setting(RULES_VAR).map(|named| {
        named
            .split([' ', ','])
            .filter(|slug| !slug.is_empty())
            .map(|slug| {
                RuleId::from_str(slug)
                    .unwrap_or_else(|_| panic!("{RULES_VAR} names an unknown rule: {slug}"))
            })
            .collect()
    })
}

/// The single-rule pipelines a selection of exactly `N` rules splits
/// into, each rule as that selection constructed it.
fn seated<const N: usize>(config: &Config, rules: [RuleId; N]) -> [(RuleId, Pipeline); N] {
    subset(config, &rules)
        .split()
        .try_into()
        .ok()
        .expect("invariant: a selection of N rules splits into N pipelines")
}

/// The zero-based share and the share count [`SHARD_VAR`] names, the
/// whole set when it is absent or blank.
fn shard() -> (usize, usize) {
    let Some(spec) = setting(SHARD_VAR) else {
        return (0, 1);
    };
    let parsed = spec
        .split_once('/')
        .and_then(|(k, n)| Some((k.parse::<usize>().ok()?, n.parse::<usize>().ok()?)))
        .filter(|&(k, n)| k >= 1 && k <= n);
    let (k, n) =
        parsed.unwrap_or_else(|| panic!("{SHARD_VAR} takes `k/n` with 1 <= k <= n: {spec}"));
    (k - 1, n)
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

/// Folds `pair` over `text` and panics where the fold's text or its
/// still-editing set differs from the `chained` single-rule runs.
fn verify_pair(
    memo: &mut Memo,
    probes: &Probes,
    path: &Path,
    text: &Rc<str>,
    pair @ [earlier, later]: [RuleId; 2],
    seats: [usize; 2],
    chained: &Result<Rc<str>, Rc<str>>,
) {
    let folded = subset(&probes.config, &pair);
    let old = text
        .parse::<Source>()
        .ok()
        .map(|source| folded.run(source).map(|(out, _)| out.text().to_owned()));
    match (chained, old) {
        (Ok(new), Some(Ok(old))) => assert!(
            **new == *old,
            "forward text differs for `{earlier}` then `{later}` on {}:\n{}",
            path.display(),
            common::unified_diff(&old, new),
        ),
        (Err(_), Some(Err(_))) => {}
        (new, old) => panic!(
            "forward verdict differs for `{earlier}` then `{later}` on {}: chained {} vs folded {}",
            path.display(),
            new.is_ok(),
            old.map_or("unparsed".to_owned(), |o| o.is_ok().to_string()),
        ),
    }
    if let Ok(new) = chained
        && let Ok(parsed) = new.parse::<Source>()
    {
        assert_eq!(
            folded.unsettled(&parsed),
            memo.editing(&seats, new),
            "still-editing set differs for `{earlier}` then `{later}` on {}",
            path.display()
        );
    }
    note_verified();
}

#[test]
#[cfg_attr(coverage, ignore = "the sweep runs uninstrumented in its own row")]
fn every_rule_subset_settles_and_declares_its_seating() {
    let files = corpus();
    assert!(!files.is_empty(), "the corpus holds no `.py` files");
    let lengths = lengths();
    let mut findings = Findings::default();
    for &length in &lengths {
        let probes = Probes::build(length);
        eprintln!(
            "{} single-rule pipelines serve {} solos and {} pairs at width {length}",
            probes.singles.len(),
            probes.solo.len(),
            probes.pairs.len(),
        );
        findings.absorb(sweep(&probes, &files));
    }
    report_verified("chained pairs against the two-rule fold");
    let report = format!(
        "{}{}",
        findings.unsettled.render("unsettled subsets"),
        findings.undeclared.render("undeclared seatings"),
    );
    assert!(
        report.is_empty(),
        "{} distinct defects across the corpus's {} files, swept at `code-line-length` {}:{report}",
        findings.total(),
        files.len(),
        lengths.iter().format(", "),
    );
}
