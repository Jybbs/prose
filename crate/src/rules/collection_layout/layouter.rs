//! The collection-layout serializer. Walks each literal and subscript,
//! renders its inline and expanded forms, and emits the edit that fits
//! the budget.

use std::{borrow::Cow, collections::HashMap};

use itertools::Itertools;
use ruff_diagnostics::Edit;
use ruff_python_ast::{
    AnyNodeRef, Comprehension, DictItem, Expr, ExprDict,
    visitor::{Visitor, walk_expr},
};
use ruff_text_size::{Ranged, TextRange, TextSize};
use unicode_width::UnicodeWidthStr;

use super::classify::{
    Segment, is_align_colons_gap, is_atomic, is_collapse_only, is_collapsible, requires_expand,
    segments,
};
use super::flow::flow_lines;
use crate::{
    primitives::{
        INDENT_STEP, edit::narrowed_replacement, inline::single_line_form, layout::is_layoutable,
    },
    source::Source,
};

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

pub(super) struct Layouter<'a> {
    pub(super) code_line_length: usize,
    pub(super) collapse: bool,
    pub(super) edits: Vec<Edit>,
    pub(super) explode: bool,
    pub(super) max_atomics: usize,
    pub(super) newline: &'static str,
    pub(super) reservations: HashMap<TextSize, usize>,
    pub(super) source: &'a Source,
    pub(super) tripping_dicts: Vec<TextRange>,
    pub(super) wrap_dict_entries: bool,
}

impl<'a> Layouter<'a> {
    /// Builds the expanded form of `expr` as a string, recursively
    /// laying out any qualifying child collections.
    fn expand(&self, expr: &Expr, indent: usize) -> String {
        let item_indent = indent + INDENT_STEP;
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
            }
        }
        out.extend(std::iter::repeat_n(' ', indent));
        out.push(close);
        out
    }

    /// Collects the bracket pair and per-item text, atomicity, and source
    /// range for the collection at `expr`, each child serialized through
    /// `serialize_expr` / `serialize_dict_item` at `indent` so nested
    /// collections arrive already laid out. An item needing no rewrite
    /// borrows its source slice.
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
    /// INDENT_STEP`. Returns `None` for `**value` unpacking items.
    fn hang_dict_value(
        &self,
        item: &DictItem,
        parent: AnyNodeRef,
        item_indent: usize,
    ) -> Option<String> {
        let key_text = self.source.slice(item.key.as_ref()?);
        let hang_column = item_indent + INDENT_STEP;
        let value_text = self.serialize_expr(&item.value, parent, hang_column, hang_column);
        let hang_prefix = " ".repeat(hang_column);
        Some(format!(
            "{key_text}:{newline}{hang_prefix}{value_text}",
            newline = self.newline,
        ))
    }

    /// True when `expr` contains an over-cap `Dict` at any depth,
    /// including itself.
    fn has_over_count_dict(&self, expr: &Expr) -> bool {
        let range = expr.range();
        self.tripping_dicts
            .iter()
            .any(|dict| range.contains_range(*dict))
    }

    /// Builds the inline form of `expr`, recursively inlining any nested
    /// collection or subscript. Leaves pass through as their source slice,
    /// explicit parens recovered against `parent` so a precedence-bearing
    /// `(-a) ** 2` survives the collapse.
    fn inline_form(&self, expr: &Expr) -> String {
        let mut buf = String::new();
        self.write_inline(&mut buf, expr, AnyNodeRef::from(expr));
        buf
    }

    /// `expr`'s inline form when it joins without a residual line break
    /// and fits the budget from `column`, else `None`. A leaf the inline
    /// serializer cannot itself join keeps its break and falls to the
    /// expand path.
    fn joined_if_fits(&self, expr: &Expr, column: usize) -> Option<String> {
        let inline = self.inline_form(expr);
        (!inline.contains('\n') && column + inline.width() <= self.code_line_length)
            .then_some(inline)
    }

    /// Returns the canonical rewrite for `expr`, or `None` to descend
    /// into its children. `indent` is where the closing bracket lands on
    /// expand. A multi-line literal, subscript, or comprehension that fits
    /// joins inline, while a multi-item `Dict`, `List`, `Set`, or
    /// parenthesized `Tuple` that overflows expands, as does a `Dict` over
    /// `max_dict_entries`. A subscript and a comprehension only ever
    /// collapse. The `collapse` and `explode` facets gate the join and the
    /// expansion, a cleared facet returning `None`.
    fn replacement_for(&self, expr: &Expr, column: usize, indent: usize) -> Option<String> {
        let range = expr.range();
        if self.source.intersects_comment(range) {
            return None;
        }
        if is_collapse_only(expr) {
            if !self.collapse || !self.source.contains_line_break(range) {
                return None;
            }
            return self.joined_if_fits(expr, column);
        }
        if !is_layoutable(expr) {
            return None;
        }
        let expandable = requires_expand(expr);
        let over_count = self.has_over_count_dict(expr);
        if self.source.contains_line_break(range) {
            if !over_count && let Some(inline) = self.joined_if_fits(expr, column) {
                return self.collapse.then_some(inline);
            }
            return (self.explode && expandable).then(|| self.expand(expr, indent));
        }
        (self.explode
            && expandable
            && (over_count || column + self.source.slice(range).width() > self.code_line_length))
            .then(|| self.expand(expr, indent))
    }

    /// Serializes a dict item as `key: value` or `**value`, paired with
    /// its display width at the canonical `": "` separator. Key and value
    /// both route through `serialize_expr`, the value's fit column offset
    /// past the key text and `": "`. A borrowed key and value over an
    /// `align-colons`-padded gap return the source slice whole so the
    /// padding round-trips, the width counting the canonical `": "`.
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
        let key_text = self.serialize_expr(key, parent, indent, indent);
        let value_column = indent + key_text.width() + 2;
        let value_text = self.serialize_expr(&item.value, parent, value_column, indent);
        let width = key_text.width() + 2 + value_text.width();
        let gap = self
            .source
            .slice(TextRange::new(key.end(), item.value.start()));
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
    /// Dispatches through `replacement_for`, falling back to a
    /// paren-recovered source slice when no rewrite applies.
    /// `column` and `indent` differ for dict values, where the key
    /// text sits between the line indent and the value's own start.
    fn serialize_expr(
        &self,
        expr: &Expr,
        parent: AnyNodeRef,
        column: usize,
        indent: usize,
    ) -> Cow<'a, str> {
        match self.replacement_for(expr, column, indent) {
            Some(text) => Cow::Owned(text),
            None => Cow::Borrowed(self.slice_with_parens(expr, parent)),
        }
    }

    /// Returns the source slice covering `expr`, with explicit parens
    /// recovered against `parent` so precedence-bearing parens like
    /// `(-a) ** 2` survive a borrow.
    fn slice_with_parens(&self, expr: &Expr, parent: AnyNodeRef) -> &'a str {
        let range = self.source.paren_aware_range(expr.into(), parent);
        self.source.slice(range)
    }

    /// Appends a comprehension's `for`/`if` clause chain to `buf`, each
    /// clause a space from the preceding text and an async generator
    /// carrying its `async` keyword. Targets, iterables, and conditions
    /// route through `write_inline`.
    fn write_comprehension_clauses(
        &self,
        buf: &mut String,
        generators: &[Comprehension],
        parent: AnyNodeRef,
    ) {
        for generator in generators {
            buf.push_str(if generator.is_async {
                " async for "
            } else {
                " for "
            });
            self.write_inline(buf, &generator.target, parent);
            buf.push_str(" in ");
            self.write_inline(buf, &generator.iter, parent);
            for condition in &generator.ifs {
                buf.push_str(" if ");
                self.write_inline(buf, condition, parent);
            }
        }
    }

    /// Appends the inline serialization of `expr` to `buf`. Recursive
    /// helper backing `inline_form`. `parent` is the immediate
    /// enclosing AST node, used for `paren_aware_range` recovery on
    /// non-collection leaves.
    fn write_inline(&self, buf: &mut String, expr: &Expr, parent: AnyNodeRef) {
        let here = AnyNodeRef::from(expr);
        match expr {
            Expr::Dict(d) => self.write_inline_dict(buf, d, here),
            Expr::DictComp(c) => {
                let brackets = Some(('{', '}'));
                self.write_inline_comprehension(
                    buf,
                    brackets,
                    c.key.as_deref(),
                    &c.value,
                    &c.generators,
                    here,
                );
            }
            Expr::Generator(c) => {
                let brackets = c.parenthesized.then_some(('(', ')'));
                self.write_inline_comprehension(buf, brackets, None, &c.elt, &c.generators, here);
            }
            Expr::List(l) => self.write_inline_seq(buf, Some(('[', ']')), &l.elts, here, false),
            Expr::ListComp(c) => {
                let brackets = Some(('[', ']'));
                self.write_inline_comprehension(buf, brackets, None, &c.elt, &c.generators, here);
            }
            Expr::Set(s) => self.write_inline_seq(buf, Some(('{', '}')), &s.elts, here, false),
            Expr::SetComp(c) => {
                let brackets = Some(('{', '}'));
                self.write_inline_comprehension(buf, brackets, None, &c.elt, &c.generators, here);
            }
            Expr::Subscript(s) => {
                self.write_inline(buf, &s.value, here);
                buf.push('[');
                self.write_inline(buf, &s.slice, here);
                buf.push(']');
            }
            Expr::Tuple(t) => {
                let brackets = t.parenthesized.then_some(('(', ')'));
                self.write_inline_seq(buf, brackets, &t.elts, here, t.len() == 1);
            }
            _ => {
                let slice = self.slice_with_parens(expr, parent);
                // An operator tree over atoms soft-wrapped across lines
                // rejoins by collapsing its break, where any other leaf
                // passes through with its source breaks intact for the
                // fit guard to reject.
                buf.push_str(&single_line_form(expr, slice).unwrap_or(Cow::Borrowed(slice)));
            }
        }
    }

    /// Appends a comprehension's bracketed inline form to `buf`: an
    /// optional `key: ` head, the element, then the clause chain, wrapped
    /// in `brackets`. A `None` bracket carries the bare generator whose
    /// call parens stand in, a `Some` key the dict comprehension's head.
    fn write_inline_comprehension(
        &self,
        buf: &mut String,
        brackets: Option<(char, char)>,
        key: Option<&Expr>,
        element: &Expr,
        generators: &[Comprehension],
        parent: AnyNodeRef,
    ) {
        let (open, close) = brackets.unzip();
        buf.extend(open);
        if let Some(key) = key {
            self.write_inline(buf, key, parent);
            buf.push_str(": ");
        }
        self.write_inline(buf, element, parent);
        self.write_comprehension_clauses(buf, generators, parent);
        buf.extend(close);
    }

    /// Writes `d`'s inline serialization into `buf` as `{k: v, ...}`,
    /// emitting `**v` for `None`-keyed unpacking items. `parent` is
    /// the dict itself, threaded into each child's `write_inline` for
    /// paren recovery on non-collection leaves.
    fn write_inline_dict(&self, buf: &mut String, d: &ExprDict, parent: AnyNodeRef) {
        buf.push('{');
        for (i, item) in d.iter().enumerate() {
            if i > 0 {
                buf.push_str(", ");
            }
            match &item.key {
                Some(key) => {
                    self.write_inline(buf, key, parent);
                    buf.push_str(": ");
                }
                None => buf.push_str("**"),
            }
            self.write_inline(buf, &item.value, parent);
        }
        buf.push('}');
    }

    /// Writes `elts` joined by `", "` into `buf`, optionally wrapped
    /// in a bracket pair and optionally followed by a trailing comma.
    /// The trailing comma carries the 1-tuple `(x,)` case.
    fn write_inline_seq(
        &self,
        buf: &mut String,
        brackets: Option<(char, char)>,
        elts: &[Expr],
        parent: AnyNodeRef,
        trailing_comma: bool,
    ) {
        let (open, close) = brackets.unzip();
        buf.extend(open);
        for (i, e) in elts.iter().enumerate() {
            if i > 0 {
                buf.push_str(", ");
            }
            self.write_inline(buf, e, parent);
        }
        if trailing_comma {
            buf.push(',');
        }
        buf.extend(close);
    }
}

impl<'a> Visitor<'a> for Layouter<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if !is_collapsible(expr) {
            walk_expr(self, expr);
            return;
        }
        let range = expr.range();
        // Test the collapse against the column `align_equals` shifts the
        // value to, not the unaligned column the literal currently opens
        // at, so a fit that survives the shift is what the rule collapses.
        let column = self
            .reservations
            .get(&range.start())
            .copied()
            .unwrap_or_else(|| self.source.column_of(range.start()));
        let indent = self.source.line_indent_width(range.start());
        match self.replacement_for(expr, column, indent) {
            Some(text) => self
                .edits
                .extend(narrowed_replacement(self.source, range, text)),
            None => walk_expr(self, expr),
        }
    }
}
