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
//! same settings. A pair the registry declares independent also runs
//! as one pipeline splicing both rules into a single buffer, and fails
//! on any file where that run differs from the chained one, whereas a
//! pointed sweep reports which undeclared pairs agree on every file
//! they edit together. The fixture tree runs the sweep at every line
//! length the harness carries, because a subset that settles at one
//! `code-line-length` can still edit its own output at another.

use std::{
    assert_matches,
    collections::{BTreeMap, BTreeSet},
    num::NonZeroUsize,
    path::Path,
    rc::Rc,
    str::FromStr,
};

use itertools::Itertools;
use prose::{
    config::Config,
    pipeline::{Pipeline, PipelineError, Sharing},
    rule::{RuleId, independent, render_slugs, runs_behind},
    source::Source,
};
use rstest::rstest;
use rustc_hash::FxHashMap;

use common::{
    Absorbing, CORPUS, Hit, Slot, Tally, WIDTHS, WIDTHS_VAR, corpus, note_verified, pointed_corpus,
    report_verified, setting, swept, unread, verifying, widths_or,
};

mod common;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// The environment variable choosing how a scoped run claims its pairs.
const PAIRS_VAR: &str = "PROSE_SETTLE_PAIRS";

/// The environment variable naming the rules a sweep touches, every
/// subset holding none of them left unprobed.
const RULES_VAR: &str = "PROSE_SETTLE_RULES";

/// The environment variable taking one share of the pairs as `k/n`,
/// the `k`th of `n` counted from one.
const SHARD_VAR: &str = "PROSE_SETTLE_SHARD";

/// How often one undeclared pair's spliced run matched its chained one
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

    /// Counts one file's verdict, holding the first divergent file as
    /// the example.
    fn record(&mut self, agrees: bool, path: &Path) {
        if agrees {
            self.agreed += 1;
        } else {
            self.diverged += 1;
            self.example
                .get_or_insert_with(|| path.display().to_string());
        }
    }
}

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

/// How a scoped run claims the pairs it probes.
#[derive(Clone, Copy, Debug)]
enum Claim {
    /// Every pair whose earlier rule this run scopes, so a set of runs
    /// partitioning the rules probes each pair exactly once.
    Owned,
    /// Every pair either of whose rules this run scopes, so one run
    /// reaches every subset touching them.
    Touching,
}

/// The corpus defects one sweep found, each keyed by its own wording so
/// the same shape across many files reports once.
#[derive(Default)]
struct Findings {
    /// Declared-independent pairs whose spliced run differs from their
    /// chained one.
    divergent: Tally,
    /// How each undeclared pair's spliced run compared, gathered only
    /// for the pointed sweep that prints it.
    sharing: BTreeMap<[RuleId; 2], Agreement>,
    /// Files the corpus held that the probe could not read, named once
    /// however many widths reached them.
    skipped: BTreeSet<String>,
    /// Seatings the settling rests on, absent from the dependency column.
    undeclared: Tally,
    /// Subsets leaving a rule still editing their own output.
    unsettled: Tally,
}

impl Findings {
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
            "\nundeclared pairs whose spliced run matches their chained one ({}):\n{}\n\nundeclared pairs whose spliced run diverges from their chained one ({}):\n{}\n",
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

impl Absorbing for Findings {
    fn absorb(&mut self, other: Self) {
        self.divergent.absorb(other.divergent);
        for (pair, agreement) in other.sharing {
            self.sharing.entry(pair).or_default().absorb(agreement);
        }
        self.skipped.extend(other.skipped);
        self.undeclared.absorb(other.undeclared);
        self.unsettled.absorb(other.unsettled);
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
                Ok(source) => match probes.singles[seat].pipeline.format(source) {
                    Ok(out) if out.text() == &**text => Applied::Same,
                    Ok(out) => Applied::Changed(Rc::from(out.text())),
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

/// One ordered rule pair's probes, built once per budget and shared
/// across the corpus.
struct Pair {
    /// Whether the registry declares the pair independent, so `folded`
    /// splices both rules into one buffer.
    declared: bool,
    /// Both rules in one pipeline, under the sharing a run ships.
    folded: Pipeline,
    rules: [RuleId; 2],
    /// The seats in [`Probes::singles`] of the two rules as this pair
    /// constructs them.
    seats: [usize; 2],
    /// The pair spliced into one buffer whatever the registry
    /// declares, held for the agreement a pointed sweep reports and
    /// absent for every other run.
    spliced: Option<Pipeline>,
}

/// Every pipeline one budget's sweep runs, built once and shared across
/// the corpus rather than rebuilt per file.
struct Probes {
    /// The `code-line-length` clause every defect this budget files
    /// carries.
    budget: String,
    /// The pairs this run probes, in registry order, narrowed by
    /// [`RULES_VAR`] and [`SHARD_VAR`].
    pairs: Vec<Pair>,
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

impl Probes {
    fn build(width: usize) -> Self {
        let config = Config {
            code_line_length: NonZeroUsize::new(width),
            ..Config::default()
        };
        let scope = scope();
        let (share, shares) = shard();
        let claim = claim();
        let in_scope = |rule: &RuleId| scope.as_ref().is_none_or(|set| set.contains(rule));
        let reporting_agreement = pointed_corpus().is_some();
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
            .copied()
            .array_combinations()
            .filter(|[earlier, later]| match claim {
                Claim::Owned => in_scope(earlier),
                Claim::Touching => in_scope(earlier) || in_scope(later),
            })
            .skip(share)
            .step_by(shares)
            .map(|pair @ [earlier, later]| {
                let [(_, first), (_, second)] = seated(&config, pair);
                let declared = independent(later.as_str(), earlier.as_str());
                Pair {
                    declared,
                    folded: subset(&config, &pair),
                    rules: pair,
                    seats: [seat(earlier, first), seat(later, second)],
                    spliced: (!declared && reporting_agreement)
                        .then(|| subset(&config, &pair).sharing(Sharing::Always)),
                }
            })
            .collect();
        let reported = Pipeline::known_ids()
            .iter()
            .copied()
            .filter(in_scope)
            .skip(share)
            .step_by(shares)
            .collect();
        Self {
            budget: format!("at `code-line-length` {width}"),
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

/// A rule run on its own, constructed against the selection its
/// pipeline resolved.
struct Single {
    pipeline: Pipeline,
    rule: RuleId,
}

/// How this run claims its pairs, [`PAIRS_VAR`] naming the shape and
/// `touching` standing where it is absent.
fn claim() -> Claim {
    claim_of(setting(PAIRS_VAR).as_deref())
}

/// The claim `named` chooses, `touching` for anything else.
fn claim_of(named: Option<&str>) -> Claim {
    match named {
        Some("owned") => Claim::Owned,
        Some(other) if other != "touching" => {
            panic!("{PAIRS_VAR} takes `owned` or `touching`: {other}")
        }
        _ => Claim::Touching,
    }
}

/// The budgets this run probes. A pointed corpus takes the shipped
/// default alone, holding its wall clock where it already sits, and
/// `PROSE_SETTLE_WIDTHS` names the set outright either way.
fn lengths() -> Vec<usize> {
    let pointed = pointed_corpus()
        .and(Config::default().code_line_length)
        .map(|length| vec![length.get()]);
    widths_or(pointed.as_deref().unwrap_or(WIDTHS))
}

/// Sweeps one corpus file across every subset it reaches. A rule that
/// fails alone is dropped from the pair sweep, whose every slot would
/// otherwise re-report that one defect, and a pair neither of whose
/// rules edits the file leaves it as it was.
fn probe(probes: &Probes, path: &Path) -> Findings {
    let mut findings = Findings::default();
    let Ok(source) = Source::from_path(path) else {
        findings.skipped.insert(path.display().to_string());
        return findings;
    };
    let _slot = Slot::open(format!("{} {}", path.display(), probes.budget));
    let text: Rc<str> = Rc::from(source.text());
    let mut memo = Memo {
        probes,
        runs: FxHashMap::default(),
    };
    let mut broken: BTreeSet<RuleId> = BTreeSet::new();
    for (&rule, &seat) in &probes.solo {
        let label = || format!("`{rule}` alone {}", probes.budget);
        let defect = match memo.apply(seat, &text) {
            Applied::Same => continue,
            Applied::Rejected(error) => Some(format!("{} was rejected: {error}", label())),
            Applied::Changed(once) => {
                let left = memo.editing(&[seat], &once);
                (!left.is_empty())
                    .then(|| format!("{} leaves {} editing", label(), render_slugs(&left)))
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

    for probe in &probes.pairs {
        let (declared, pair @ [earlier, later], seats @ [first, second]) =
            (probe.declared, probe.rules, probe.seats);
        if broken.contains(&earlier) || broken.contains(&later) {
            continue;
        }
        let clause =
            |first: RuleId, then: RuleId| format!("`{first}` then `{then}` {}", probes.budget);
        let file = |tally: &mut Tally, defect: String| {
            tally.record_hit(defect, path, probes.hit(path, &pair));
        };
        let chained = memo.chain(seats, &text);
        if verifying() {
            verify_pair(&mut memo, path, &text, probe, &chained);
        }
        let forward = match chained {
            Ok(forward) => forward,
            Err(error) => {
                file(
                    &mut findings.unsettled,
                    format!("{} was rejected: {error}", clause(earlier, later)),
                );
                continue;
            }
        };
        if forward == text {
            continue;
        }
        if memo.editing(&seats, &text).len() == 2 {
            let agrees = |spliced: &Pipeline| spliced_matches(spliced, &text, &forward);
            if declared {
                if !agrees(&probe.folded) {
                    file(
                        &mut findings.divergent,
                        format!(
                            "`{later}` spliced beside `{earlier}` differs from its chained run {}",
                            probes.budget
                        ),
                    );
                }
            } else if let Some(spliced) = &probe.spliced {
                findings
                    .sharing
                    .entry(pair)
                    .or_default()
                    .record(agrees(spliced), path);
            }
        }
        let left = memo.editing(&seats, &forward);
        if !left.is_empty() {
            file(
                &mut findings.unsettled,
                format!(
                    "{} leaves {} editing",
                    clause(earlier, later),
                    render_slugs(&left)
                ),
            );
            continue;
        }
        let reversed = match memo.chain([second, first], &text) {
            Ok(reversed) => reversed,
            Err(error) => {
                file(
                    &mut findings.unsettled,
                    format!("{} was rejected: {error}", clause(later, earlier)),
                );
                continue;
            }
        };
        if runs_behind(later.as_str(), earlier.as_str())
            || memo.editing(&seats, &reversed).is_empty()
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
    scope_of(setting(RULES_VAR).as_deref())
}

/// The rules `named` lists, separated by spaces or commas.
fn scope_of(named: Option<&str>) -> Option<BTreeSet<RuleId>> {
    named.map(|named| {
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
    shard_of(setting(SHARD_VAR).as_deref())
}

/// The zero-based share and the share count `spec` names as `k/n`, the
/// whole set for no spec.
fn shard_of(spec: Option<&str>) -> (usize, usize) {
    let Some(spec) = spec else {
        return (0, 1);
    };
    let parsed = spec
        .split_once('/')
        .and_then(|(k, n)| k.parse::<usize>().ok().zip(n.parse::<usize>().ok()))
        .filter(|&(k, n)| k >= 1 && k <= n);
    let (k, n) =
        parsed.unwrap_or_else(|| panic!("{SHARD_VAR} takes `k/n` with 1 <= k <= n: {spec}"));
    (k - 1, n)
}

/// The text `pipeline` folds `text` into, `None` where the buffer does
/// not parse.
fn folded_text(pipeline: &Pipeline, text: &str) -> Option<Result<String, PipelineError>> {
    text.parse::<Source>()
        .ok()
        .map(|source| pipeline.format(source).map(|out| out.text().to_owned()))
}

/// True where `spliced` leaves `text` as the `chained` run did. A run
/// `spliced` rejects, and a buffer it cannot parse, both count as a
/// divergence.
fn spliced_matches(spliced: &Pipeline, text: &str, chained: &str) -> bool {
    matches!(folded_text(spliced, text), Some(Ok(out)) if out == chained)
}

/// A pipeline carrying exactly `rules`, bypassing each one's `enabled`
/// flag the way a `--select` run does.
fn subset(config: &Config, rules: &[RuleId]) -> Pipeline {
    Pipeline::with_filters(config, rules, &[])
}

/// Folds `probe`'s pair over `text` and panics where the fold's text or
/// its still-editing set differs from the `chained` single-rule runs.
fn verify_pair(
    memo: &mut Memo,
    path: &Path,
    text: &Rc<str>,
    probe: &Pair,
    chained: &Result<Rc<str>, Rc<str>>,
) {
    let (folded, [earlier, later], seats) = (&probe.folded, probe.rules, probe.seats);
    let old = folded_text(folded, text);
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

#[rstest]
#[case(None)]
#[case(Some("touching"))]
fn claim_of_reaches_every_pair_touching_the_scope(#[case] named: Option<&str>) {
    assert_matches!(claim_of(named), Claim::Touching);
}

#[test]
fn claim_of_reads_the_owned_shape() {
    assert_matches!(claim_of(Some("owned")), Claim::Owned);
}

#[rstest]
#[case("both")]
#[case("Owned")]
#[should_panic(expected = "takes `owned` or `touching`")]
fn claim_of_rejects_an_unknown_shape(#[case] named: &str) {
    let _ = claim_of(Some(named));
}

#[test]
#[cfg_attr(coverage, ignore = "the sweep runs uninstrumented in its own row")]
fn every_rule_subset_settles_and_declares_its_seating() {
    let files = corpus();
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
        findings.absorb(swept(&files, |path| probe(&probes, path)));
    }
    report_verified("chained pairs against the two-rule fold");
    if pointed_corpus().is_some() {
        eprintln!("{}", findings.render_sharing());
    }
    let unread = unread(findings.skipped.len(), files.len(), "probe");
    let report = format!(
        "{}{}{}",
        findings.unsettled.render("unsettled subsets"),
        findings.undeclared.render("undeclared seatings"),
        findings
            .divergent
            .render("declared independence the chained run contradicts"),
    );
    assert!(
        report.is_empty(),
        "{} distinct defects across the corpus's {} files{unread}, swept at \
         `code-line-length` {}:{report}",
        findings.total(),
        files.len(),
        lengths.iter().format(", "),
    );
}

#[test]
fn scope_of_answers_every_rule_for_no_setting() {
    assert!(scope_of(None).is_none());
}

#[rstest]
#[case("align-equals band-constants")]
#[case("align-equals,band-constants")]
#[case("align-equals, band-constants")]
fn scope_of_splits_on_spaces_and_commas(#[case] named: &str) {
    let named = scope_of(Some(named)).expect("a named set scopes the sweep");

    assert_eq!(
        named,
        BTreeSet::from([
            RuleId::from_str("align-equals").unwrap(),
            RuleId::from_str("band-constants").unwrap(),
        ])
    );
}

#[rstest]
#[case(None, (0, 1))]
#[case(Some("1/1"), (0, 1))]
#[case(Some("2/3"), (1, 3))]
#[case(Some("3/3"), (2, 3))]
fn shard_of_counts_its_share_from_one(#[case] spec: Option<&str>, #[case] share: (usize, usize)) {
    assert_eq!(shard_of(spec), share);
}

#[rstest]
#[case("0/3")]
#[case("4/3")]
#[case("2")]
#[case("k/n")]
#[should_panic(expected = "takes `k/n`")]
fn shard_of_rejects_a_share_outside_one_through_n(#[case] spec: &str) {
    let _ = shard_of(Some(spec));
}
