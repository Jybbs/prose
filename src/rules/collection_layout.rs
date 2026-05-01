//! Expands multi-item `dict`, `list`, and `set` literals when the
//! inline form would overflow `Config::line_length`. Comprehensions,
//! tuple literals, and any literal whose source range contains a
//! comment are out of scope.

use std::borrow::Cow;
use std::num::NonZeroUsize;
use std::ops::Range;

use ruff_diagnostics::Edit;
use ruff_python_ast::helpers::is_dotted_name;
use ruff_python_ast::token::parenthesized_range;
use ruff_python_ast::visitor::{walk_expr, Visitor};
use ruff_python_ast::{AnyNodeRef, DictItem, Expr};
use ruff_text_size::{Ranged, TextRange};
use unicode_width::UnicodeWidthStr;

use crate::config::Config;
use crate::pipeline::Rule;
use crate::primitives::edit::narrowed_replacement;
use crate::source::Source;

const DEFAULT_LINE_LENGTH: usize = 88;
const DEFAULT_MAX_ATOMICS_PER_LINE: usize = 8;
const INDENT_STEP: usize = 4;

pub(crate) struct CollectionLayout {
    line_length: usize,
    max_atomics_per_line: usize,
}

impl CollectionLayout {
    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            line_length: config
                .line_length
                .map_or(DEFAULT_LINE_LENGTH, NonZeroUsize::get),
            max_atomics_per_line: config
                .rules
                .collection_layout
                .max_atomics_per_line
                .map_or(DEFAULT_MAX_ATOMICS_PER_LINE, NonZeroUsize::get),
        }
    }
}

impl Rule for CollectionLayout {
    fn apply(&self, source: &Source) -> Vec<Edit> {
        let mut visitor = Expander {
            edits: Vec::new(),
            line_length: self.line_length,
            max_atomics_per_line: self.max_atomics_per_line,
            newline: source.newline_str(),
            source,
        };
        visitor.visit_body(&source.ast().body);
        visitor.edits
    }

    fn name(&self) -> &'static str {
        "collection-layout"
    }
}

struct Expander<'a> {
    edits: Vec<Edit>,
    line_length: usize,
    max_atomics_per_line: usize,
    newline: &'static str,
    source: &'a Source,
}

impl<'a> Expander<'a> {
    /// Builds the expanded form of `expr` as a string, recursively
    /// expanding any qualifying child collections.
    fn expand(&self, expr: &Expr, indent: usize) -> String {
        let item_indent = indent + INDENT_STEP;
        let GatheredItems {
            atomics,
            close,
            open,
            ranges,
            texts,
        } = self.gather_items(expr, item_indent);
        let total = texts.len();
        let item_prefix = " ".repeat(item_indent);
        let available = self.line_length.saturating_sub(item_indent);
        let mut out = String::new();
        out.push(open);
        out.push_str(self.newline);
        for segment in segments(&atomics) {
            match segment {
                Segment::OnePerLine(range) => {
                    for idx in range {
                        out.push_str(&item_prefix);
                        out.push_str(&texts[idx]);
                        if idx + 1 < total {
                            out.push(',');
                        }
                        out.push_str(self.newline);
                        if idx + 1 < total
                            && self.source.has_blank_line_before(ranges[idx + 1].start())
                        {
                            out.push_str(self.newline);
                        }
                    }
                }
                Segment::Flow(range) => {
                    let run_start = range.start;
                    let widths: Vec<usize> = range.map(|i| texts[i].width()).collect();
                    for line_range in flow_lines(&widths, available, self.max_atomics_per_line) {
                        let line_start = run_start + line_range.start;
                        let line_end = run_start + line_range.end;
                        let parts: Vec<&str> =
                            (line_start..line_end).map(|i| texts[i].as_ref()).collect();
                        out.push_str(&item_prefix);
                        out.push_str(&parts.join(", "));
                        if line_end < total {
                            out.push(',');
                        }
                        out.push_str(self.newline);
                    }
                }
            }
        }
        out.push_str(&" ".repeat(indent));
        out.push(close);
        out
    }

    /// Collects the bracket pair, per-item serialized text, per-item
    /// atomicity, and per-item source range for the collection at
    /// `expr`. The text is produced via `serialize_expr` /
    /// `serialize_dict_item` at `indent`, so nested qualifying
    /// children are already recursively expanded in the returned
    /// strings. Items that need no expansion pass through as
    /// `Cow::Borrowed` of their source slice.
    fn gather_items(&self, expr: &Expr, indent: usize) -> GatheredItems<'a> {
        let parent = AnyNodeRef::from(expr);
        if let Expr::Dict(d) = expr {
            let (texts, ranges): (Vec<Cow<'a, str>>, Vec<TextRange>) = d
                .iter()
                .map(|item| (self.serialize_dict_item(item, parent, indent), item.range()))
                .unzip();
            return GatheredItems {
                atomics: vec![false; d.len()],
                close: '}',
                open: '{',
                ranges,
                texts,
            };
        }
        let (open, close, elts) = match expr {
            Expr::List(l) => ('[', ']', &l.elts),
            Expr::Set(s) => ('{', '}', &s.elts),
            _ => unreachable!("gather_items called on non-collection expr"),
        };
        let mut texts = Vec::with_capacity(elts.len());
        let mut atomics = Vec::with_capacity(elts.len());
        let mut ranges = Vec::with_capacity(elts.len());
        for e in elts {
            texts.push(self.serialize_expr(e, parent, indent, indent));
            atomics.push(is_atomic(e));
            ranges.push(e.range());
        }
        GatheredItems {
            atomics,
            close,
            open,
            ranges,
            texts,
        }
    }

    /// Serializes a dict item as `key: value` or `**value`.
    ///
    /// `indent` is the column where the item sits (the item-indent of
    /// the enclosing dict). The value's actual column for the
    /// line-length check is offset by the key text plus `": "`, so a
    /// long key that pushes its value past the budget correctly
    /// triggers expansion of the value. When the value does expand,
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
            } else if aligned {
                Cow::Owned(format!("{key_text}{gap}{value_text}"))
            } else {
                Cow::Owned(format!("{key_text}: {value_text}"))
            }
        } else {
            let value_text = self.serialize_expr(&item.value, parent, indent + 2, indent);
            Cow::Owned(format!("**{value_text}"))
        }
    }

    /// Serializes `expr` into the expanded output. Returns
    /// `Cow::Borrowed` of the source slice (with explicit parentheses
    /// recovered) when `expr` needs no expansion, `Cow::Owned` when
    /// `expr` itself expands.
    ///
    /// `column` is where the expression actually begins on its line
    /// (used for the line-length overflow check). `indent` is where
    /// its closing bracket should land if it expands. They differ for
    /// dict values, where the key text sits between the line indent
    /// and the value's own starting column. `parent` is the immediate
    /// enclosing collection, used to recover any explicit parentheses
    /// around `expr` that `expr.range()` would otherwise drop.
    fn serialize_expr(
        &self,
        expr: &Expr,
        parent: AnyNodeRef,
        column: usize,
        indent: usize,
    ) -> Cow<'a, str> {
        if self.should_expand(expr, column) {
            return Cow::Owned(self.expand(expr, indent));
        }
        let range = parenthesized_range(expr.into(), parent, self.source.tokens())
            .unwrap_or_else(|| expr.range());
        Cow::Borrowed(self.source.slice(range))
    }

    /// Returns `true` when `expr` is a qualifying collection whose
    /// inline form would overflow the line-length budget at `column`.
    /// Multi-line input always returns `true`. Single-line input
    /// returns `true` only when `column + inline_length > line_length`.
    fn should_expand(&self, expr: &Expr, column: usize) -> bool {
        let multi_item = match expr {
            Expr::Dict(d) => d.len() > 1,
            Expr::List(l) => l.len() > 1,
            Expr::Set(s) => s.len() > 1,
            _ => return false,
        };
        let range = expr.range();
        multi_item
            && !self.source.intersects_comment(range)
            && (self.source.contains_line_break(range)
                || column + self.source.slice(range).width() > self.line_length)
    }
}

impl<'a> Visitor<'a> for Expander<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        let range = expr.range();
        if !self.should_expand(expr, self.source.column_of(range.start())) {
            walk_expr(self, expr);
            return;
        }
        let indent = self.source.line_indent_width(range.start());
        let replacement = self.expand(expr, indent);
        self.edits
            .extend(narrowed_replacement(self.source, range, replacement));
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
    let mut prefix = Vec::with_capacity(n + 1);
    let mut sum = 0usize;
    prefix.push(sum);
    for &w in widths {
        sum += w;
        prefix.push(sum);
    }
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
    expr.is_literal_expr()
        || is_dotted_name(expr)
        || expr
            .as_unary_op_expr()
            .is_some_and(|u| is_atomic(&u.operand))
}

/// Partitions `atomics` into segments. Every contiguous run of
/// atomic items becomes one `Flow` segment. Every non-atomic item
/// becomes a singleton `OnePerLine` segment. Non-atomic items always
/// break atomic runs.
fn segments(atomics: &[bool]) -> Vec<Segment> {
    let mut start = 0;
    atomics
        .chunk_by(|a, b| a == b)
        .map(|chunk| {
            let range = start..start + chunk.len();
            start += chunk.len();
            if chunk[0] {
                Segment::Flow(range)
            } else {
                Segment::OnePerLine(range)
            }
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
