//! Predicts the column an alignment rule shifts each assignment and
//! keyword value to, so a layout decision tests a construct against the
//! position it lands at after alignment rather than its current one, and
//! reads that prediction back per offset through [`Columns`]. No
//! column is reserved for a value inside an f-string or t-string
//! replacement field. A row whose value spans lines groups as if
//! single-line, since a collapsing construct becomes single-line before
//! the alignment runs, and a wider sibling then joins the run the
//! collapse closes.

use std::collections::HashMap;

use ruff_python_ast::{
    Expr, InterpolatedStringElement, Stmt,
    visitor::{Visitor, walk_body, walk_expr},
};
use ruff_text_size::TextSize;

use crate::{
    primitives::{aligner, equal_targets},
    rule::RuleId,
    source::Source,
};

/// The column each aligned value lands at once the alignment settles,
/// keyed by the value's start offset.
pub(crate) struct Columns(HashMap<TextSize, usize>);

impl Columns {
    /// The column `offset` lands at, the column reserved for it where
    /// the alignment moves it and `fallback` otherwise.
    pub(crate) fn column(&self, offset: TextSize, fallback: impl FnOnce() -> usize) -> usize {
        self.0.get(&offset).copied().unwrap_or_else(fallback)
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
#[derive(Clone, Copy)]
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
            return Columns(HashMap::new());
        };
        let mut visitor = ReserveVisitor {
            columns: HashMap::new(),
            rule: self.rule,
            settings,
            source,
        };
        visitor.visit_body(&source.ast().body);
        Columns(visitor.columns)
    }
}

struct ReserveVisitor<'a> {
    columns: HashMap<TextSize, usize>,
    rule: RuleId,
    settings: aligner::Settings,
    source: &'a Source,
}

impl ReserveVisitor<'_> {
    /// Records each member's aligned value column. The value follows the
    /// operator's column by the operator's final character and the
    /// one-space value gap. A member whose value opens on a later line
    /// records nothing.
    fn record(&mut self, groups: Vec<Vec<aligner::Member>>) {
        for group in groups {
            let columns = aligner::operator_columns(self.source, &group, self.settings);
            self.columns
                .extend(group.iter().zip(columns).filter_map(|(member, column)| {
                    Some((member.rewritten_value_gap(self.source)?.end(), column + 2))
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
}
