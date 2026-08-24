//! The measuring half of the `reflow-collections` walker: the column a
//! construct lands at, the width it settles to, and the columns
//! trailing it on its row.

use std::borrow::Cow;

use ruff_python_ast::{AnyNodeRef, Expr};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};
use unicode_width::UnicodeWidthStr;

use super::{Layouter, entry_tail};
use crate::primitives::{edit::apply_inline_edits, padding};

impl<'a> Layouter<'a> {
    /// True when `expr` contains an over-cap `Dict` at any depth,
    /// including itself. A `Dict` inside a replacement field does not
    /// count.
    pub(super) fn has_over_count_dict(&self, expr: &Expr) -> bool {
        let range = expr.range();
        self.tripping_dicts
            .iter()
            .any(|dict| range.contains_range(*dict))
    }

    /// The source text between a keyed dict entry's `key` and the
    /// `value_start` its parens are recovered against, the span carrying
    /// the `:` and the padding around it.
    pub(super) fn key_value_gap(&self, key_end: TextSize, value_start: TextSize) -> &'a str {
        self.source.slice(TextRange::new(key_end, value_start))
    }

    /// The narrower of the width `range` settles to as written and the
    /// width `expr`'s canonical rebuild carries.
    pub(super) fn narrowest_width(
        &self,
        expr: &Expr,
        parent: AnyNodeRef,
        range: TextRange,
    ) -> usize {
        self.one_row
            .narrowest_width(self.source, expr, parent, range, self.padding)
    }

    /// The text ahead of `offset` on its logical line, rendered with the
    /// edits this walk has emitted so far.
    pub(super) fn placed_head(&self, offset: TextSize) -> Cow<'a, str> {
        let start = self.source.logical_line_start(offset).start();
        apply_inline_edits(self.source, TextRange::new(start, offset), &self.edits)
    }

    /// The range covering `expr` with explicit parens recovered against
    /// `parent`.
    pub(super) fn range_with_parens(&self, expr: &Expr, parent: AnyNodeRef) -> TextRange {
        self.source.paren_aware_range(expr.into(), parent)
    }

    /// The display width of the text trailing `expr` on its own physical
    /// row once the padding rule drops the padding inside it, read as
    /// the separator a sort pending over `parent`, itself under
    /// `grandparent`, leaves closing the entry where that text is at
    /// most the comma the entry carries now, and at least that separator
    /// where more follows on the row or the sort leaves `parent` as laid
    /// out. A construct the expand path relocates lands on a row of its
    /// own instead, so only the walk's own entry reads this.
    pub(super) fn row_tail(
        &self,
        expr: &Expr,
        parent: AnyNodeRef,
        grandparent: AnyNodeRef,
    ) -> usize {
        let end = expr.range().end();
        let current = self
            .source
            .row_tail_width(end)
            .saturating_add_signed(-padding::slack(
                self.source,
                self.padding,
                self.source.row_tail(end),
            ));
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
        self.reservations
            .column_in(self.source, offset)
            .saturating_add_signed(-padding::slack(self.source, self.padding, row))
    }

    /// The display width `range` settles to once the padding rule drops
    /// the delimiter padding and colon padding inside it.
    pub(super) fn settled_width(&self, range: TextRange) -> usize {
        self.source
            .slice(range)
            .width()
            .saturating_add_signed(-padding::slack(self.source, self.padding, range))
    }

    /// The display width `text` settles to: the settled width of `range`
    /// where `text` is that source slice as written, and its own width
    /// for a rewrite, which carries no padding.
    pub(super) fn text_width(&self, text: &str, range: TextRange) -> usize {
        if self.source.slice(range) == text {
            self.settled_width(range)
        } else {
            text.width()
        }
    }
}
