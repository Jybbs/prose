//! Predicts the column an alignment rule shifts each assignment,
//! keyword, and parameter-default value to, so a layout decision tests
//! a construct against the position it lands at after alignment rather
//! than its current one, and reads that prediction back per offset
//! through [`Columns`]. The runs are built the way `align_equals` builds
//! them, a statement or keyword whose value spans lines closing its run
//! and a held statement staying transparent, the widenings the rule's
//! other groups seat on a line read the way the rule reads them, and a
//! row whose value a later rule joins onto it measured at that joined
//! width, so the prediction and the rule seat the same rows. No column
//! is reserved for a value inside an f-string or t-string replacement
//! field.

use std::hash::BuildHasher;

use ruff_diagnostics::SourceMap;
use ruff_python_ast::{
    AnyNodeRef, Expr, InterpolatedStringElement, Stmt,
    comparable::ComparableParameters,
    visitor::{Visitor, walk_expr},
};
use ruff_text_size::{Ranged, TextRange, TextSize};
#[cfg(test)]
use rustc_hash::FxHashSet;
use rustc_hash::{FxBuildHasher, FxHashMap};

use crate::{
    primitives::{
        aligner,
        call_keywords::{CallTargets, module_call_params},
        edit::{forward_range, forward_start},
        equal_targets,
        inline::display_width,
        one_row,
        range::overlaps,
        slots::item_holding,
        walk,
    },
    rules::RuleId,
    source::Source,
};

mod visit;

use visit::{ReserveVisitor, widenings_over};

/// The columns each aligned value shifts by once the alignment
/// settles, one entry per reservation ascending by start, each carrying
/// the span from the value's own start to the end of its physical row.
/// Reading a shift over a span rather than a column at one offset lets
/// a construct nested inside an aligned value move with it, and lets
/// the shift compose with a caller's own placement rather than
/// replacing it.
#[derive(Clone, Debug)]
pub(crate) struct Columns {
    /// The gap an aligned row holds ahead of its operator, `None` where
    /// the alignment rule is off.
    buffer: Option<usize>,
    /// The statement each run formed inside, indexed by the run each
    /// shift names, the module itself for a module-body run.
    runs: Vec<TextRange>,
    shifts: Vec<Shift>,
    /// Each in-module call's callee offset and the hash of the
    /// parameters it resolves to, ascending, the map every joined
    /// width read, so a rebuild can tell whether that map still holds.
    targets: Vec<(TextSize, u64)>,
    /// The widening each run's members seat on their lines, keyed by
    /// the run.
    widenings: Vec<(usize, aligner::Widening)>,
}

impl Columns {
    /// The columns the alignment moves `offset` by, zero where no
    /// reservation covers it. A reservation never spans a row, so the
    /// nearest one starting at or before `offset` is the only candidate.
    fn shift(&self, offset: TextSize) -> isize {
        item_holding(&self.shifts, offset)
            .filter(|shift| shift.span.contains(offset))
            .map_or(0, |shift| shift.columns)
    }

    /// The column `offset` lands at, `fallback` moved by the shift the
    /// alignment applies to the row `offset` sits on.
    pub(crate) fn column(&self, offset: TextSize, fallback: impl FnOnce() -> usize) -> usize {
        fallback().saturating_add_signed(self.shift(offset))
    }

    /// The column `offset` lands at, falling back to the column its own
    /// source line puts it at.
    pub(crate) fn column_in(&self, source: &Source, offset: TextSize) -> usize {
        self.column(offset, || source.column_of(offset))
    }

    /// The column the value of a keyword `name_width` wide lands at
    /// once the alignment buffers it, the keyword sitting alone on its
    /// row at `indent`. The value follows the name by the buffer, the
    /// `=` itself, and the one-space value gap, which is the floor a
    /// lone row settles at and the column a run resolving within the
    /// line cap leaves it at. `None` where the alignment rule is off,
    /// leaving the value where its row writes it.
    pub(crate) fn keyword_value_column(&self, indent: usize, name_width: usize) -> Option<usize> {
        self.buffer
            .map(|buffer| indent + name_width + buffer + aligner::VALUE_OFFSET)
    }

    /// Each run as its scope, its shifts, and its widenings, ascending,
    /// the form two tables compare in whatever order their runs were
    /// numbered.
    #[cfg(test)]
    fn canonical(&self) -> Vec<CanonicalRun> {
        let mut runs: Vec<_> = self
            .runs
            .iter()
            .enumerate()
            .map(|(run, &scope)| {
                let mut shifts: Vec<(TextRange, isize)> = self
                    .shifts
                    .iter()
                    .filter(|shift| shift.run == run)
                    .map(|shift| (shift.span, shift.columns))
                    .collect();
                shifts.sort_unstable_by_key(|&(span, _)| span.start());
                let mut widenings: Vec<aligner::Widening> = self
                    .widenings
                    .iter()
                    .filter(|&&(owner, _)| owner == run)
                    .map(|&(_, entry)| entry)
                    .collect();
                widenings.sort_unstable_by_key(|&(line, gap, _)| (line, gap.start()));
                (scope, shifts, widenings)
            })
            .collect();
        runs.sort_unstable_by_key(|(scope, shifts, _)| {
            (scope.start(), shifts.first().map(|&(span, _)| span.start()))
        });
        runs
    }

    /// The table an alignment rule that is off leaves, reserving no
    /// column.
    fn unreserved() -> Self {
        Self {
            buffer: None,
            runs: Vec::new(),
            shifts: Vec::new(),
            targets: Vec::new(),
            widenings: Vec::new(),
        }
    }

    /// The shifts of this table a splice over `map` cannot carry into
    /// `fresh`, the table a fresh read of the spliced source builds. A
    /// run whose scope a `held` window reaches is re-formed rather than
    /// carried, its scope moved through `slide` to name the fresh run
    /// that replaces it, as is a fresh run a `slid` window reaches.
    /// Every other shift is carried through `map`, and an escape is a
    /// carried shift `fresh` holds at another column or not at all, or
    /// a fresh shift outside every re-formed run that no carried shift
    /// lands on. Each names its span and what went wrong.
    #[cfg(test)]
    pub(crate) fn escapes(
        &self,
        fresh: &Columns,
        map: &SourceMap,
        held: &[TextRange],
        slid: &[TextRange],
        slide: impl Fn(TextRange) -> TextRange,
    ) -> Vec<String> {
        let reformed: FxHashSet<TextRange> = self
            .runs
            .iter()
            .filter(|scope| overlaps(**scope, held))
            .map(|&scope| slide(scope))
            .collect();
        let mut escapes = Vec::new();
        let mut landed = FxHashSet::default();
        for shift in &self.shifts {
            if overlaps(self.runs[shift.run], held) {
                continue;
            }
            let Some(span) = forward_range(shift.span, map) else {
                escapes.push(format!(
                    "{:?} replaced by an edit outside its run's scope",
                    shift.span
                ));
                continue;
            };
            landed.insert(span);
            match fresh
                .shifts
                .binary_search_by_key(&span.start(), Ranged::start)
            {
                Ok(at) if fresh.shifts[at].span == span => {
                    if fresh.shifts[at].columns != shift.columns {
                        escapes.push(format!(
                            "{span:?} moved from {} to {} columns",
                            shift.columns, fresh.shifts[at].columns
                        ));
                    }
                }
                _ => escapes.push(format!("{span:?} carried where the fresh table holds none")),
            }
        }
        for shift in &fresh.shifts {
            let scope = fresh.runs[shift.run];
            if reformed.contains(&scope) || overlaps(scope, slid) || landed.contains(&shift.span) {
                continue;
            }
            escapes.push(format!(
                "{:?} fresh where no carried shift lands",
                shift.span
            ));
        }
        escapes
    }
}

/// The alignment a layout rule measures against, resolved from
/// configuration once and carried as a value. `settings` is `None`
/// where the alignment rule is off, leaving every column unreserved,
/// and `one_row` names the terms a value joins onto its row under.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Reservations {
    one_row: one_row::Settings<'static>,
    rule: RuleId,
    settings: Option<aligner::Settings>,
}

impl Reservations {
    /// The reservation for `rule` running under `settings`, each value
    /// joined under `one_row`.
    pub(crate) fn new(
        rule: RuleId,
        settings: Option<aligner::Settings>,
        one_row: one_row::Settings<'static>,
    ) -> Self {
        Self {
            one_row,
            rule,
            settings,
        }
    }

    /// Walks `source` collecting the runs the reserved rule builds,
    /// descending only into the statements a `windows` span reaches
    /// where one is given.
    fn collected<'a>(
        &self,
        source: &'a Source,
        windows: Option<&'a [TextRange]>,
    ) -> ReserveVisitor<'a> {
        let mut visitor = ReserveVisitor {
            rule: self.rule,
            runs: Vec::new(),
            source,
            stmt: source.module_range(),
            values: FxHashMap::default(),
            windows,
        };
        visitor.visit_body(&source.ast().body);
        visitor
    }

    /// Maps each aligned value's start offset to the display column it
    /// lands at once the run is aligned. A value the run leaves where it
    /// sits maps to that same column, so a lookup is a no-op for a value
    /// the alignment does not move.
    pub(crate) fn columns(self, source: &Source) -> Columns {
        let Some(settings) = self.settings else {
            return Columns::unreserved();
        };
        let targets = module_call_params(source);
        let visitor = self.collected(source, None);
        self.formed(source, settings, &visitor, Carried::default(), &targets)
    }

    /// `carried`, the table the source before a splice held, moved
    /// through `map` past that splice: every run whose scope no `held`
    /// window reaches keeps its shifts and widenings, forwarded, and
    /// the runs the windows reached re-form over this source by walking
    /// the statements a `slid` window reaches. `None` where the
    /// module's call targets no longer hold outside the windows or an
    /// edit replaced a carried span, leaving a fresh build to the next
    /// read.
    pub(crate) fn rebuilt(
        self,
        source: &Source,
        carried: &Columns,
        map: &SourceMap,
        held: &[TextRange],
        slid: &[TextRange],
    ) -> Option<Columns> {
        let Some(settings) = self.settings else {
            return Some(Columns::unreserved());
        };
        let targets = module_call_params(source);
        if !targets_hold(&carried.targets, &targets, map, held, slid) {
            return None;
        }
        let mut forwarded = Carried::default();
        let mut slots: Vec<Option<usize>> = vec![None; carried.runs.len()];
        for (run, &scope) in carried.runs.iter().enumerate() {
            if overlaps(scope, held) {
                continue;
            }
            slots[run] = Some(forwarded.runs.len());
            forwarded.runs.push(forward_range(scope, map)?);
        }
        for shift in &carried.shifts {
            let Some(run) = slots[shift.run] else {
                continue;
            };
            forwarded.shifts.push(Shift {
                columns: shift.columns,
                run,
                span: forward_range(shift.span, map)?,
            });
        }
        for &(run, (line, gap, delta)) in &carried.widenings {
            let Some(run) = slots[run] else {
                continue;
            };
            let line = forward_start(line, map)?;
            forwarded
                .widenings
                .push((run, (line, forward_range(gap, map)?, delta)));
        }
        let visitor = self.collected(source, Some(slid));
        Some(self.formed(source, settings, &visitor, forwarded, &targets))
    }

    /// The table `carried` grows into once `visitor`'s runs form on top
    /// of it: each run's scope, widening entries, and shifts, every
    /// joined width read against `targets`.
    fn formed(
        self,
        source: &Source,
        settings: aligner::Settings,
        visitor: &ReserveVisitor,
        carried: Carried,
        targets: &CallTargets,
    ) -> Columns {
        let Carried {
            mut runs,
            mut shifts,
            mut widenings,
        } = carried;
        let base = runs.len();
        for (index, run) in visitor.runs.iter().enumerate() {
            runs.push(run.scope);
            let entries = aligner::widening_entries(source, settings, run.members.iter().copied());
            widenings.extend(entries.into_iter().map(|entry| (base + index, entry)));
        }
        let seated =
            aligner::Widenings::from_entries(widenings.iter().map(|&(_, entry)| entry).collect());
        let one_row = self.one_row.against(targets);
        let place = |member: aligner::Member| {
            let start = member.rewritten_value_gap(source)?.end();
            Some((start, source.column_of(start)))
        };
        let joined = |(start, column): (TextSize, usize)| {
            let &(expr, parent) = visitor.values.get(&start)?;
            let end = source.paren_aware_range(expr.into(), parent).end();
            let tail = source.row_tail_width(end);
            let form = one_row.rejoined(source, expr, parent, column, tail)?;
            Some(column + display_width(&form) + tail)
        };
        for (index, run) in visitor.runs.iter().enumerate() {
            if run.candidate && !aligner::is_alignment_candidate(&run.members) {
                continue;
            }
            let placed: Vec<Option<(TextSize, usize)>> =
                run.members.iter().map(|&m| place(m)).collect();
            let joined: Vec<Option<usize>> = placed.iter().map(|&at| joined(at?)).collect();
            let columns =
                aligner::operator_columns(source, &run.members, settings, &seated, &joined);
            shifts.extend(placed.iter().zip(columns).filter_map(|(&placed, column)| {
                let (start, at) = placed?;
                Some(Shift {
                    columns: (column + aligner::VALUE_OFFSET).cast_signed() - at.cast_signed(),
                    run: base + index,
                    span: source.row_tail(start),
                })
            }));
        }
        shifts.sort_unstable_by_key(Ranged::start);
        Columns {
            buffer: Some(settings.buffer()),
            runs,
            shifts,
            targets: hashed(targets),
            widenings,
        }
    }

    /// The widening the reserved rule seats on each line, empty where
    /// that rule is off. A rule deciding a column ahead of the reserved
    /// one reads this so its line-cap check measures a row at the width
    /// the reserved rule leaves rather than the width the source
    /// carries.
    pub(crate) fn widenings(&self, source: &Source) -> aligner::Widenings {
        let Some(settings) = self.settings else {
            return aligner::Widenings::default();
        };
        widenings_over(source, settings, &self.collected(source, None))
    }
}

/// One run as its scope, its shifts as span and columns, and its
/// widenings, the form [`Columns::canonical`] lists.
#[cfg(test)]
type CanonicalRun = (TextRange, Vec<(TextRange, isize)>, Vec<aligner::Widening>);

/// Two tables are equal where they reserve the same columns over the
/// same runs, whatever order their runs were numbered in.
#[cfg(test)]
impl PartialEq for Columns {
    fn eq(&self, other: &Self) -> bool {
        self.buffer == other.buffer
            && self.targets == other.targets
            && self.canonical() == other.canonical()
    }
}

/// The runs, shifts, and widenings a rebuild carries past a splice,
/// which a fresh build starts empty.
#[derive(Default)]
struct Carried {
    runs: Vec<TextRange>,
    shifts: Vec<Shift>,
    widenings: Vec<(usize, aligner::Widening)>,
}

/// Each of `targets` as its callee offset and the hash of the
/// parameters it resolves to, ascending by offset.
fn hashed(targets: &CallTargets) -> Vec<(TextSize, u64)> {
    let hasher = FxBuildHasher;
    let mut hashed: Vec<(TextSize, u64)> = targets
        .iter()
        .map(|(&offset, &params)| (offset, hasher.hash_one(ComparableParameters::from(params))))
        .collect();
    hashed.sort_unstable_by_key(|&(offset, _)| offset);
    hashed
}

/// True where the call targets a splice over `map` leaves outside its
/// windows are the ones `carried` recorded, so every joined width read
/// against them still holds: each carried target outside a `held`
/// window forwards to a fresh one resolving the same parameters, and
/// no fresh target outside a `slid` window is new.
fn targets_hold(
    carried: &[(TextSize, u64)],
    fresh: &CallTargets,
    map: &SourceMap,
    held: &[TextRange],
    slid: &[TextRange],
) -> bool {
    let inside = |offset: TextSize, windows: &[TextRange]| {
        item_holding(windows, offset).is_some_and(|window| window.contains(offset))
    };
    let mut expected: FxHashMap<TextSize, u64> = FxHashMap::default();
    for &(offset, hash) in carried {
        if inside(offset, held) {
            continue;
        }
        let Some(offset) = forward_start(offset, map) else {
            return false;
        };
        expected.insert(offset, hash);
    }
    hashed(fresh)
        .into_iter()
        .filter(|&(offset, _)| !inside(offset, slid))
        .all(|(offset, hash)| expected.remove(&offset) == Some(hash))
        && expected.is_empty()
}

/// One reservation's row-tail span, the columns the alignment shifts
/// it by, and the run it belongs to.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Shift {
    columns: isize,
    run: usize,
    span: TextRange,
}

impl Ranged for Shift {
    fn range(&self) -> TextRange {
        self.span
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use rstest::rstest;

    use super::*;
    use crate::{
        config::{AlignmentConfig, Config},
        testing::parse,
    };

    /// The reservation table an `align-equals` run under `settings`
    /// reads back, built over a source carrying one assignment.
    fn columns_under(settings: Option<aligner::Settings>) -> Columns {
        Reservations::new(
            RuleId::from("align-equals"),
            settings,
            one_row::Settings::from(&Config::default()),
        )
        .columns(&parse("a = 1\n"))
    }

    /// The column each value in `text` lands at under the default
    /// configuration capped at `line_length`, one entry per `values`
    /// offset.
    fn landed(text: &str, line_length: usize, values: &[u32]) -> Vec<usize> {
        let source = parse(text);
        let config = Config {
            code_line_length: NonZeroUsize::new(line_length),
            ..Config::default()
        };
        let columns = config.equals_reservations().columns(&source);
        values
            .iter()
            .map(|&offset| columns.column_in(&source, TextSize::new(offset)))
            .collect()
    }

    #[test]
    fn columns_count_a_keyword_the_rule_widens_on_the_same_line() {
        // Aligning `x` to `longer` lands its line on 15 columns, inside
        // a cap of 16 until the stacked `k=1` keyword the rule buffers
        // to `k = 1` widens it past, so the run breaks and `x` stays put.
        let text = "longer = 2\nx = f(k=1,\n      j=2)\n";
        assert_eq!(landed(text, 16, &[15]), vec![4]);
        assert_eq!(landed(text, 18, &[15]), vec![9]);
    }

    #[test]
    fn columns_measure_a_value_a_later_rule_joins_at_its_joined_width() {
        // `[1234]` joins onto `b`'s row at 10 columns, which fits a cap
        // of 12 where the row stands but not three columns over at
        // `aaaa`'s column, so the run breaks and `b`'s value stays put
        // where its opening line alone would have fit.
        let text = "aaaa = 2\nb = [\n    1234\n]\n";
        assert_eq!(landed(text, 12, &[13]), vec![4]);
        assert_eq!(landed(text, 16, &[13]), vec![7]);
    }

    #[test]
    fn columns_reserve_a_parameter_run_only_where_it_is_a_candidate() {
        let stacked = "def f(\n    a: int = 1,\n    bbb: str = \"\",\n):\n    pass\n";
        assert_eq!(landed(stacked, 88, &[20]), vec![15]);
        let packed = "def f(a: int = 1, bbb: str = \"\"):\n    pass\n";
        assert_eq!(landed(packed, 88, &[15]), vec![15]);
    }

    #[test]
    fn columns_shift_each_value_to_the_run_column() {
        // `a`'s value follows `bbb`'s to column 6 while `bbb`'s stays.
        assert_eq!(landed("a = 1\nbbb = 2\n", 88, &[4, 12]), vec![6, 6]);
    }

    #[test]
    fn keyword_value_column_answers_none_where_the_alignment_is_off() {
        assert_eq!(columns_under(None).keyword_value_column(4, 5), None);
    }

    #[rstest]
    #[case::at_the_margin(0, 3, 6)]
    #[case::one_indent_step(4, 5, 12)]
    #[case::wide_name(8, 12, 23)]
    fn keyword_value_column_seats_the_value_past_the_buffer_and_the_operator(
        #[case] indent: usize,
        #[case] name: usize,
        #[case] expected: usize,
    ) {
        let settings = aligner::Settings::from(&AlignmentConfig::default());
        assert_eq!(
            columns_under(Some(settings)).keyword_value_column(indent, name),
            Some(expected),
        );
    }
}
