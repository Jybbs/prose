//! Predicts the column an alignment rule shifts each assignment,
//! keyword, and parameter-default value to, and reads that prediction
//! back per offset through [`Columns`]. Runs form the way `align_equals`
//! forms them over the source the layout and reorder rules leave: a
//! value those rules join onto one row extends its run, one they break
//! open to join its column ends it, a body reads in the order a reorder
//! rule seats it, and an annotated row shifts by the padding
//! `align-colons` gives its `:`. No column is reserved for a value
//! inside an f-string or t-string replacement field.

use std::ops::Range;

use ruff_diagnostics::SourceMap;
use ruff_python_ast::{
    AnyNodeRef, Expr, InterpolatedStringElement, Stmt,
    visitor::{Visitor, walk_expr},
};
use ruff_text_size::{Ranged, TextRange, TextSize};
use rustc_hash::FxHashMap;
#[cfg(test)]
use rustc_hash::FxHashSet;

use crate::{
    config::Config,
    primitives::{
        aligner,
        call_keywords::module_call_params,
        edit::{forward_range, forward_start},
        equal_targets, one_row,
        orderer::Seatings,
        padding::{self, Stranding},
        range::{covers, overlaps},
        scope::sub_bodies,
        slots::item_holding,
        walk,
    },
    rules::{
        RuleId, alphabetize_siblings::AlphabetizeSiblings, band_constants::BandConstants,
        prefer_fstring::PreferFstring,
    },
    source::Source,
};

mod measure;
mod visit;

use measure::Measure;
use visit::ReserveVisitor;

/// The table a splice carries into the source it produced: the runs
/// the edit could not reach, moved to where the woven text holds them,
/// and the completion that forms the rest on the first read.
#[derive(Clone, Debug)]
pub(crate) struct Carry {
    forwarded: Forwarded,
    reform: Reform,
}

/// The columns each aligned value shifts by once the alignment
/// settles, one entry per reservation ascending by start, each carrying
/// the span from the value's own start to the end of its physical row.
/// A construct nested inside an aligned value moves with it, and the
/// shift composes with a caller's own placement.
#[derive(Clone, Debug)]
pub(crate) struct Columns {
    /// The gap an aligned row holds ahead of its operator, `None` where
    /// the alignment rule is off.
    buffer: Option<usize>,
    /// Each run's scope, indexed by the run each shift names.
    runs: Vec<Scope>,
    shifts: Vec<Shift>,
    /// The widening each run's members seat on their lines, keyed by
    /// the run.
    widenings: Vec<(usize, aligner::Widening)>,
}

impl Columns {
    /// Builds an empty table, used when the alignment rule is off and no
    /// column is reserved.
    fn unreserved() -> Self {
        Self {
            buffer: None,
            runs: Vec::new(),
            shifts: Vec::new(),
            widenings: Vec::new(),
        }
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
            .map(|(run, scope)| {
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
                (scope.stmt, shifts, widenings)
            })
            .collect();
        runs.sort_unstable_by_key(|(scope, shifts, _)| {
            (scope.start(), shifts.first().map(|&(span, _)| span.start()))
        });
        runs
    }

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
            .filter(|scope| overlaps(scope.stmt, held))
            .map(|scope| slide(scope.stmt))
            .collect();
        let mut escapes = Vec::new();
        let mut landed = FxHashSet::default();
        for shift in &self.shifts {
            if overlaps(self.runs[shift.run].stmt, held) {
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
            let scope = fresh.runs[shift.run].stmt;
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
}

/// Two tables are equal where they reserve the same columns over the
/// same runs, whatever order their runs were numbered in.
#[cfg(test)]
impl PartialEq for Columns {
    fn eq(&self, other: &Self) -> bool {
        self.buffer == other.buffer && self.canonical() == other.canonical()
    }
}

/// The slices a carried table's completion forms a body's runs over
/// and the windows it descends into, both in the completed source.
/// An entry pairs a body's owning statement, the module range at top
/// level, with the span of the siblings whose runs the splice reached.
#[derive(Clone, Debug)]
pub(crate) struct Reform {
    entries: Vec<(TextRange, TextRange)>,
    windows: Vec<TextRange>,
}

impl Reform {
    /// The index ranges of `body`, owned by `owner`, whose runs the
    /// completion forms: every statement where a window covers the
    /// owner whole, and otherwise the statements overlapping the spans
    /// entered for the owner, each maximal stretch of them one slice.
    fn slices(&self, owner: TextRange, body: &[Stmt]) -> Vec<Range<usize>> {
        if covers(owner, &self.windows) {
            return std::iter::once(0..body.len()).collect();
        }
        let spans: Vec<TextRange> = self
            .entries
            .iter()
            .filter(|(key, _)| *key == owner)
            .map(|&(_, span)| span)
            .collect();
        let mut slices: Vec<Range<usize>> = Vec::new();
        for (index, stmt) in body.iter().enumerate() {
            if !spans.iter().any(|span| span.ordering(stmt.range()).is_eq()) {
                continue;
            }
            match slices.last_mut() {
                Some(last) if last.end == index => last.end = index + 1,
                _ => slices.push(index..index + 1),
            }
        }
        slices
    }
}

/// The alignment a layout rule measures against, resolved from
/// configuration once and carried as a value. `settings` is `None`
/// where the alignment rule is off, leaving every column unreserved,
/// `one_row` names the terms a value joins onto its row or breaks open
/// under, `fstrings` the rewrites each value is measured through,
/// `stranding` the padding rule whose deletions each row settles past,
/// `bands` and `sorts` the rules whose seating each body is read in, and
/// `colons` the settings `align-colons` pads an annotated row's `:`
/// under, each `None` where that rule is off.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Reservations {
    bands: Option<BandConstants>,
    colons: Option<aligner::Settings>,
    fstrings: PreferFstring,
    one_row: one_row::Settings<'static>,
    rule: RuleId,
    settings: Option<aligner::Settings>,
    sorts: Option<AlphabetizeSiblings>,
    stranding: Stranding,
}

impl Reservations {
    /// Builds the reservation for `rule` running under `settings`, reading
    /// every other term off `config`.
    pub(crate) fn new(rule: RuleId, settings: Option<aligner::Settings>, config: &Config) -> Self {
        Self {
            bands: config.band_forecast(),
            colons: config.colon_forecast(),
            fstrings: config.fstrings(),
            one_row: config.one_row_settings(),
            rule,
            settings,
            sorts: config
                .alphabetize_siblings_enabled()
                .then(|| AlphabetizeSiblings::from_config(config)),
            stranding: config.stranded_padding(),
        }
    }

    /// Walks `source` collecting the runs the reserved rule builds, over the
    /// whole tree or, given `reform`, over the slices and windows a carried
    /// table's completion names. The completion forms whole each body holding
    /// one of the `seated` runs, which name their owner and row span, and each
    /// statement whose value `measure` joins or breaks open extends its run.
    fn collected<'a>(
        &self,
        source: &'a Source,
        measure: &'a Measure<'a>,
        reform: Option<&'a Reform>,
        seated: &'a [(TextRange, TextRange)],
    ) -> ReserveVisitor<'a> {
        let mut visitor = ReserveVisitor {
            colons: Vec::new(),
            measure,
            reform,
            rule: self.rule,
            runs: Vec::new(),
            seated,
            source,
            stmt: source.module_range(),
            stranding: self.stranding,
            values: FxHashMap::default(),
            whole: Vec::new(),
        };
        visitor.visit_body(&source.ast().body);
        visitor
    }

    /// Builds the table `carried` grows into once `visitor`'s runs form on
    /// top of it, adding each run's scope, widening entries, and shifts, with
    /// every joined and broken width read through `measure`. A statement run
    /// cuts after each value a layout rule breaks open, per
    /// [`aligner::breaking_columns`], each annotated row shifted by the
    /// padding `align-colons` gives its `:`. Each colon run adds a scope
    /// holding no shift, so a splice reaching one of its rows forms every
    /// statement run it shifts afresh.
    fn formed(
        &self,
        settings: aligner::Settings,
        measure: &Measure,
        visitor: &ReserveVisitor,
        carried: Forwarded,
    ) -> Columns {
        let source = measure.source;
        let Forwarded {
            mut runs,
            mut shifts,
            mut widenings,
        } = carried.without(&visitor.whole);
        let base = runs.len();
        for (index, run) in visitor.runs.iter().enumerate() {
            runs.push(Scope::of(run));
            let entries = aligner::widening_entries(source, settings, run.members.iter().copied());
            widenings.extend(entries.into_iter().map(|entry| (base + index, entry)));
        }
        runs.extend(visitor.colons.iter().map(Scope::of));
        let seated =
            aligner::Widenings::from_entries(widenings.iter().map(|&(_, entry)| entry).collect());
        let colons = measure.colon_shifts(&visitor.colons, &seated);
        let place = |member: aligner::Member| {
            let start = member.rewritten_value_gap(source)?.end();
            Some((start, source.column_of(start)))
        };
        let value = |(start, column): (TextSize, usize)| {
            let &(expr, parent) = visitor.values.get(&start)?;
            Some((expr, parent, column))
        };
        for (index, run) in visitor.runs.iter().enumerate() {
            if run.candidate && !aligner::is_alignment_candidate(&run.members) {
                continue;
            }
            let placed: Vec<Option<(TextSize, usize)>> =
                run.members.iter().map(|&m| place(m)).collect();
            let values: Vec<_> = placed.iter().map(|&at| at.and_then(value)).collect();
            let joined: Vec<Option<usize>> = values
                .iter()
                .map(|&found| {
                    let (expr, parent, column) = found?;
                    measure.joined(expr, parent, column, run.body)
                })
                .collect();
            let columns = if run.body {
                let statements: Vec<aligner::Statement> = values
                    .iter()
                    .zip(&run.members)
                    .map(|(&found, member)| {
                        let colon = colons.get(&member.line_start).copied().unwrap_or_default();
                        let row = found.and_then(|(expr, parent, column)| {
                            measure.breakable(expr, parent, column)
                        });
                        aligner::Statement {
                            breaks: row.map(|(breaks, _)| breaks),
                            holds: colon.holds || row.is_some_and(|(_, inside)| inside),
                            shift: colon.shift,
                        }
                    })
                    .collect();
                aligner::breaking_columns(
                    source,
                    &run.members,
                    settings,
                    &seated,
                    &joined,
                    &statements,
                )
            } else {
                aligner::operator_columns(source, &run.members, settings, &seated, &joined)
            };
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
            widenings,
        }
    }

    /// Calls `read` with the [`Measure`] this reservation reads `source`'s
    /// values under, each body the reorder rules seat other than as written
    /// read in that seating.
    fn measured<R>(&self, source: &Source, read: impl FnOnce(&Measure) -> R) -> R {
        let targets = module_call_params(source);
        let rewrites = source.fstring_rewrites(self.fstrings);
        let stranded = source.stranded_padding(self.stranding);
        let padding = padding::beside(&stranded, &rewrites);
        let seatings: Seatings = self
            .bands
            .iter()
            .map(|bands| bands.seatings(source))
            .chain(self.sorts.iter().map(|sorts| sorts.seatings(source)))
            .flatten()
            .collect();
        read(&Measure {
            colons: self.colons,
            one_row: self.one_row.against(&targets).forecasting(&rewrites),
            padding: &padding,
            seatings: &seatings,
            source,
        })
    }

    /// What a splice over `held` carries of `carried`, the table over
    /// `source`, the text before the splice: every run the edit could
    /// not reach, moved through `map` and the slides, and the
    /// completion forming the rest. A statement run stays where its
    /// rows sit outside the neighborhood of the siblings a window
    /// reaches, that neighborhood being one sibling either side widened
    /// to the full extent of any run it cuts, and a keyword or
    /// parameter run stays where no window reaches its statement.
    /// `None` where an edit replaced a carried row or swallowed a
    /// carried run's opening, leaving a fresh build to the first read.
    pub(crate) fn carry(&self, source: &Source, carried: &Columns, weave: &Weave) -> Option<Carry> {
        let &Weave {
            held,
            map,
            slid,
            slide_span,
            slide_stmt,
        } = weave;
        let module = source.module_range();
        let mut entries = Vec::new();
        reform_entries(
            &source.ast().body,
            module,
            held,
            &carried.runs,
            &mut entries,
        );
        let dropped = |scope: &Scope| {
            if scope.body {
                entries
                    .iter()
                    .any(|&(owner, span)| owner == scope.stmt && span.ordering(scope.span).is_eq())
            } else {
                overlaps(scope.stmt, held)
            }
        };
        let slide_owner = |stmt: TextRange| {
            if stmt == module {
                Some(slide_span(stmt))
            } else {
                slide_stmt(stmt)
            }
        };
        let mut forwarded = Forwarded::default();
        let mut slots: Vec<Option<usize>> = vec![None; carried.runs.len()];
        for (run, scope) in carried.runs.iter().enumerate() {
            if dropped(scope) {
                continue;
            }
            slots[run] = Some(forwarded.runs.len());
            forwarded.runs.push(Scope {
                body: scope.body,
                seated: scope.seated,
                span: forward_range(scope.span, map)?,
                stmt: slide_owner(scope.stmt)?,
            });
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
        Some(Carry {
            forwarded,
            reform: Reform {
                entries: entries
                    .into_iter()
                    .map(|(owner, span)| Some((slide_owner(owner)?, slide_span(span))))
                    .collect::<Option<Vec<_>>>()?,
                windows: slid.to_vec(),
            },
        })
    }

    /// Maps each aligned value's start offset to the display column it
    /// lands at once the run is aligned. A value the run leaves where it
    /// sits maps to that same column, so a lookup is a no-op for a value
    /// the alignment does not move.
    pub(crate) fn columns(&self, source: &Source) -> Columns {
        let Some(settings) = self.settings else {
            return Columns::unreserved();
        };
        self.measured(source, |measure| {
            let visitor = self.collected(source, measure, None, &[]);
            self.formed(settings, measure, &visitor, Forwarded::default())
        })
    }

    /// Builds the table `carry` completes to over `source`, the text the
    /// splice produced, laying the carried runs, less those of each body the
    /// completion forms whole, on top of the runs it forms.
    pub(crate) fn completed(&self, source: &Source, carry: &Carry) -> Columns {
        let Some(settings) = self.settings else {
            return Columns::unreserved();
        };
        let seated: Vec<(TextRange, TextRange)> = carry
            .forwarded
            .runs
            .iter()
            .filter(|scope| scope.body && scope.seated)
            .map(|scope| (scope.stmt, scope.span))
            .collect();
        self.measured(source, |measure| {
            let visitor = self.collected(source, measure, Some(&carry.reform), &seated);
            self.formed(settings, measure, &visitor, carry.forwarded.clone())
        })
    }

    /// Returns the widening the reserved rule seats on each line, read
    /// from the entries `source`'s column table holds for every run, and
    /// empty where that rule is off.
    pub(crate) fn widenings(&self, source: &Source) -> aligner::Widenings {
        let columns = source.columns(self);
        aligner::Widenings::from_entries(
            columns.widenings.iter().map(|&(_, entry)| entry).collect(),
        )
    }
}

/// Where one run formed: the statement whose body holds a statement
/// run or whose expressions hold a keyword or parameter run, the module
/// range for a module-body run, and the rows its members span.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Scope {
    /// True for a run formed over a body's statements.
    body: bool,
    /// True for a run formed over a body a reorder rule seats other than
    /// as written.
    seated: bool,
    span: TextRange,
    stmt: TextRange,
}

impl Scope {
    /// Builds the scope `run` formed in, spanning its members' rows from the
    /// first one's line start to the end of the last one's gap, or the run's
    /// own scope where it holds no member.
    fn of(run: &visit::Run) -> Self {
        Self {
            body: run.body,
            seated: run.seated,
            span: run
                .members
                .iter()
                .map(|member| TextRange::new(member.line_start, member.gap.end()))
                .reduce(TextRange::cover)
                .unwrap_or(run.scope),
            stmt: run.scope,
        }
    }
}

/// The geometry of one splice, as a carry reads it: the weave its
/// edits describe, its windows in the buffer the source held and in the
/// text it produced, and the slides moving a statement and a span past
/// those edits.
pub(crate) struct Weave<'a> {
    pub(crate) held: &'a [TextRange],
    pub(crate) map: &'a SourceMap,
    pub(crate) slid: &'a [TextRange],
    pub(crate) slide_span: &'a dyn Fn(TextRange) -> TextRange,
    pub(crate) slide_stmt: &'a dyn Fn(TextRange) -> Option<TextRange>,
}

/// One run as its scope, its shifts as span and columns, and its
/// widenings, the form [`Columns::canonical`] lists.
#[cfg(test)]
type CanonicalRun = (TextRange, Vec<(TextRange, isize)>, Vec<aligner::Widening>);

/// The runs, shifts, and widenings a carry moves past a splice, which
/// a fresh build starts empty.
#[derive(Clone, Debug, Default)]
struct Forwarded {
    runs: Vec<Scope>,
    shifts: Vec<Shift>,
    widenings: Vec<(usize, aligner::Widening)>,
}

impl Forwarded {
    /// Drops from this carry every statement run formed over one of `bodies`,
    /// each named by its owner and range, along with the shifts and widenings
    /// those runs hold.
    fn without(self, bodies: &[(TextRange, TextRange)]) -> Self {
        if bodies.is_empty() {
            return self;
        }
        let mut runs = Vec::with_capacity(self.runs.len());
        let slots: Vec<Option<usize>> = self
            .runs
            .into_iter()
            .map(|scope| {
                let dropped = scope.body
                    && bodies.iter().any(|&(owner, range)| {
                        scope.stmt == owner && range.contains_range(scope.span)
                    });
                (!dropped).then(|| {
                    runs.push(scope);
                    runs.len() - 1
                })
            })
            .collect();
        Self {
            runs,
            shifts: self
                .shifts
                .into_iter()
                .filter_map(|shift| {
                    Some(Shift {
                        run: slots[shift.run]?,
                        ..shift
                    })
                })
                .collect(),
            widenings: self
                .widenings
                .into_iter()
                .filter_map(|(run, entry)| Some((slots[run]?, entry)))
                .collect(),
        }
    }
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

/// Appends to `entries` the span of the siblings whose statement runs
/// the splice can change, for `body` owned by `owner` and each body
/// beneath a statement a `held` window reaches: each maximal stretch
/// of reached siblings with one sibling either side, widened to the
/// extent of any run of `runs` that stretch cuts.
fn reform_entries(
    body: &[Stmt],
    owner: TextRange,
    held: &[TextRange],
    runs: &[Scope],
    entries: &mut Vec<(TextRange, TextRange)>,
) {
    let reached: Vec<bool> = body
        .iter()
        .map(|stmt| overlaps(stmt.range(), held))
        .collect();
    let joined = |a: &Stmt, b: &Stmt| {
        runs.iter().any(|run| {
            run.body
                && run.stmt == owner
                && run.span.ordering(a.range()).is_eq()
                && run.span.ordering(b.range()).is_eq()
        })
    };
    let mut index = 0;
    while index < body.len() {
        if !reached[index] {
            index += 1;
            continue;
        }
        let first = index;
        while index + 1 < body.len() && reached[index + 1] {
            index += 1;
        }
        let mut lo = first.saturating_sub(1);
        let mut hi = (index + 1).min(body.len() - 1);
        while lo > 0 && joined(&body[lo - 1], &body[lo]) {
            lo -= 1;
        }
        while hi + 1 < body.len() && joined(&body[hi], &body[hi + 1]) {
            hi += 1;
        }
        entries.push((owner, TextRange::new(body[lo].start(), body[hi].end())));
        for stmt in &body[first..=index] {
            for (nested, _) in sub_bodies(stmt) {
                reform_entries(nested, stmt.range(), held, runs, entries);
            }
        }
        index += 1;
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use rstest::rstest;

    use super::*;
    use crate::{
        config::{Config, MaxShift},
        testing::parse,
    };

    /// The reservation table an `align-equals` run under `settings`
    /// reads back, built over a source carrying one assignment.
    fn columns_under(settings: Option<aligner::Settings>) -> Columns {
        Reservations::new(RuleId::from("align-equals"), settings, &Config::default())
            .columns(&parse("a = 1\n"))
    }

    /// Builds the default configuration capped at `line_length`.
    fn capped(line_length: usize) -> Config {
        Config {
            code_line_length: NonZeroUsize::new(line_length),
            ..Config::default()
        }
    }

    /// The column each value in `text` lands at under the default
    /// configuration capped at `line_length`, one entry per `values`
    /// offset.
    fn landed(text: &str, line_length: usize, values: &[u32]) -> Vec<usize> {
        landed_under(&capped(line_length), text, values)
    }

    /// Reads the column each value in `text` lands at under `config`, one
    /// entry per `values` offset.
    fn landed_under(config: &Config, text: &str, values: &[u32]) -> Vec<usize> {
        let source = parse(text);
        let columns = config.equals_reservations().columns(&source);
        values
            .iter()
            .map(|&offset| columns.column_in(&source, TextSize::new(offset)))
            .collect()
    }

    #[test]
    fn columns_break_open_a_hand_wrapped_call_whose_joined_row_overflows_the_column() {
        // Rejoined, `fontlist`'s call reaches 75 columns where it stands
        // and 82 padded to `font_name_title`'s column, past a cap of 80,
        // so the call breaks open and its row joins that column.
        let text = "frame = Frame(parent)\nfont_name_title = Label(frame, justify=LEFT, \
                    text=\"Font Face :\")\nfontlist = Listbox(frame, height=15,\n                   \
                    takefocus=True, exportselection=FALSE)\n";
        assert_eq!(landed(text, 80, &[98]), vec![18]);
    }

    #[rstest]
    #[case::exploded("", true, 15)]
    #[case::kept_without_reflow_calls("", false, 4)]
    #[case::kept_under_a_skip("  # prose: skip[reflow-calls]", true, 4)]
    fn columns_break_open_a_last_row_value_into_the_run_column(
        #[case] trailing: &str,
        #[case] explodes: bool,
        #[case] expected: usize,
    ) {
        // `c`'s row crosses a cap of 80 padded to `bbbbbbbbbbbb`'s column,
        // and breaking its call open leaves one column rather than two,
        // wherever `reflow-calls` can explode the call.
        let mut config = capped(80);
        config.rules.reflow_calls.enabled = explodes;
        let text = format!(
            "a = 1\nbbbbbbbbbbbb = 2\nc = frobnicate(first_argument_value, \
             second_argument_value, third_argument_v){trailing}\n"
        );
        assert_eq!(landed_under(&config, &text, &[27]), vec![expected]);
    }

    #[rstest]
    #[case::within(12, 15)]
    #[case::past(8, 4)]
    fn columns_break_open_a_value_only_within_max_shift(
        #[case] max_shift: usize,
        #[case] expected: usize,
    ) {
        // Broken open, `c`'s call would join `bbbbbbbbbbbb`'s column
        // eleven columns past its name, which a cap of 8 refuses.
        let mut config = capped(80);
        config.rules.align_equals.max_shift =
            MaxShift::Cap(NonZeroUsize::new(max_shift).expect("the cap is non-zero"));
        let text = "aaaaaaaaaa = 1\nbbbbbbbbbbbb = 2\nc = frobnicate(first_argument_value, \
                    second_argument_value, third_argument_v)\n";
        assert_eq!(landed_under(&config, text, &[36]), vec![expected]);
    }

    #[test]
    fn columns_break_open_an_augmented_value_into_the_run_column() {
        // `c`'s `+=` right-aligns on `bbbbbbbbbbbb`'s `=`, which pushes
        // its row past a cap of 80, so the call breaks open.
        let text = "a = 1\nbbbbbbbbbbbb = 2\nc += frobnicate(first_argument_value, \
                    second_argument_value, third_argu)\n";
        assert_eq!(landed(text, 80, &[28]), vec![15]);
    }

    #[rstest]
    #[case::breaks_open(true, 48)]
    #[case::read_as_written(false, 42)]
    fn columns_break_open_the_last_row_of_a_colon_run(
        #[case] colons: bool,
        #[case] expected: usize,
    ) {
        // `input_trans` ends its colon run, so breaking its call open into
        // `keymap`'s `=` column splits no colon run and the call breaks
        // open. With `align-colons` off, the row fits where it stands.
        let mut config = capped(60);
        config.rules.align_colons.enabled = colons;
        let text = "@dataclass\nclass Reader:\n    keymap: tuple[tuple[str, str], ...] = ()\n    \
                    input_trans: input.KeymapTranslator = field(init=False)\n";
        let value = u32::try_from(text.find("field(init").expect("the row carries a call"))
            .expect("the text fits u32");
        assert_eq!(landed_under(&config, text, &[value]), vec![expected]);
    }

    #[test]
    fn columns_count_a_keyword_the_rule_widens_on_the_same_line() {
        // Aligning `x` to `longer` lands its line on 15 columns, inside
        // a cap of 16 until the stacked `k=1` keyword the rule buffers
        // to `k = 1` widens it past, so the run breaks and `x` stays put.
        // With `reflow-calls` off, nothing joins or breaks open the call.
        let text = "longer = 2\nx = f(k=1,\n      j=2)\n";
        let under = |line_length| {
            let mut config = capped(line_length);
            config.rules.reflow_calls.enabled = false;
            landed_under(&config, text, &[15])
        };
        assert_eq!(under(16), vec![4]);
        assert_eq!(under(18), vec![9]);
    }

    #[test]
    fn columns_end_a_run_at_a_row_breaking_inside_its_value() {
        // `declaration_match` crosses a cap of 40 at its own column, so its
        // `compile` call breaks open inside the `.match` access and ends the
        // run, leaving `close` to stand at its own column.
        let text = "declaration_match = compile(r\"[a-z][-_.a-z0-9]*\").match\n\
                    close = compile(r\"--\\s*>\\s*\")\n";
        assert_eq!(landed(text, 40, &[64]), vec![8]);
    }

    #[test]
    fn columns_extend_a_run_past_a_value_a_layout_rule_rejoins() {
        // `b`'s call rejoins onto one row, so its run reaches `cccc`
        // below it and `a`'s value follows `cccc`'s to column 7.
        let text = "a = 1\nb = frob(first,\n         second)\ncccc = 2\n";
        assert_eq!(landed(text, 88, &[4]), vec![7]);
    }

    #[rstest]
    #[case::held_by_its_colon_run(true, 43)]
    #[case::read_as_written(false, 42)]
    fn columns_hold_an_annotated_row_whose_colon_run_continues_below_it(
        #[case] colons: bool,
        #[case] expected: usize,
    ) {
        // Once `align-colons` pads its `:`, `input_trans`'s `=` sits five
        // columns short of `keymap`'s, and padding it to that column pushes
        // its row past a cap of 60. Breaking its call open would split the
        // colon run above `input_trans_stack`, so the call keeps its row and
        // the value lands past the `:` padding alone. With `align-colons` off,
        // no padding widens the row and it stays where it stands.
        let mut config = capped(60);
        config.rules.align_colons.enabled = colons;
        let text = "@dataclass\nclass Reader:\n    keymap: tuple[tuple[str, str], ...] = ()\n    \
                    input_trans: input.KeymapTranslator = field(init=False)\n    \
                    input_trans_stack: list[input.KeymapTranslator] = field(default_factory=list)\n";
        let value = u32::try_from(text.find("field(init").expect("the row carries a call"))
            .expect("the text fits u32");
        assert_eq!(landed_under(&config, text, &[value]), vec![expected]);
    }

    #[test]
    fn columns_leave_a_middle_row_on_its_row_where_breaking_moves_the_unpadded_row() {
        // Breaking `c`'s call open into `xxxxxxxxxxxx`'s column would
        // leave `dd` alone below it, just as `xxxxxxxxxxxx` stands alone
        // while the call keeps its row, so `c` and `dd` align as a pair.
        let text = "xxxxxxxxxxxx = 1\nc = frobnicate(first_argument_value, \
                    second_argument_value, third_argument_v)\ndd = 4\n";
        assert_eq!(landed(text, 80, &[21, 100]), vec![5, 5]);
    }

    #[test]
    fn columns_leave_a_row_breaking_inside_its_value_where_it_fits_alone() {
        // `keys_to_move` fits a cap of 62 at its own column but crosses it
        // padded to `adapters[prefix]`'s column, and its bracket sits inside
        // the comprehension, so no layout rule breaks it open to join that
        // column and it stands at its own.
        let text = "adapters[prefix] = adapter\n\
                    keys_to_move = [k for k in adapters if len(k) < len(prefix)]\n";
        assert_eq!(landed(text, 62, &[42]), vec![15]);
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

    #[rstest]
    #[case::read_in_sorted_order(true, 8)]
    #[case::read_as_written(false, 19)]
    fn columns_read_a_class_body_in_the_order_alphabetize_siblings_sorts_it(
        #[case] sorted: bool,
        #[case] expected: usize,
    ) {
        // Sorted, `c` sits between `a` and `stroke_width`, where breaking
        // its call open gains nothing. Read as written, `c` sits below
        // `stroke_width` and its call breaks open into that column.
        let mut config = capped(80);
        config.rules.alphabetize_siblings.enabled = sorted;
        let text = "class Palette:\n    stroke_width = 1\n    c = frobnicate(\
                    first_argument_value, second_argument_value, third_arg)\n    a = 2\n";
        assert_eq!(landed_under(&config, text, &[44]), vec![expected]);
    }

    #[rstest]
    #[case::read_in_band_order(true, 14)]
    #[case::read_as_written(false, 11)]
    fn columns_read_a_module_body_in_the_order_band_constants_seats_it(
        #[case] banded: bool,
        #[case] expected: usize,
    ) {
        // Banded, `CHECK_DELAY` heads the run and `_openers` padded to its
        // column crosses a cap of 40, so the dict breaks open into that
        // column. Read as written, `_openers` heads the run and keeps its
        // row.
        let mut config = capped(40);
        config.rules.band_constants.enabled = banded;
        let text = "_openers = {')': '(',']': '[','}': '{'}\nCHECK_DELAY = 100\n";
        assert_eq!(landed_under(&config, text, &[11]), vec![expected]);
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
    fn columns_start_a_run_below_a_broken_value() {
        // `c`'s call breaks open into `bbbbbbbbbbbb`'s column, which ends
        // its run, so `d` stands alone at its own column.
        let text = "a = 1\nbbbbbbbbbbbb = 2\nc = frobnicate(first_argument_value, \
                    second_argument_value, third_argument_v)\nd = 4\n";
        assert_eq!(landed(text, 80, &[27, 105]), vec![15, 4]);
    }

    #[test]
    fn forwarded_without_drops_the_runs_of_each_named_body() {
        let range = |start: u32, end: u32| TextRange::new(TextSize::new(start), TextSize::new(end));
        let scope = |span: TextRange, stmt: TextRange, body| Scope {
            body,
            seated: false,
            span,
            stmt,
        };
        let shift = |run, span: TextRange| Shift {
            columns: 1,
            run,
            span,
        };
        let widening = |line: u32| (TextSize::new(line), TextRange::default(), 1);
        let owner = range(0, 40);
        let forwarded = Forwarded {
            runs: vec![
                scope(range(2, 6), owner, true),
                scope(range(4, 6), owner, false),
                scope(range(20, 24), owner, true),
                scope(range(50, 54), range(48, 60), true),
            ],
            shifts: vec![
                shift(0, range(3, 5)),
                shift(1, range(5, 6)),
                shift(2, range(21, 23)),
                shift(3, range(51, 53)),
            ],
            widenings: vec![(0, widening(2)), (2, widening(20))],
        }
        .without(&[(owner, range(2, 10))]);

        // Only the statement run inside the named arm drops. The keyword
        // run sharing its owner, the run of the sibling arm under that
        // same owner, and another owner's run stay and take the slots the
        // drop frees.
        assert_eq!(
            forwarded.runs,
            vec![
                scope(range(4, 6), owner, false),
                scope(range(20, 24), owner, true),
                scope(range(50, 54), range(48, 60), true),
            ]
        );
        assert_eq!(
            forwarded.shifts,
            vec![
                shift(0, range(5, 6)),
                shift(1, range(21, 23)),
                shift(2, range(51, 53)),
            ]
        );
        assert_eq!(forwarded.widenings, vec![(1, widening(20))]);
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
        assert_eq!(
            columns_under(Some(aligner::Settings::aligned(MaxShift::default())))
                .keyword_value_column(indent, name),
            Some(expected),
        );
    }
}
