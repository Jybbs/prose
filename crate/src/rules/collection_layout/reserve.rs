//! Predicts the column `align_equals` shifts each assignment and keyword
//! value to, so the collapse decision tests a literal against the
//! position it lands at after alignment rather than its current one. A
//! row whose value spans lines groups as if single-line, since a
//! collapsing collection becomes single-line before `align_equals` runs,
//! and a wider sibling then joins the run the collapse closes.

use std::collections::HashMap;

use ruff_python_ast::{
    Expr, Stmt,
    visitor::{Visitor, walk_body, walk_expr},
};
use ruff_text_size::TextSize;

use crate::{
    primitives::{
        aligner,
        equal_targets::{self, EqualMember},
    },
    rules::align_equals::AlignEquals,
    source::Source,
};

struct ReserveVisitor<'a> {
    columns: HashMap<TextSize, usize>,
    settings: aligner::Settings,
    source: &'a Source,
}

impl ReserveVisitor<'_> {
    /// Records each member's aligned value column. The value follows the
    /// operator's column by the operator's final character and the
    /// one-space value gap.
    fn record(&mut self, groups: Vec<Vec<EqualMember>>) {
        for group in groups {
            let members: Vec<aligner::Member> = group.iter().map(|m| m.member).collect();
            let columns = aligner::operator_columns(self.source, &members, self.settings);
            for (member, column) in group.iter().zip(columns) {
                self.columns.insert(member.value_start(), column + 2);
            }
        }
    }
}

impl<'a> Visitor<'a> for ReserveVisitor<'a> {
    fn visit_body(&mut self, body: &'a [Stmt]) {
        let source = self.source;
        let groups = aligner::adjacent_member_groups(source, body, false, |stmt| {
            equal_targets::assignment(source, stmt)
                .map_or(aligner::Slot::Break, |m| m.slot(source, AlignEquals::SLUG))
        });
        self.record(groups);
        walk_body(self, body);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Call(call) = expr {
            self.record(equal_targets::keyword_groups(
                self.source,
                AlignEquals::SLUG,
                call,
                false,
            ));
        }
        walk_expr(self, expr);
    }
}

/// Maps each `align_equals`-aligned value's start offset to the display
/// column it lands at once the run is aligned. A value the run leaves at
/// its current column maps to that same column, so a lookup is a no-op
/// for a value `align_equals` does not move.
pub(super) fn reserved_columns(
    source: &Source,
    settings: aligner::Settings,
) -> HashMap<TextSize, usize> {
    let mut visitor = ReserveVisitor {
        columns: HashMap::new(),
        settings,
        source,
    };
    visitor.visit_body(&source.ast().body);
    visitor.columns
}
