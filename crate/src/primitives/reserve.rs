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

use std::collections::HashMap;

use ruff_python_ast::{
    AnyNodeRef, Expr, InterpolatedStringElement, Stmt,
    visitor::{Visitor, walk_body, walk_expr},
};
use ruff_text_size::{Ranged, TextRange, TextSize};
use unicode_width::UnicodeWidthStr;

use crate::{
    primitives::{aligner, call_keywords::module_call_params, equal_targets, one_row, walk},
    rule::RuleId,
    source::Source,
};

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
    shifts: Vec<(TextRange, isize)>,
}

impl Columns {
    /// The columns the alignment moves `offset` by, zero where no
    /// reservation covers it. A reservation never spans a row, so the
    /// nearest one starting at or before `offset` is the only candidate.
    fn shift(&self, offset: TextSize) -> isize {
        let slot = self
            .shifts
            .partition_point(|(range, _)| range.start() <= offset);
        slot.checked_sub(1)
            .map(|i| self.shifts[i])
            .filter(|(range, _)| range.contains(offset))
            .map_or(0, |(_, shift)| shift)
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

    /// The widening the reserved rule seats on each line, empty where
    /// that rule is off. A rule deciding a column ahead of the reserved
    /// one reads this so its line-cap check measures a row at the width
    /// the reserved rule leaves rather than the width the source
    /// carries.
    pub(crate) fn widenings(&self, source: &Source) -> aligner::Widenings {
        let Some(settings) = self.settings else {
            return aligner::Widenings::default();
        };
        let visitor = self.collected(source);
        aligner::Widenings::of(
            source,
            settings,
            visitor
                .runs
                .iter()
                .flat_map(|run| run.members.iter().copied()),
        )
    }

    /// Walks `source` collecting the runs the reserved rule builds.
    fn collected<'a>(&self, source: &'a Source) -> ReserveVisitor<'a> {
        let mut visitor = ReserveVisitor {
            rule: self.rule,
            runs: Vec::new(),
            source,
            values: HashMap::new(),
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
            return Columns {
                buffer: None,
                shifts: Vec::new(),
            };
        };
        let visitor = self.collected(source);
        let targets = module_call_params(source);
        let one_row = self.one_row.against(&targets);
        let widenings = aligner::Widenings::of(
            source,
            settings,
            visitor
                .runs
                .iter()
                .flat_map(|run| run.members.iter().copied()),
        );
        let joined = |member: aligner::Member| {
            let start = member.rewritten_value_gap(source)?.end();
            let &(expr, parent) = visitor.values.get(&start)?;
            let column = source.column_of(start);
            let end = source.paren_aware_range(expr.into(), parent).end();
            let tail = source.row_tail_width(end);
            let form = one_row.rejoined(source, expr, parent, column, tail)?;
            Some(column + form.width() + tail)
        };
        let mut shifts = Vec::new();
        for run in &visitor.runs {
            if run.candidate && !aligner::is_alignment_candidate(&run.members) {
                continue;
            }
            let columns =
                aligner::operator_columns(source, &run.members, settings, &widenings, joined);
            shifts.extend(
                run.members
                    .iter()
                    .zip(columns)
                    .filter_map(|(member, column)| {
                        let start = member.rewritten_value_gap(source)?.end();
                        let shift = (column + aligner::VALUE_OFFSET).cast_signed()
                            - source.column_of(start).cast_signed();
                        Some((source.row_tail(start), shift))
                    }),
            );
        }
        shifts.sort_unstable_by_key(|(range, _)| range.start());
        Columns {
            buffer: Some(settings.buffer()),
            shifts,
        }
    }
}

/// One collected alignment run, `candidate` where the rule aligns it
/// to a column or leaves it alone rather than buffering each row.
struct Run {
    candidate: bool,
    members: Vec<aligner::Member>,
}

/// Collects the rule's runs and the value each member's row carries,
/// keyed by the paren-aware start the member's value gap ends at.
struct ReserveVisitor<'a> {
    rule: RuleId,
    runs: Vec<Run>,
    source: &'a Source,
    values: HashMap<TextSize, (&'a Expr, AnyNodeRef<'a>)>,
}

impl<'a> ReserveVisitor<'a> {
    /// Records `value` under the start of its paren-aware range against
    /// `parent`.
    fn note(&mut self, value: &'a Expr, parent: AnyNodeRef<'a>) {
        let start = self.source.paren_aware_range(value.into(), parent).start();
        self.values.insert(start, (value, parent));
    }

    fn record(&mut self, groups: Vec<Vec<aligner::Member>>, candidate: bool) {
        self.runs
            .extend(groups.into_iter().map(|members| Run { candidate, members }));
    }
}

impl<'a> Visitor<'a> for ReserveVisitor<'a> {
    /// Builds the statement runs the way `align_equals` builds them, so
    /// a multi-line statement closes its run and a held one is
    /// transparent.
    fn visit_body(&mut self, body: &'a [Stmt]) {
        let source = self.source;
        self.record(
            aligner::line_adjacent_groups(source, body, self.rule, |stmt| {
                equal_targets::assignment(source, stmt)
            }),
            false,
        );
        for stmt in body {
            match stmt {
                Stmt::Assign(a) => self.note(&a.value, stmt.into()),
                Stmt::AugAssign(a) => self.note(&a.value, stmt.into()),
                Stmt::AnnAssign(a) => {
                    if let Some(value) = a.value.as_deref() {
                        self.note(value, stmt.into());
                    }
                }
                _ => {}
            }
        }
        walk_body(self, body);
    }

    /// Builds the keyword runs the way `align_equals` builds them, a
    /// multi-line value closing the run after it.
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Call(call) = expr {
            self.record(
                equal_targets::keyword_groups(self.source, self.rule, call, true),
                false,
            );
            for keyword in &call.arguments.keywords {
                self.note(&keyword.value, keyword.into());
            }
        }
        walk_expr(self, expr);
    }

    /// Leaves a replacement field unwalked.
    fn visit_interpolated_string_element(&mut self, _: &'a InterpolatedStringElement) {}

    /// Builds the parameter-default runs the way `align_equals` builds
    /// them, a run aligning to a column or not at all and a multi-line
    /// default closing the run after it.
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::FunctionDef(def) = stmt {
            let source = self.source;
            let groups = aligner::adjacent_member_groups(
                source,
                def.parameters.iter_source_order(),
                true,
                |param| equal_targets::parameter(source, param).into(),
            );
            let rule = self.rule;
            self.record(
                groups
                    .into_iter()
                    .map(|group| aligner::retain_unheld(source, rule, group))
                    .collect(),
                true,
            );
            for param in def.parameters.iter_non_variadic_params() {
                if let Some(default) = param.default.as_deref() {
                    self.note(default, param.into());
                }
            }
        }
        walk::walk_stmt(self, stmt);
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
    fn columns_shift_each_value_to_the_run_column() {
        // `a`'s value follows `bbb`'s to column 6 while `bbb`'s stays.
        assert_eq!(landed("a = 1\nbbb = 2\n", 88, &[4, 12]), vec![6, 6]);
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
