//! Expands multi-item `dict`, `list`, and `set` literals when the
//! inline form would overflow `Config::line_length`. Comprehensions,
//! tuple literals, and any literal whose source range contains a
//! comment are out of scope.

use std::num::NonZeroUsize;
use std::ops::Range;

use ruff_diagnostics::Edit;
use ruff_python_ast::helpers::is_dotted_name;
use ruff_python_ast::token::parenthesized_range;
use ruff_python_ast::visitor::{walk_expr, Visitor};
use ruff_python_ast::{AnyNodeRef, DictItem, Expr};
use ruff_python_trivia::CommentRanges;
use ruff_source_file::{find_newline, LineEnding, LineRanges};
use ruff_text_size::{Ranged, TextRange, TextSize};
use unicode_width::UnicodeWidthStr;

use crate::config::Config;
use crate::pipeline::Rule;
use crate::primitives::locator;
use crate::source::Source;

const DEFAULT_LINE_LENGTH: usize = 88;
const DEFAULT_MAX_ATOMICS_PER_LINE: usize = 8;
const INDENT_STEP: usize = 4;

pub struct CollectionLayout {
    line_length: usize,
    max_atomics_per_line: usize,
}

impl CollectionLayout {
    pub fn from_config(config: &Config) -> Self {
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
        let newline = find_newline(source.text())
            .map_or(LineEnding::Lf, |(_, ending)| ending)
            .as_str();
        let mut visitor = Expander {
            comment_ranges: CommentRanges::from(source.tokens()),
            edits: Vec::new(),
            line_length: self.line_length,
            max_atomics_per_line: self.max_atomics_per_line,
            newline,
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
    comment_ranges: CommentRanges,
    edits: Vec<Edit>,
    line_length: usize,
    max_atomics_per_line: usize,
    newline: &'static str,
    source: &'a Source,
}

impl Expander<'_> {
    fn column_of(&self, offset: TextSize) -> usize {
        locator::column_of(self.source, offset)
    }

    /// Returns `true` when a comment falls inside `range`, meaning the
    /// rule must not rewrite this literal because slicing through it
    /// would drop the comment. Backed by a per-`apply` `CommentRanges`,
    /// so the lookup is a binary search rather than a token-stream
    /// walk.
    fn contains_comment(&self, range: TextRange) -> bool {
        self.comment_ranges.intersects(range)
    }

    /// Builds the expanded form of `expr` as a string, recursively
    /// expanding any qualifying child collections so the caller's
    /// single `range_replacement` covers every descendant rewrite.
    fn expand(&self, expr: &Expr, indent: usize) -> String {
        let item_indent = indent + INDENT_STEP;
        let (open, close, texts, atomics) = self.gather_items(expr, item_indent);
        let total = texts.len();
        let item_prefix = " ".repeat(item_indent);
        let close_prefix = " ".repeat(indent);
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
                    }
                }
                Segment::Flow(range) => {
                    let run_start = range.start;
                    let widths: Vec<usize> = range.map(|i| texts[i].width()).collect();
                    for line_range in flow_lines(&widths, available, self.max_atomics_per_line) {
                        out.push_str(&item_prefix);
                        let line_size = line_range.len();
                        let absolute = (run_start + line_range.start)..(run_start + line_range.end);
                        for (line_pos, idx) in absolute.enumerate() {
                            out.push_str(&texts[idx]);
                            if idx + 1 != total {
                                out.push(',');
                                if line_pos + 1 != line_size {
                                    out.push(' ');
                                }
                            }
                        }
                        out.push_str(self.newline);
                    }
                }
            }
        }
        out.push_str(&close_prefix);
        out.push(close);
        out
    }

    /// Collects the bracket pair, per-item serialized text, and
    /// per-item atomicity for the collection at `expr`. The text is
    /// produced via `serialize_expr` / `serialize_dict_item` at
    /// `indent`, so nested qualifying children are already recursively
    /// expanded in the returned strings.
    fn gather_items(&self, expr: &Expr, indent: usize) -> (char, char, Vec<String>, Vec<bool>) {
        let parent = AnyNodeRef::from(expr);
        if let Expr::Dict(d) = expr {
            let texts = d
                .items
                .iter()
                .map(|item| self.serialize_dict_item(item, parent, indent))
                .collect();
            // Dict items are never treated as atomic, even when both
            // key and value render atomically. The `key: value` pair
            // has internal structure that benefits from a dedicated
            // line, both for readability and so that `align_colons`
            // has rows to align across.
            let atomics = vec![false; d.items.len()];
            return ('{', '}', texts, atomics);
        }
        let (open, close, elts) = match expr {
            Expr::List(l) => ('[', ']', &l.elts),
            Expr::Set(s) => ('{', '}', &s.elts),
            _ => unreachable!("gather_items called on non-collection expr"),
        };
        let (texts, atomics): (Vec<String>, Vec<bool>) = elts
            .iter()
            .map(|e| (self.serialize_expr(e, parent, indent, indent), is_atomic(e)))
            .unzip();
        (open, close, texts, atomics)
    }

    /// Returns the leading-whitespace column of the line containing
    /// `offset`, in characters. Tabs and form-feeds count as one
    /// column each. The rule's output uses spaces only, so tab-indented
    /// input is preserved on non-rewritten lines and re-emitted as
    /// spaces on rewritten ones.
    fn line_indent(&self, offset: TextSize) -> usize {
        locator::line_indent_width(self.source, offset)
    }

    /// Serializes a dict item as `key: value` or `**value`.
    ///
    /// `indent` is the column where the item sits (the item-indent of
    /// the enclosing dict). The value's actual column for the
    /// line-length check is offset by the key text plus `": "`, so a
    /// long key that pushes its value past the budget correctly
    /// triggers expansion of the value. When the value does expand,
    /// its closing bracket still lands at `indent`.
    fn serialize_dict_item(&self, item: &DictItem, parent: AnyNodeRef, indent: usize) -> String {
        if let Some(key) = &item.key {
            let key_text = self.source.slice(key);
            let value_column = indent + key_text.width() + 2;
            let value_text = self.serialize_expr(&item.value, parent, value_column, indent);
            format!("{key_text}: {value_text}")
        } else {
            let value_text = self.serialize_expr(&item.value, parent, indent + 2, indent);
            format!("**{value_text}")
        }
    }

    /// Serializes `expr` into the expanded output.
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
    ) -> String {
        if self.should_expand(expr, column) {
            return self.expand(expr, indent);
        }
        let range = parenthesized_range(expr.into(), parent, self.source.tokens())
            .unwrap_or_else(|| expr.range());
        self.source.slice(range).to_owned()
    }

    /// Returns `true` when `expr` is a qualifying collection whose
    /// inline form would overflow the line-length budget at `column`.
    ///
    /// Multi-line input always returns `true` so that oddball layouts
    /// canonicalize through `expand`. Single-line input only returns
    /// `true` when `column + inline_length > line_length`.
    fn should_expand(&self, expr: &Expr, column: usize) -> bool {
        let count = match expr {
            Expr::Dict(d) => d.items.len(),
            Expr::List(l) => l.elts.len(),
            Expr::Set(s) => s.elts.len(),
            _ => return false,
        };
        if count <= 1 {
            return false;
        }
        let range = expr.range();
        if self.contains_comment(range) {
            return false;
        }
        self.source.text().contains_line_break(range)
            || column + self.source.slice(range).width() > self.line_length
    }
}

impl<'a> Visitor<'a> for Expander<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        let column = self.column_of(expr.start());
        if self.should_expand(expr, column) {
            let indent = self.line_indent(expr.start());
            let replacement = self.expand(expr, indent);
            if replacement != self.source.slice(expr.range()) {
                self.edits
                    .push(Edit::range_replacement(replacement, expr.range()));
            }
            return;
        }
        walk_expr(self, expr);
    }
}

/// Describes how a contiguous slice of items should lay out.
enum Segment {
    /// Items in the range flow across as few balanced lines as fit.
    Flow(Range<usize>),
    /// Each item in the range goes on its own line.
    OnePerLine(Range<usize>),
}

/// Distributes items into the smallest number of lines such that no
/// line exceeds `available` characters and no line carries more than
/// `max_atomics` items, giving roughly equal item counts per line for
/// visual balance. Escalates to more lines if the even split at the
/// minimum line count would still overflow either cap.
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

/// Returns `true` for expressions that render as a single compact
/// token and therefore do not benefit from a dedicated line.
///
/// Covers literal values, dotted names (`pkg`, `cls.name.upper`,
/// reached via `ruff_python_ast::helpers::is_dotted_name`), and unary
/// operations over atomic operands (`-1`, `not flag`). Starred
/// expressions (`*name`, `**name`) are intentionally non-atomic so a
/// spread in the middle of a list or set splits its surrounding
/// atomics into two independent runs that each flow on their own.
/// Everything else (calls, comparisons, arithmetic, nested
/// collections, comprehensions) is considered structured and earns a
/// line of its own in the output.
fn is_atomic(expr: &Expr) -> bool {
    expr.is_literal_expr()
        || is_dotted_name(expr)
        || matches!(expr, Expr::UnaryOp(u) if is_atomic(&u.operand))
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
