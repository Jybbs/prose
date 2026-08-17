//! Predicts the column an alignment rule shifts each assignment and
//! keyword value to, so a layout decision tests a construct against the
//! position it lands at after alignment rather than its current one, and
//! reads that prediction back per offset through [`Columns`]. No
//! column is reserved for a value inside an f-string or t-string
//! replacement field. A row whose value spans lines groups as if
//! single-line, since a collapsing construct becomes single-line before
//! the alignment runs, and a wider sibling then joins the run the
//! collapse closes.

use ruff_python_ast::{
    Expr, InterpolatedStringElement, Stmt,
    visitor::{Visitor, walk_body, walk_expr},
};
use ruff_text_size::{TextRange, TextSize};

use crate::{
    primitives::{aligner, equal_targets, walk},
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
pub(crate) struct Columns(Vec<(TextRange, isize)>);

impl Columns {
    /// The column `offset` lands at, `fallback` moved by the shift the
    /// alignment applies to the row `offset` sits on.
    pub(crate) fn column(&self, offset: TextSize, fallback: impl FnOnce() -> usize) -> usize {
        fallback().saturating_add_signed(self.shift(offset))
    }

    /// The columns the alignment moves `offset` by, zero where no
    /// reservation covers it. A reservation never spans a row, so the
    /// nearest one starting at or before `offset` is the only candidate.
    fn shift(&self, offset: TextSize) -> isize {
        let slot = self.0.partition_point(|(range, _)| range.start() <= offset);
        slot.checked_sub(1)
            .map(|i| self.0[i])
            .filter(|(range, _)| range.contains(offset))
            .map_or(0, |(_, shift)| shift)
    }

    /// The column `offset` lands at, falling back to the column its own
    /// source line puts it at.
    pub(crate) fn column_in(&self, source: &Source, offset: TextSize) -> usize {
        self.column(offset, || source.column_of(offset))
    }
}

/// The alignment a layout rule measures against, resolved from
/// configuration once and carried as a value. `settings` is `None`
/// where the alignment rule is off, leaving every column unreserved.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Reservations {
    rule: RuleId,
    settings: Option<aligner::Settings>,
}

impl Reservations {
    /// The reservation for `rule` running under `settings`.
    pub(crate) fn new(rule: RuleId, settings: Option<aligner::Settings>) -> Self {
        Self { rule, settings }
    }

    /// Maps each aligned value's start offset to the display column it
    /// lands at once the run is aligned. A value the run leaves where it
    /// sits maps to that same column, so a lookup is a no-op for a value
    /// the alignment does not move.
    pub(crate) fn columns(self, source: &Source) -> Columns {
        let Some(settings) = self.settings else {
            return Columns(Vec::new());
        };
        let mut visitor = ReserveVisitor {
            columns: Vec::new(),
            rule: self.rule,
            settings,
            source,
        };
        visitor.visit_body(&source.ast().body);
        visitor
            .columns
            .sort_unstable_by_key(|(range, _)| range.start());
        Columns(visitor.columns)
    }
}

struct ReserveVisitor<'a> {
    columns: Vec<(TextRange, isize)>,
    rule: RuleId,
    settings: aligner::Settings,
    source: &'a Source,
}

impl ReserveVisitor<'_> {
    /// Records the columns each member's aligned value shifts by, over
    /// the span from that value's start to the end of its physical row.
    /// The value follows the operator's column by the operator's final
    /// character and the one-space value gap. A member whose value opens
    /// on a later line records nothing.
    fn record(&mut self, groups: Vec<Vec<aligner::Member>>) {
        for group in groups {
            let columns = aligner::operator_columns(self.source, &group, self.settings);
            self.columns
                .extend(group.iter().zip(columns).filter_map(|(member, column)| {
                    let start = member.rewritten_value_gap(self.source)?.end();
                    let shift =
                        (column + 2).cast_signed() - self.source.column_of(start).cast_signed();
                    Some((self.source.row_tail(start), shift))
                }));
        }
    }
}

impl<'a> Visitor<'a> for ReserveVisitor<'a> {
    fn visit_body(&mut self, body: &'a [Stmt]) {
        let source = self.source;
        let rule = self.rule;
        let groups = aligner::adjacent_member_groups(source, body, false, |stmt| {
            equal_targets::assignment(source, stmt)
                .map_or(aligner::Slot::Break, |m| m.slot(source, rule))
        });
        self.record(groups);
        walk_body(self, body);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Call(call) = expr {
            self.record(equal_targets::keyword_groups(
                self.source,
                self.rule,
                call,
                false,
            ));
        }
        walk_expr(self, expr);
    }

    /// Leaves a replacement field unwalked.
    fn visit_interpolated_string_element(&mut self, _: &'a InterpolatedStringElement) {}

    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        walk::walk_stmt(self, stmt);
    }
}
