//! The collection-layout walker. Visits each literal and subscript
//! outside an f-string or t-string replacement field, decides between
//! the rejoin and the expansion, and emits the edit that fits the
//! budget. The one-line rendering the decision measures against lives
//! in the sibling `inline` module.

use std::borrow::Cow;

use itertools::Itertools;
use ruff_diagnostics::Edit;
use ruff_python_ast::{
    AnyNodeRef, DictItem, Expr, InterpolatedStringElement,
    visitor::{Visitor, walk_expr},
};
use ruff_text_size::{Ranged, TextRange};
use unicode_width::UnicodeWidthStr;

use super::{
    classify::{
        Segment, is_align_colons_gap, is_atomic, is_collapse_only, is_collapsible, is_multi_entry,
        pre_colon_padding, requires_expand, segments,
    },
    flow::flow_lines,
};
use crate::{
    primitives::{
        INDENT_STEP,
        edit::narrowed_replacement,
        fracture,
        layout::{is_column_shaped, is_layoutable, item_indent, placed_block},
        reserve,
    },
    rules::stack_adjacent_strings::concatenated_run,
    source::Source,
};

pub(super) struct Layouter<'a> {
    pub(super) code_line_length: usize,
    pub(super) edits: Vec<Edit>,
    pub(super) explode: bool,
    pub(super) keep_multiline_literals: bool,
    pub(super) max_atomics: usize,
    pub(super) newline: &'static str,
    pub(super) rejoin: fracture::Settings,
    pub(super) reservations: reserve::Columns,
    pub(super) source: &'a Source,
    pub(super) tripping_dicts: Vec<TextRange>,
    pub(super) wrap_dict_entries: bool,
}

impl<'a> Layouter<'a> {
    /// Builds the expanded form of `expr` as a string, recursively
    /// laying out any qualifying child collections.
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
        let mut out = String::new();
        out.push(open);
        out.push_str(self.newline);
        for segment in segments(&atomics) {
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
                            && item_indent + widths[idx] + usize::from(has_more)
                                > self.code_line_length;
                        let hung = dict_items
                            .filter(|_| row_overflows && self.wrap_dict_entries)
                            .and_then(|items| {
                                self.hang_dict_value(&items[idx], parent, item_indent)
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
    /// collections arrive already laid out. An item needing neither a
    /// rewrite nor a move borrows its source slice.
    fn gather_items(&self, expr: &Expr, indent: usize) -> GatheredItems<'a> {
        let parent = AnyNodeRef::from(expr);
        if let Expr::Dict(d) = expr {
            let (texts, widths, atomics, ranges): (Vec<_>, Vec<_>, Vec<_>, Vec<_>) = d
                .iter()
                .map(|item| {
                    let (text, width) = self.serialize_dict_item(item, parent, indent);
                    (text, width, false, item.range())
                })
                .multiunzip();
            return GatheredItems {
                atomics,
                close: '}',
                open: '{',
                ranges,
                texts,
                widths,
            };
        }
        let (open, close, elts) = match expr {
            Expr::List(l) => ('[', ']', &l.elts),
            Expr::Set(s) => ('{', '}', &s.elts),
            Expr::Tuple(t) => ('(', ')', &t.elts),
            _ => unreachable!("gather_items called on non-expandable expr"),
        };
        let (texts, widths, atomics, ranges): (Vec<_>, Vec<_>, Vec<_>, Vec<_>) = elts
            .iter()
            .map(|e| {
                let text = self.serialize_expr(e, parent, indent, indent);
                let width = text.width();
                (text, width, is_atomic(e), e.range())
            })
            .multiunzip();
        GatheredItems {
            atomics,
            close,
            open,
            ranges,
            texts,
            widths,
        }
    }

    /// Builds the hung two-line form of a `key: value` dict entry,
    /// breaking at `:` and emitting the value at `item_indent +
    /// INDENT_STEP`. The key routes through `repaired_key` the same way
    /// `serialize_dict_item` does, and its pre-colon padding carries
    /// through, the column belonging to `align_colons`. Returns `None`
    /// for a `**value` unpacking item and for an entry either side of
    /// whose `:` carries an implicitly concatenated string, which
    /// `stack-adjacent-strings` breaks in place.
    fn hang_dict_value(
        &self,
        item: &DictItem,
        parent: AnyNodeRef,
        item_indent: usize,
    ) -> Option<String> {
        let key = item.key.as_ref()?;
        if concatenated_run(key).is_some() || concatenated_run(&item.value).is_some() {
            return None;
        }
        let key_text = self.repaired_key(key, parent, item_indent);
        let padding = pre_colon_padding(self.key_value_gap(key, &item.value));
        let hang_column = item_indent + INDENT_STEP;
        let value_text = self.serialize_expr(&item.value, parent, hang_column, hang_column);
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

    /// The source text between a keyed dict entry's `key` and its
    /// `value`, the span carrying the `:` and the padding around it.
    fn key_value_gap(&self, key: &Expr, value: &Expr) -> &'a str {
        self.source.slice(TextRange::new(key.end(), value.start()))
    }

    /// `expr`'s paren-recovered source range placed at `indent`.
    fn placed_slice(&self, expr: &Expr, parent: AnyNodeRef, indent: usize) -> Cow<'a, str> {
        placed_block(
            self.source,
            self.range_with_parens(expr, parent),
            Some(expr),
            indent,
        )
    }

    /// The one-line form of a fractured `expr`, or `None` when it holds
    /// no break or overflows the budget once joined. The repair runs
    /// whatever `keep_multiline_literals` holds, covering a subscript, a
    /// comprehension, and a dict key, whose breaks fall outside the entry
    /// boundaries the expand path lays a literal out on.
    fn repaired(&self, expr: &Expr, column: usize) -> Option<String> {
        self.source
            .contains_line_break(expr.range())
            .then(|| self.joined_if_fits(expr, column))
            .flatten()
    }

    /// Serializes a dict key, rejoining one written across lines so its
    /// `:` sits beside it and falling through to `serialize_expr`
    /// otherwise.
    fn repaired_key(&self, key: &Expr, parent: AnyNodeRef, indent: usize) -> Cow<'a, str> {
        self.repaired(key, indent).map_or_else(
            || self.serialize_expr(key, parent, indent, indent),
            Cow::Owned,
        )
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
    fn replacement_for(&self, expr: &Expr, column: usize, indent: usize) -> Option<String> {
        let range = expr.range();
        if self.source.intersects_comment(range) {
            return None;
        }
        if is_collapse_only(expr) {
            return self.repaired(expr, column);
        }
        if !is_layoutable(expr) {
            return None;
        }
        let expandable = requires_expand(expr);
        let over_count = self.has_over_count_dict(expr);
        if self.source.contains_line_break(range) {
            let held = self.holds_its_column(expr);
            if !held
                && !over_count
                && let Some(inline) = self.joined_if_fits(expr, column)
            {
                return Some(inline);
            }
            return (self.explode && expandable).then(|| self.expand(expr, indent));
        }
        (self.explode
            && expandable
            && (over_count || column + self.source.slice(range).width() > self.code_line_length))
            .then(|| self.expand(expr, indent))
    }

    /// Serializes a dict item as `key: value` or `**value`, paired with
    /// its display width at the canonical `": "` separator. The key
    /// routes through `repaired_key` so one written across lines rejoins
    /// beside its `:`, and the value's fit column sits past the key text
    /// and `": "`. A borrowed key and value over an `align-colons`-padded
    /// gap return the source slice whole so the padding round-trips, the
    /// width counting the canonical `": "`.
    fn serialize_dict_item(
        &self,
        item: &DictItem,
        parent: AnyNodeRef,
        indent: usize,
    ) -> (Cow<'a, str>, usize) {
        let Some(key) = &item.key else {
            let value_text = self.serialize_expr(&item.value, parent, indent + 2, indent);
            let width = 2 + value_text.width();
            return (Cow::Owned(format!("**{value_text}")), width);
        };
        let key_text = self.repaired_key(key, parent, indent);
        let value_column = indent + key_text.width() + 2;
        let value_text = self.serialize_expr(&item.value, parent, value_column, indent);
        let width = key_text.width() + 2 + value_text.width();
        let gap = self.key_value_gap(key, &item.value);
        // A rewritten key drops the source slice's alignment padding, so
        // the padded separator and the borrowed round-trip both hold only
        // while the key passes through unchanged.
        let padded = is_align_colons_gap(gap) && matches!(key_text, Cow::Borrowed(_));
        let text = if padded && matches!(value_text, Cow::Borrowed(_)) {
            Cow::Borrowed(self.source.slice(item))
        } else {
            let separator = if padded { gap } else { ": " };
            Cow::Owned(format!("{key_text}{separator}{value_text}"))
        };
        (text, width)
    }

    /// Serializes `expr` into a child slot of an enclosing expand.
    /// Dispatches through `replacement_for`, falling back to the
    /// paren-recovered source slice placed at `indent` when no rewrite
    /// applies. `column` and `indent` differ for dict values, where the
    /// key text sits between the line indent and the value's own start.
    fn serialize_expr(
        &self,
        expr: &Expr,
        parent: AnyNodeRef,
        column: usize,
        indent: usize,
    ) -> Cow<'a, str> {
        self.replacement_for(expr, column, indent)
            .map_or_else(|| self.placed_slice(expr, parent, indent), Cow::Owned)
    }

    /// True for a multi-entry literal the author laid out as a flush
    /// column while `keep_multiline_literals` holds it.
    pub(super) fn holds_its_column(&self, expr: &Expr) -> bool {
        self.keep_multiline_literals
            && is_multi_entry(expr)
            && is_column_shaped(self.source.slice(expr.range()))
    }
}

impl<'a> Visitor<'a> for Layouter<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if !is_collapsible(expr) {
            walk_expr(self, expr);
            return;
        }
        let range = expr.range();
        let start = range.start();
        // Test the collapse against the column `align_equals` shifts the
        // value to, not the unaligned column the literal currently opens
        // at, so a fit that survives the shift is what the rule collapses.
        let column = self.reservations.column_in(self.source, start);
        let indent = self.source.line_indent_width(start);
        let Some(text) = self.replacement_for(expr, column, indent) else {
            walk_expr(self, expr);
            return;
        };
        self.edits
            .extend(narrowed_replacement(self.source, range, text));
    }

    /// Leaves a replacement field unwalked.
    fn visit_interpolated_string_element(&mut self, _: &'a InterpolatedStringElement) {}
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
