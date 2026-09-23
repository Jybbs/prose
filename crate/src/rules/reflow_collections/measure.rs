//! The measuring half of the `reflow-collections` walker: the column a
//! construct lands at, the range its recovered parens cover, the gap
//! around a dict entry's `:`, and the columns trailing it on its row.

use ruff_python_ast::{AnyNodeRef, Expr};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use super::{Layouter, entry_tail};
use crate::primitives::inline::settled_width;

impl<'a> Layouter<'a> {
    /// The source text between a keyed dict entry's `key` and the
    /// `value_start` its parens are recovered against, the span carrying
    /// the `:` and the padding around it.
    pub(super) fn key_value_gap(&self, key_end: TextSize, value_start: TextSize) -> &'a str {
        self.source.slice(TextRange::new(key_end, value_start))
    }

    /// The range covering `expr` with explicit parens recovered against
    /// `parent`.
    pub(super) fn range_with_parens(&self, expr: &Expr, parent: AnyNodeRef) -> TextRange {
        self.source.paren_aware_range(expr.into(), parent)
    }

    /// The display width of the text trailing `expr` on its physical
    /// row once the padding rule drops the padding inside it. Where a
    /// sort is pending over `parent`, itself under `grandparent`, the
    /// separator that sort leaves closing the entry replaces a tail
    /// holding at most a bare comma unless the sort leaves `parent` as
    /// laid out, and otherwise sets the floor of the measure. A
    /// construct the expand path relocates lands on a row of its own,
    /// so only the walk's own entry reads this.
    pub(super) fn row_tail(
        &self,
        expr: &Expr,
        parent: AnyNodeRef,
        grandparent: AnyNodeRef,
    ) -> usize {
        let end = expr.end();
        let current = settled_width(
            self.source,
            self.padding,
            self.source.row_tail(end),
            self.source.row_tail_width(end),
        );
        let Some(last) = self.reorders.sorted_last(self.source, parent, grandparent) else {
            return current;
        };
        let forecast = entry_tail(Some(last), expr.range(), 0);
        let bare_comma = matches!(
            self.source.slice(self.source.row_tail(end)).trim(),
            "" | ","
        );
        if bare_comma && !self.reorders.holds_as_laid_out(self.source, parent) {
            forecast
        } else {
            current.max(forecast)
        }
    }

    /// The column `offset` settles to once `align_equals` shifts its row
    /// and the padding rule drops the padding ahead of it on that row.
    pub(super) fn settled_column(&self, offset: TextSize) -> usize {
        let row = TextRange::new(self.source.text().line_start(offset), offset);
        settled_width(
            self.source,
            self.padding,
            row,
            self.reservations.column_in(self.source, offset),
        )
    }
}
