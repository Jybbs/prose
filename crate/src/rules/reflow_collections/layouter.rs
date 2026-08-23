//! The `reflow-collections` walker. Visits each literal and subscript
//! outside an f-string or t-string replacement field, decides between
//! the rejoin and the expansion, and emits the edit that fits the
//! budget. The one-row rendering the decision measures against comes
//! from `primitives::one_row`.

use std::borrow::Cow;

use itertools::Itertools;
use ruff_diagnostics::Edit;
use ruff_python_ast::{AnyNodeRef, DictItem, Expr};
use ruff_text_size::{Ranged, TextRange, TextSize};
use unicode_width::UnicodeWidthStr;

use super::{
    classify::{Segment, is_align_colons_gap, is_atomic, pre_colon_padding, segments},
    flow::flow_lines,
};
use crate::{
    primitives::{
        INDENT_STEP,
        call_keywords::CallTargets,
        edit::narrowed_replacement,
        inline::end_column,
        layout::{
            is_collapse_only, is_collapsible, is_layoutable, is_multi_entry, item_indent,
            requires_expand,
        },
        one_row, padding, reserve,
        travel::{Landing, placed_block},
        walk::{Descent, ParentedProbe},
    },
    rules::{reflow_calls::Reshaper, stack_adjacent_strings::concatenated_run},
    source::Source,
};

pub(super) struct Layouter<'a> {
    pub(super) code_line_length: usize,
    pub(super) edits: Vec<Edit>,
    pub(super) explode: bool,
    pub(super) max_atomics: usize,
    pub(super) newline: &'static str,
    pub(super) one_row: one_row::Settings<'a>,
    pub(super) padding: &'a [Edit],
    pub(super) reservations: &'a reserve::Columns,
    pub(super) source: &'a Source,
    pub(super) targets: &'a CallTargets<'a>,
    pub(super) tripping_dicts: Vec<TextRange>,
    pub(super) wrap_dict_entries: bool,
}

impl<'a> Layouter<'a> {
    /// Builds the expanded form of `expr` as a string, recursively
    /// laying out any qualifying child collections. Every row is charged
    /// the separator closing it whatever its position, so the layout
    /// holds under the order `alphabetize-siblings` later imposes.
    fn expand(&self, expr: &Expr, indent: usize) -> String {
        let item_indent = item_indent(indent);
        let dict_items = expr.as_dict_expr().map(|d| &d.items);
        let parent = AnyNodeRef::from(expr);
        let GatheredItems {
            atomics,
            close,
            open,
            ranges,
            texts,
            widths,
        } = self.gather_items(expr, item_indent);
        let total = texts.len();
        let item_prefix = " ".repeat(item_indent);
        let available = self.code_line_length.saturating_sub(item_indent);
        let separator = usize::from(total > 1);
        let mut out = String::new();
        out.push(open);
        out.push_str(self.newline);
        for segment in segments(&atomics, expr.is_set_expr()) {
            match segment {
                Segment::Flow(range) => {
                    let run_start = range.start;
                    for line_range in flow_lines(&widths[range], available, self.max_atomics) {
                        let line_start = run_start + line_range.start;
                        let line_end = run_start + line_range.end;
                        out.push_str(&item_prefix);
                        out.push_str(&texts[line_start..line_end].join(", "));
                        if line_end < total {
                            out.push(',');
                        }
                        out.push_str(self.newline);
                    }
                }
                Segment::OnePerLine(range) => {
                    for idx in range {
                        let has_more = idx + 1 < total;
                        let inline = &texts[idx];
                        let row_overflows = !inline.contains('\n')
                            && item_indent + widths[idx] + separator > self.code_line_length;
                        let hung = dict_items
                            .filter(|_| row_overflows && self.wrap_dict_entries)
                            .and_then(|items| {
                                self.hang_dict_value(&items[idx], parent, item_indent, separator)
                            });
                        out.push_str(&item_prefix);
                        out.push_str(hung.as_deref().unwrap_or(inline));
                        if has_more {
                            out.push(',');
                        }
                        out.push_str(self.newline);
                        if has_more && self.source.has_blank_line_before(ranges[idx + 1].start()) {
                            out.push_str(self.newline);
                        }
                    }
                }
            }
        }
        out.push_str(&item_prefix[..indent]);
        out.push(close);
        out
    }

    /// Collects the bracket pair and per-item text, atomicity, and source
    /// range for the collection at `expr`, each child serialized through
    /// `serialize_expr` / `serialize_dict_item` at `indent` so nested
    /// collections arrive already laid out, every one charged the
    /// separator closing its row. An item needing neither a rewrite nor
    /// a move borrows its source slice.
    fn gather_items(&self, expr: &Expr, indent: usize) -> GatheredItems<'a> {
        let parent = AnyNodeRef::from(expr);
        let separator = usize::from(is_multi_entry(expr.into()));
        let (open, close, elts) = match expr {
            Expr::Dict(d) => {
                return GatheredItems::of(
                    '{',
                    '}',
                    d.iter().map(|item| {
                        let (text, width) =
                            self.serialize_dict_item(item, parent, indent, separator);
                        (text, width, false, item.range())
                    }),
                );
            }
            Expr::List(l) => ('[', ']', &l.elts),
            Expr::Set(s) => ('{', '}', &s.elts),
            Expr::Tuple(t) => ('(', ')', &t.elts),
            _ => unreachable!("gather_items called on non-expandable expr"),
        };
        GatheredItems::of(
            open,
            close,
            elts.iter().map(|e| {
                let text = self.serialize_expr(e, parent, indent, indent, separator);
                let width = self.text_width(&text, self.range_with_parens(e, parent));
                (text, width, is_atomic(e), e.range())
            }),
        )
    }

    /// Builds the hung two-line form of a `key: value` dict entry,
    /// breaking at `:` and emitting the value at `item_indent +
    /// INDENT_STEP` with `tail` columns closing its row. The key routes
    /// through `repaired_key` the same way `serialize_dict_item` does,
    /// and its pre-colon padding carries through, the column belonging
    /// to `align_colons`. Returns `None` for a `**value` unpacking item
    /// and for an entry either side of whose `:` carries an implicitly
    /// concatenated string, which `stack-adjacent-strings` breaks in
    /// place.
    fn hang_dict_value(
        &self,
        item: &DictItem,
        parent: AnyNodeRef,
        item_indent: usize,
        tail: usize,
    ) -> Option<String> {
        let key = item.key.as_ref()?;
        if concatenated_run(key).is_some() || concatenated_run(&item.value).is_some() {
            return None;
        }
        let key_text = self.repaired_key(key, parent, item_indent);
        let value_start = self.range_with_parens(&item.value, parent).start();
        let padding = pre_colon_padding(self.key_value_gap(key.end(), value_start));
        let hang_column = item_indent + INDENT_STEP;
        let value_text = self.serialize_expr(&item.value, parent, hang_column, hang_column, tail);
        let hang_prefix = " ".repeat(hang_column);
        Some(format!(
            "{key_text}{padding}:{newline}{hang_prefix}{value_text}",
            newline = self.newline,
        ))
    }

    /// True when `expr` contains an over-cap `Dict` at any depth,
    /// including itself. A `Dict` inside a replacement field does not
    /// count.
    fn has_over_count_dict(&self, expr: &Expr) -> bool {
        let range = expr.range();
        self.tripping_dicts
            .iter()
            .any(|dict| range.contains_range(*dict))
    }

    /// The source text between a keyed dict entry's `key` and the
    /// `value_start` its parens are recovered against, the span carrying
    /// the `:` and the padding around it.
    fn key_value_gap(&self, key_end: TextSize, value_start: TextSize) -> &'a str {
        self.source.slice(TextRange::new(key_end, value_start))
    }

    /// `expr`'s paren-recovered source range placed per `landing`, the
    /// calls inside it reshaped where the move pushes one past the
    /// budget and the slice moved whole otherwise. `tail` is the columns
    /// the enclosing layout writes after the text on its last row.
    fn placed_slice(
        &self,
        expr: &Expr,
        parent: AnyNodeRef,
        landing: Landing,
        tail: usize,
    ) -> Cow<'a, str> {
        let range = self.range_with_parens(expr, parent);
        self.reshaper()
            .reshaped(expr, range, landing, tail)
            .map_or_else(|| placed_block(self.source, range, landing), Cow::Owned)
    }

    /// The one-line form of a fractured `expr`, or `None` when it holds
    /// no break or overflows the budget once joined. The repair runs
    /// whatever `keep_multiline_literals` holds, covering a subscript, a
    /// comprehension, and a dict key, whose breaks fall outside the entry
    /// boundaries the expand path lays a literal out on.
    fn repaired(&self, expr: &Expr, column: usize, tail: usize) -> Option<String> {
        self.source
            .contains_line_break(expr.range())
            .then(|| {
                self.one_row
                    .repaired(self.source, expr, expr.into(), column, tail)
            })
            .flatten()
            .map(Cow::into_owned)
    }

    /// Serializes a dict key, rejoining one written across lines so its
    /// `:` sits beside it and falling through to `serialize_expr`
    /// otherwise.
    fn repaired_key(&self, key: &Expr, parent: AnyNodeRef, indent: usize) -> Cow<'a, str> {
        self.repaired(key, indent, 0).map_or_else(
            || self.serialize_expr(key, parent, indent, indent, 0),
            Cow::Owned,
        )
    }

    /// The terms the calls inside a relocated expression reshape under.
    fn reshaper(&self) -> Reshaper<'a> {
        Reshaper {
            one_row: self.one_row,
            reservations: self.reservations,
            source: self.source,
            targets: self.targets,
        }
    }

    /// Returns the canonical rewrite for `expr`, or `None` to descend
    /// into its children. `indent` is where the closing bracket lands on
    /// expand. A multi-line subscript or comprehension that fits rejoins,
    /// while a multi-item `Dict`, `List`, `Set`, or parenthesized `Tuple`
    /// that overflows expands, as does a `Dict` over `max_dict_entries`
    /// and a literal already laid out as a flush column. A subscript and a
    /// comprehension only ever rejoin. The `explode` facet gates every
    /// expansion, and a set `keep_multiline_literals` suppresses the
    /// literal rejoin, a cleared `explode` returning `None`.
    fn replacement_for(
        &self,
        expr: &Expr,
        column: usize,
        indent: usize,
        tail: usize,
    ) -> Option<String> {
        let range = expr.range();
        if self.source.intersects_comment(range) {
            return None;
        }
        if is_collapse_only(expr) {
            return self.repaired(expr, column, tail);
        }
        if !is_layoutable(expr) {
            return None;
        }
        let expandable = requires_expand(expr);
        let over_count = self.has_over_count_dict(expr);
        if self.source.contains_line_break(range) {
            if let Some(inline) = self.joined_if_fits(expr, column, tail) {
                return Some(inline);
            }
            return (self.explode && expandable).then(|| self.expand(expr, indent));
        }
        (self.explode
            && expandable
            && (over_count || column + self.settled_width(range) + tail > self.code_line_length))
            .then(|| self.expand(expr, indent))
    }

    /// Serializes a dict item as `key: value` or `**value`, paired with
    /// its display width at the canonical `": "` separator. The key
    /// routes through `repaired_key` so one written across lines rejoins
    /// beside its `:`, and the value's fit column sits past the key
    /// text's last row and the separator that lands ahead of it. A
    /// borrowed key and value
    /// over an `align-colons`-padded gap return the source slice whole so
    /// the padding round-trips, the width counting the canonical `": "`.
    fn serialize_dict_item(
        &self,
        item: &DictItem,
        parent: AnyNodeRef,
        indent: usize,
        tail: usize,
    ) -> (Cow<'a, str>, usize) {
        let value_range = self.range_with_parens(&item.value, parent);
        let Some(key) = &item.key else {
            let value_text = self.serialize_expr(&item.value, parent, indent + 2, indent, tail);
            let width = 2 + self.text_width(&value_text, value_range);
            return (Cow::Owned(format!("**{value_text}")), width);
        };
        let key_text = self.repaired_key(key, parent, indent);
        let gap = self.key_value_gap(key.end(), value_range.start());
        // A rewritten key drops the source slice's alignment padding, so
        // the padded separator and the borrowed round-trip both hold only
        // while the key passes through unchanged.
        let padded = is_align_colons_gap(gap) && matches!(key_text, Cow::Borrowed(_));
        let separator = if padded { gap } else { ": " };
        let value_column = end_column(&key_text, indent) + separator.width();
        let landing = Landing {
            column: value_column,
            indent,
            item: key.start(),
        };
        let value_text = self
            .replacement_for(&item.value, value_column, indent, tail)
            .map_or_else(
                || self.placed_slice(&item.value, parent, landing, tail),
                Cow::Owned,
            );
        let width =
            self.text_width(&key_text, key.range()) + 2 + self.text_width(&value_text, value_range);
        let text = if padded && matches!(value_text, Cow::Borrowed(_)) {
            Cow::Borrowed(
                self.source
                    .slice(TextRange::new(key.start(), value_range.end())),
            )
        } else {
            Cow::Owned(format!("{key_text}{separator}{value_text}"))
        };
        (text, width)
    }

    /// Serializes `expr` into a child slot of an enclosing expand with
    /// `tail` columns closing its row. Dispatches through
    /// `replacement_for`, falling back to the paren-recovered source
    /// slice placed at `indent` when no rewrite applies. `column` and
    /// `indent` differ for dict values, where the key text sits between
    /// the line indent and the value's own start.
    fn serialize_expr(
        &self,
        expr: &Expr,
        parent: AnyNodeRef,
        column: usize,
        indent: usize,
        tail: usize,
    ) -> Cow<'a, str> {
        let landing = Landing {
            column,
            indent,
            item: expr.start(),
        };
        self.replacement_for(expr, column, indent, tail)
            .map_or_else(
                || self.placed_slice(expr, parent, landing, tail),
                Cow::Owned,
            )
    }

    /// The display width `range` settles to once the padding rule drops
    /// the delimiter padding and colon padding inside it.
    fn settled_width(&self, range: TextRange) -> usize {
        self.source
            .slice(range)
            .width()
            .saturating_add_signed(-padding::slack(self.source, self.padding, range))
    }

    /// The display width `text` settles to: the settled width of `range`
    /// where `text` is that source slice as written, and its own width
    /// for a rewrite, which carries no padding.
    fn text_width(&self, text: &str, range: TextRange) -> usize {
        if self.source.slice(range) == text {
            self.settled_width(range)
        } else {
            text.width()
        }
    }

    /// `expr`'s one-row form when it joins without a residual break and
    /// fits the budget from `column` across `tail` trailing columns,
    /// else `None`. A held column and a leaf reaching no single row each
    /// leave the enclosing construct to the expand path.
    fn joined_if_fits(&self, expr: &Expr, column: usize, tail: usize) -> Option<String> {
        self.one_row
            .fitted(self.source, expr, expr.into(), column, tail)
            .map(Cow::into_owned)
    }

    /// The display width of the text trailing `expr` on its own physical
    /// row, at least the separator closing an entry of a multi-entry
    /// `parent`. A construct the expand path relocates lands on a row of
    /// its own instead, so only the walk's own entry reads this.
    fn row_tail(&self, expr: &Expr, parent: AnyNodeRef) -> usize {
        self.source
            .row_tail_width(expr.range().end())
            .max(usize::from(is_multi_entry(parent)))
    }

    /// The range covering `expr` with explicit parens recovered against
    /// `parent`.
    fn range_with_parens(&self, expr: &Expr, parent: AnyNodeRef) -> TextRange {
        self.source.paren_aware_range(expr.into(), parent)
    }
}

impl<'a> ParentedProbe<'a> for Layouter<'a> {
    const INTERPOLATIONS: Descent = Descent::Over;

    /// Descends past any expression the rule does not lay out or leaves
    /// as written.
    fn probe(&mut self, expr: &'a Expr, parent: AnyNodeRef<'a>, _: &[AnyNodeRef<'a>]) -> Descent {
        if !is_collapsible(expr) {
            return Descent::Into;
        }
        let range = expr.range();
        let start = range.start();
        // Test the collapse against the column `align_equals` shifts the
        // value to, not the unaligned column the literal currently opens
        // at, so a fit that survives the shift is what the rule collapses.
        let column = self.reservations.column_in(self.source, start);
        let indent = self.source.line_indent_width(start);
        let tail = self.row_tail(expr, parent);
        let Some(text) = self.replacement_for(expr, column, indent, tail) else {
            return Descent::Into;
        };
        self.edits
            .extend(narrowed_replacement(self.source, range, text));
        Descent::Over
    }
}

/// Per-item state for a dict, list, set, or tuple literal: serialized
/// text, atomicity for layout dispatch, source range for blank-line
/// lookups, and display width at the canonical `": "` separator, so an
/// `align_colons`-padded gap does not inflate the measure.
struct GatheredItems<'src> {
    atomics: Vec<bool>,
    close: char,
    open: char,
    ranges: Vec<TextRange>,
    texts: Vec<Cow<'src, str>>,
    widths: Vec<usize>,
}

impl<'src> GatheredItems<'src> {
    /// The items between `open` and `close`, each read as its text,
    /// width, atomicity, and source range.
    fn of(
        open: char,
        close: char,
        items: impl Iterator<Item = (Cow<'src, str>, usize, bool, TextRange)>,
    ) -> Self {
        let (texts, widths, atomics, ranges) = items.multiunzip();
        Self {
            atomics,
            close,
            open,
            ranges,
            texts,
            widths,
        }
    }
}
