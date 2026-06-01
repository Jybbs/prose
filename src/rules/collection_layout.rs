//! Lays out `dict`, `list`, `set`, and `tuple` literals against the
//! `Config::code_line_length` budget. Multi-line literals whose
//! assembled inline form fits collapse back to a single line.
//! Single-line literals whose inline form overflows expand to one
//! entry per line. A dict holding more entries than
//! `max_inline_dict_entries` expands whatever its width, taking any
//! enclosing collection with it. A dict entry whose `key: value`
//! width overflows at the item-indent column breaks at `:` and hangs
//! the value at `item_indent + INDENT_STEP`. Comprehensions and any
//! literal whose source range contains a comment are out of scope.

use std::{borrow::Cow, num::NonZeroUsize, ops::Range};

use itertools::Itertools;
use ruff_diagnostics::Edit;
use ruff_python_ast::{
    AnyNodeRef, DictItem, Expr, ExprDict,
    helpers::{any_over_body, is_dotted_name},
    visitor::{Visitor, walk_expr},
};
use ruff_text_size::{Ranged, TextRange};
use unicode_width::UnicodeWidthStr;

use crate::{
    config::Config,
    primitives::{
        INDENT_STEP,
        edit::{narrowed_replacement, singleton_groups},
        range::paren_aware_range,
    },
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct CollectionLayout {
    code_line_length: usize,
    max_atomics_per_line: usize,
    max_inline_dict_entries: Option<usize>,
}

impl CollectionLayout {
    pub(crate) fn from_config(config: &Config) -> Self {
        let rules = &config.rules.collection_layout;
        Self {
            code_line_length: config.code_width(),
            max_atomics_per_line: rules
                .max_atomics_per_line
                .map_or(usize::MAX, NonZeroUsize::get),
            max_inline_dict_entries: rules.max_inline_dict_entries.map(NonZeroUsize::get),
        }
    }
}

impl Rule for CollectionLayout {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let body = &source.ast().body;
        // Precomputed once so the per-node count check is a containment
        // scan rather than re-walking each subtree.
        let tripping_dicts = self.max_inline_dict_entries.map_or_else(Vec::new, |cap| {
            let mut ranges = Vec::new();
            any_over_body(body, |expr| {
                if expr.as_dict_expr().is_some_and(|dict| dict.len() > cap) {
                    ranges.push(expr.range());
                }
                false
            });
            ranges
        });
        let mut visitor = Layouter {
            code_line_length: self.code_line_length,
            edits: Vec::new(),
            max_atomics_per_line: self.max_atomics_per_line,
            newline: source.newline_str(),
            source,
            tripping_dicts,
        };
        visitor.visit_body(body);
        singleton_groups(visitor.edits)
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

/// Per-item state collected from a dict, list, or set literal:
/// serialized text, atomicity for layout dispatch, and source range
/// for blank-line-preservation lookups.
struct GatheredItems<'src> {
    atomics: Vec<bool>,
    close: char,
    open: char,
    ranges: Vec<TextRange>,
    texts: Vec<Cow<'src, str>>,
}

struct Layouter<'a> {
    code_line_length: usize,
    edits: Vec<Edit>,
    max_atomics_per_line: usize,
    newline: &'static str,
    source: &'a Source,
    tripping_dicts: Vec<TextRange>,
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
                            && item_indent + inline.width() + usize::from(has_more)
                                > self.code_line_length;
                        let hung = dict_items.filter(|_| row_overflows).and_then(|items| {
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
                    let widths: Vec<usize> = texts[range].iter().map(|c| c.width()).collect();
                    for line_range in flow_lines(&widths, available, self.max_atomics_per_line) {
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

    /// Collects the bracket pair, per-item serialized text, per-item
    /// atomicity, and per-item source range for the collection at
    /// `expr`. The text is produced via `serialize_expr` /
    /// `serialize_dict_item` at `indent`, so nested qualifying
    /// children are already recursively laid out in the returned
    /// strings. Items that need no rewrite pass through as
    /// `Cow::Borrowed` of their source slice.
    fn gather_items(&self, expr: &Expr, indent: usize) -> GatheredItems<'a> {
        let parent = AnyNodeRef::from(expr);
        if let Expr::Dict(d) = expr {
            let (texts, atomics, ranges): (Vec<_>, Vec<_>, Vec<_>) = d
                .iter()
                .map(|item| {
                    (
                        self.serialize_dict_item(item, parent, indent),
                        false,
                        item.range(),
                    )
                })
                .multiunzip();
            return GatheredItems {
                atomics,
                close: '}',
                open: '{',
                ranges,
                texts,
            };
        }
        let (open, close, elts) = match expr {
            Expr::List(l) => ('[', ']', &l.elts),
            Expr::Set(s) => ('{', '}', &s.elts),
            _ => unreachable!("gather_items called on non-expandable expr"),
        };
        let (texts, atomics, ranges): (Vec<_>, Vec<_>, Vec<_>) = elts
            .iter()
            .map(|e| {
                (
                    self.serialize_expr(e, parent, indent, indent),
                    is_atomic(e),
                    e.range(),
                )
            })
            .multiunzip();
        GatheredItems {
            atomics,
            close,
            open,
            ranges,
            texts,
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

    /// Builds the canonical inline form of `expr`, recursively
    /// inlining any nested collection literal. Non-collection leaves
    /// pass through as their source slice, with explicit parentheses
    /// recovered against the enclosing `parent` so precedence-bearing
    /// parens (`(-a) ** 2`) survive the collapse.
    fn inline_form(&self, expr: &Expr) -> String {
        let mut buf = String::new();
        self.write_inline(&mut buf, expr, AnyNodeRef::from(expr));
        buf
    }

    /// Returns the canonical rewrite for `expr` against the budget at
    /// `column`, or `None` when the visitor should descend into its
    /// children. `indent` is where the closing bracket lands if `expr`
    /// expands. Emits `Some(inline)` when a multi-line literal's
    /// inline form fits, `Some(expand)` when a multi-item `Dict`,
    /// `List`, or `Set`'s rendered width overflows, or when a `Dict`
    /// carries more than `max_inline_dict_entries` entries whatever
    /// its width.
    fn replacement_for(&self, expr: &Expr, column: usize, indent: usize) -> Option<String> {
        if !is_layoutable(expr) {
            return None;
        }
        let range = expr.range();
        if self.source.intersects_comment(range) {
            return None;
        }
        let expandable = requires_expand(expr);
        let over_count = self.has_over_count_dict(expr);
        if self.source.contains_line_break(range) {
            if !over_count {
                let inline = self.inline_form(expr);
                if column + inline.width() <= self.code_line_length {
                    return Some(inline);
                }
            }
            return expandable.then(|| self.expand(expr, indent));
        }
        (expandable
            && (over_count || column + self.source.slice(range).width() > self.code_line_length))
            .then(|| self.expand(expr, indent))
    }

    /// Serializes a dict item as `key: value` or `**value`.
    ///
    /// `indent` is the column where the item sits (the item-indent of
    /// the enclosing dict). The value's actual column for the
    /// `code-line-length` check is offset by the key text plus `": "`, so a
    /// long key that pushes its value past the budget correctly
    /// triggers a re-layout of the value. When the value does expand,
    /// its closing bracket still lands at `indent`. When the value
    /// passes through borrowed and the source carries an
    /// `align-colons`-shaped gap (`[ ]*: `), the item's source slice
    /// is returned borrowed so the alignment padding round-trips.
    fn serialize_dict_item(
        &self,
        item: &DictItem,
        parent: AnyNodeRef,
        indent: usize,
    ) -> Cow<'a, str> {
        if let Some(key) = &item.key {
            let key_text = self.source.slice(key);
            let value_column = indent + key_text.width() + 2;
            let value_text = self.serialize_expr(&item.value, parent, value_column, indent);
            let gap = self
                .source
                .slice(TextRange::new(key.end(), item.value.start()));
            let aligned = is_align_colons_gap(gap);
            if aligned && matches!(value_text, Cow::Borrowed(_)) {
                Cow::Borrowed(self.source.slice(item))
            } else {
                let separator = if aligned { gap } else { ": " };
                Cow::Owned(format!("{key_text}{separator}{value_text}"))
            }
        } else {
            let value_text = self.serialize_expr(&item.value, parent, indent + 2, indent);
            Cow::Owned(format!("**{value_text}"))
        }
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
        let range = paren_aware_range(expr.into(), parent, self.source.tokens());
        self.source.slice(range)
    }

    /// Appends the inline serialization of `expr` to `buf`. Recursive
    /// helper backing `inline_form`. `parent` is the immediate
    /// enclosing AST node, used for `paren_aware_range` recovery on
    /// non-collection leaves.
    fn write_inline(&self, buf: &mut String, expr: &Expr, parent: AnyNodeRef) {
        let here = AnyNodeRef::from(expr);
        match expr {
            Expr::Dict(d) => self.write_inline_dict(buf, d, here),
            Expr::List(l) => self.write_inline_seq(buf, Some(('[', ']')), &l.elts, here, false),
            Expr::Set(s) => self.write_inline_seq(buf, Some(('{', '}')), &s.elts, here, false),
            Expr::Tuple(t) => {
                let brackets = t.parenthesized.then_some(('(', ')'));
                self.write_inline_seq(buf, brackets, &t.elts, here, t.elts.len() == 1);
            }
            _ => buf.push_str(self.slice_with_parens(expr, parent)),
        }
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
        if !is_layoutable(expr) {
            walk_expr(self, expr);
            return;
        }
        let range = expr.range();
        let column = self.source.column_of(range.start());
        let indent = self.source.line_indent_width(range.start());
        match self.replacement_for(expr, column, indent) {
            Some(text) => self
                .edits
                .extend(narrowed_replacement(self.source, range, text)),
            None => walk_expr(self, expr),
        }
    }
}

/// Describes how a contiguous slice of items should lay out.
#[derive(Debug, PartialEq)]
enum Segment {
    /// Items in the range flow across as few balanced lines as fit.
    Flow(Range<usize>),
    /// Each item in the range goes on its own line.
    OnePerLine(Range<usize>),
}

/// Distributes items into the smallest number of lines such that no
/// line exceeds `available` characters and no line carries more than
/// `max_atomics` items, giving roughly equal item counts per line.
/// Escalates to more lines if the even split at the minimum line
/// count would still overflow either cap.
fn flow_lines(widths: &[usize], available: usize, max_atomics: usize) -> Vec<Range<usize>> {
    if widths.is_empty() {
        return Vec::new();
    }
    let n = widths.len();
    let prefix: Vec<usize> = std::iter::once(0)
        .chain(widths.iter().scan(0usize, |sum, &w| {
            *sum += w;
            Some(*sum)
        }))
        .collect();
    let total_slot = prefix[n] + 2 * n.saturating_sub(1);
    let by_width = total_slot.div_ceil(available.max(1));
    let by_cap = n.div_ceil(max_atomics.max(1));
    let initial = by_width.max(by_cap).max(1);
    (initial..=n)
        .find_map(|num_lines| try_even(&prefix, num_lines, available, max_atomics))
        .unwrap_or_else(|| (0..n).map(|i| i..(i + 1)).collect())
}

/// Returns `true` when `gap` is zero or more ASCII spaces, then
/// `:`, then one ASCII space.
fn is_align_colons_gap(gap: &str) -> bool {
    gap.strip_suffix(": ")
        .is_some_and(|prefix| prefix.bytes().all(|b| b == b' '))
}

/// True for expressions that render as a single compact token and
/// therefore do not benefit from a dedicated line. Covers literals,
/// dotted names, and unary operations over atomic operands. Starred
/// expressions are non-atomic so a spread splits surrounding atomics
/// into independent runs.
fn is_atomic(expr: &Expr) -> bool {
    std::iter::successors(Some(expr), |e| {
        e.as_unary_op_expr().map(|u| u.operand.as_ref())
    })
    .any(|e| e.is_literal_expr() || is_dotted_name(e))
}

/// True for the four collection-literal `Expr` variants the rule
/// considers laying out. `Tuple` joins `Dict`, `List`, and `Set` here
/// because it's collapse-eligible, even though it never expands.
fn is_layoutable(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Dict(_) | Expr::List(_) | Expr::Set(_) | Expr::Tuple(_)
    )
}

/// True for a `Dict`, `List`, or `Set` shape the expand path
/// canonicalizes. Multi-item `List` and `Set` qualify. Any
/// non-empty `Dict` qualifies. Tuples and empty collections
/// collapse only, never expand.
fn requires_expand(expr: &Expr) -> bool {
    match expr {
        Expr::Dict(d) => !d.is_empty(),
        Expr::List(l) => l.len() > 1,
        Expr::Set(s) => s.len() > 1,
        _ => false,
    }
}

/// Partitions `atomics` into segments. Every contiguous run of
/// atomic items becomes one `Flow` segment. Every non-atomic item
/// becomes a singleton `OnePerLine` segment. Non-atomic items always
/// break atomic runs.
fn segments(atomics: &[bool]) -> Vec<Segment> {
    atomics
        .chunk_by(|a, b| a == b)
        .scan(0, |start, chunk| {
            let range = *start..*start + chunk.len();
            *start += chunk.len();
            Some(if chunk[0] {
                Segment::Flow(range)
            } else {
                Segment::OnePerLine(range)
            })
        })
        .collect()
}

/// Attempts an even distribution of items across `num_lines` lines.
/// `prefix` is a length-`n+1` running sum of per-item widths so the
/// slot sum for any line `[start..end)` is one subtraction. Returns
/// `None` when any line would exceed `max_atomics` items or
/// `available` slot width. First `n % num_lines` lines carry one
/// extra item.
fn try_even(
    prefix: &[usize],
    num_lines: usize,
    available: usize,
    max_atomics: usize,
) -> Option<Vec<Range<usize>>> {
    let n = prefix.len() - 1;
    if num_lines == 0 || num_lines > n {
        return None;
    }
    let base = n / num_lines;
    let remainder = n % num_lines;
    let mut lines = Vec::with_capacity(num_lines);
    let mut start = 0;
    for k in 0..num_lines {
        let size = base + usize::from(k < remainder);
        if size > max_atomics {
            return None;
        }
        let end = start + size;
        let slot = prefix[end] - prefix[start] + 2 * size.saturating_sub(1);
        if slot > available {
            return None;
        }
        lines.push(start..end);
        start = end;
    }
    Some(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn align_colons_gap_accepts_canonical_and_padded_forms() {
        assert!(is_align_colons_gap(": "));
        assert!(is_align_colons_gap(" : "));
        assert!(is_align_colons_gap("    : "));
    }

    #[test]
    fn align_colons_gap_rejects_non_padding_shapes() {
        assert!(!is_align_colons_gap(":"));
        assert!(!is_align_colons_gap(":  "));
        assert!(!is_align_colons_gap(" :"));
        assert!(!is_align_colons_gap("\t: "));
        assert!(!is_align_colons_gap(""));
    }

    #[test]
    fn flow_lines_escalates_when_initial_split_overflows() {
        // Total width forces an initial guess of 2 lines, but the
        // only even 2-line split puts three 10-wide items on one line
        // and overflows. The find_map must escalate to 3 lines.
        let lines = flow_lines(&[10, 10, 10, 1, 1, 1], 23, 8);
        assert_eq!(lines, vec![0..2, 2..4, 4..6]);
    }

    #[test]
    fn flow_lines_falls_back_to_one_per_line_when_no_split_fits() {
        // The lone 100-wide item exceeds available=10, pushing initial
        // above n=1 so the find_map range is empty. The fallback
        // emits one item per line.
        let lines = flow_lines(&[100], 10, 8);
        assert_eq!(lines, vec![0..1]);
    }

    #[test]
    fn flow_lines_packs_into_one_line_when_budget_allows() {
        let lines = flow_lines(&[1, 1, 1, 1], 80, 8);
        assert_eq!(lines, vec![0..4]);
    }

    #[test]
    fn flow_lines_returns_empty_for_empty_widths() {
        assert!(flow_lines(&[], 80, 8).is_empty());
    }

    #[test]
    fn flow_lines_splits_when_available_width_forces_it() {
        let lines = flow_lines(&[10, 10, 10], 12, 8);
        assert_eq!(lines, vec![0..1, 1..2, 2..3]);
    }

    #[test]
    fn flow_lines_splits_when_max_atomics_forces_it() {
        let lines = flow_lines(&[1; 6], 80, 3);
        assert_eq!(lines, vec![0..3, 3..6]);
    }

    #[test]
    fn segments_partitions_alternating_atomic_runs() {
        let result = segments(&[true, true, false, true, false, false]);
        assert_eq!(
            result,
            vec![
                Segment::Flow(0..2),
                Segment::OnePerLine(2..3),
                Segment::Flow(3..4),
                Segment::OnePerLine(4..6),
            ],
        );
    }

    #[test]
    fn segments_returns_empty_for_empty_input() {
        assert!(segments(&[]).is_empty());
    }

    #[test]
    fn try_even_distributes_remainder_to_leading_lines() {
        let lines = try_even(&[0, 1, 2, 3, 4, 5], 2, 80, 8).expect("split fits");
        assert_eq!(lines, vec![0..3, 3..5]);
    }

    #[test]
    fn try_even_rejects_more_lines_than_items() {
        assert!(try_even(&[0, 1, 2], 3, 80, 8).is_none());
    }

    #[test]
    fn try_even_rejects_zero_lines() {
        assert!(try_even(&[0, 1, 2], 0, 80, 8).is_none());
    }

    #[test]
    fn try_even_returns_none_when_max_atomics_exceeded() {
        assert!(try_even(&[0, 1, 2, 3, 4, 5], 1, 80, 3).is_none());
    }

    #[test]
    fn try_even_returns_none_when_slot_overflows() {
        assert!(try_even(&[0, 50, 100], 1, 60, 8).is_none());
    }
}
