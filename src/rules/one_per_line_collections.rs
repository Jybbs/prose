//! Expands multi-item `dict`, `list`, and `set` literals when the
//! inline form would overflow the line-length budget. Literals that
//! already fit stay inline.
//!
//! The budget comes from `Config::line_length`, defaulting to 88 when
//! unset. A qualifying literal is rewritten when
//! `column_of(opening_bracket) + inline_length > line_length`. Input
//! that is already multi-line is always canonicalized via expansion,
//! so oddball layouts normalize to the canonical form.
//!
//! Inside an expanded list or set, items are grouped into runs of
//! contiguous atomic items (literals, names, attribute chains, unary
//! negations of atomic expressions) separated by non-atomic items.
//! Atomic runs always flow-pack: items fill each line up to
//! `max_atomics_per_line` (defaulting to 8 when unset) and up to the
//! line-length budget, whichever cap is tighter, across as few lines
//! as those caps allow and distributed to roughly balanced line
//! widths. Non-atomic items always break atomic runs and get their
//! own line. `*` and `**` unpackings are non-atomic, so a spread in
//! the middle of a list splits the surrounding atomics into two
//! independent runs that each flow on their own. A matrix is a list
//! of lists (non-atomic rows), so rows stay one-per-line and each
//! row's short atomic contents stay inline because they still fit at
//! the row's own column.
//!
//! Dict items are never treated as atomic, even when both key and
//! value render atomically. The `key: value` pair has internal
//! structure that benefits from a dedicated line, both for readability
//! and so that `align_colons` downstream has rows to align across.
//!
//! Nested literals check fit independently at their own column,
//! including the key-offset for dict values, so tiered structures
//! expand level-by-level only where the inline form would overflow.
//!
//! Comprehensions (`ListComp`, `SetComp`, `DictComp`, `GeneratorExp`)
//! and tuple literals are out of scope. Literals whose source range
//! contains a comment are also left alone, because slicing through a
//! comment would drop author-intended prose.
//!
//! The rule runs ahead of `align_colons` in the pipeline, so what
//! `align_colons` sees is the canonical form.
//!
//! The extra indent step is four spaces, matching PEP 8 and the
//! chalkline style guide. Trailing commas are not emitted on the
//! final item of the whole collection; `strip_trailing_commas` owns
//! that concern elsewhere in the pipeline.

use std::num::NonZeroUsize;
use std::ops::Range;

use ruff_diagnostics::Edit;
use ruff_python_ast::visitor::{walk_expr, Visitor};
use ruff_python_ast::{DictItem, Expr};
use ruff_python_trivia::CommentRanges;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::config::Config;
use crate::pipeline::Rule;
use crate::source::Source;

const DEFAULT_LINE_LENGTH: usize = 88;
const DEFAULT_MAX_ATOMICS_PER_LINE: usize = 8;
const INDENT_STEP: usize = 4;

pub struct OnePerLineCollections {
    line_length: usize,
    max_atomics_per_line: usize,
}

impl OnePerLineCollections {
    pub fn from_config(config: &Config) -> Self {
        Self {
            line_length: config
                .line_length
                .map_or(DEFAULT_LINE_LENGTH, NonZeroUsize::get),
            max_atomics_per_line: config
                .max_atomics_per_line
                .map_or(DEFAULT_MAX_ATOMICS_PER_LINE, NonZeroUsize::get),
        }
    }
}

impl Default for OnePerLineCollections {
    fn default() -> Self {
        Self::from_config(&Config::default())
    }
}

impl Rule for OnePerLineCollections {
    fn name(&self) -> &'static str {
        "one-per-line-collections"
    }

    fn apply(&self, source: &Source) -> Vec<Edit> {
        let mut visitor = Expander {
            comment_ranges: CommentRanges::from(source.tokens()),
            edits: Vec::new(),
            line_length: self.line_length,
            max_atomics_per_line: self.max_atomics_per_line,
            source,
        };
        visitor.visit_body(&source.ast().body);
        visitor.edits
    }
}

struct Expander<'a> {
    comment_ranges: CommentRanges,
    edits: Vec<Edit>,
    line_length: usize,
    max_atomics_per_line: usize,
    source: &'a Source,
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

impl Expander<'_> {
    /// Returns the zero-indexed character column of `offset` on its line.
    fn column_of(&self, offset: TextSize) -> usize {
        self.source.line_col(offset).column.to_zero_indexed()
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
        out.push('\n');
        for segment in segments(&atomics) {
            match segment {
                Segment::OnePerLine(range) => {
                    for idx in range {
                        out.push_str(&item_prefix);
                        out.push_str(&texts[idx]);
                        if idx + 1 < total {
                            out.push(',');
                        }
                        out.push('\n');
                    }
                }
                Segment::Flow(range) => {
                    let widths: Vec<usize> =
                        range.clone().map(|i| texts[i].chars().count()).collect();
                    for line_range in flow_lines(&widths, available, self.max_atomics_per_line) {
                        out.push_str(&item_prefix);
                        let absolute =
                            (range.start + line_range.start)..(range.start + line_range.end);
                        for (line_pos, idx) in absolute.clone().enumerate() {
                            out.push_str(&texts[idx]);
                            let is_last_overall = idx + 1 == total;
                            let is_last_on_line = line_pos + 1 == absolute.len();
                            if !is_last_overall {
                                out.push(',');
                                if !is_last_on_line {
                                    out.push(' ');
                                }
                            }
                        }
                        out.push('\n');
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
        if let Expr::Dict(d) = expr {
            let texts = d
                .items
                .iter()
                .map(|item| self.serialize_dict_item(item, indent))
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
        let texts = elts
            .iter()
            .map(|e| self.serialize_expr(e, indent, indent))
            .collect();
        let atomics = elts.iter().map(is_atomic).collect();
        (open, close, texts, atomics)
    }

    /// Returns the leading-whitespace column of the line containing
    /// `offset`. Tabs count as one column each; the rule's output uses
    /// spaces only, so tab-indented input is preserved on non-rewritten
    /// lines and re-emitted as spaces on rewritten ones.
    fn line_indent(&self, offset: TextSize) -> usize {
        let text = self.source.text();
        let line_start = text.line_start(offset).to_usize();
        text[line_start..]
            .bytes()
            .take_while(|&b| b == b' ' || b == b'\t')
            .count()
    }

    /// Serializes a dict item as `key: value` or `**value`.
    ///
    /// `indent` is the column where the item sits (the item-indent of
    /// the enclosing dict). The value's actual column for the
    /// line-length check is offset by the key text plus `": "`, so a
    /// long key that pushes its value past the budget correctly
    /// triggers expansion of the value. When the value does expand,
    /// its closing bracket still lands at `indent`.
    fn serialize_dict_item(&self, item: &DictItem, indent: usize) -> String {
        match &item.key {
            Some(key) => {
                let key_text = self.source.slice(key);
                let value_column = indent + key_text.chars().count() + 2;
                let value_text = self.serialize_expr(&item.value, value_column, indent);
                format!("{key_text}: {value_text}")
            }
            None => {
                let value_text = self.serialize_expr(&item.value, indent + 2, indent);
                format!("**{value_text}")
            }
        }
    }

    /// Serializes `expr` into the expanded output.
    ///
    /// `column` is where the expression actually begins on its line
    /// (used for the line-length overflow check). `indent` is where
    /// its closing bracket should land if it expands. They differ for
    /// dict values, where the key text sits between the line indent
    /// and the value's own starting column.
    fn serialize_expr(&self, expr: &Expr, column: usize, indent: usize) -> String {
        if self.should_expand(expr, column) {
            return self.expand(expr, indent);
        }
        self.source.slice(expr.range()).to_owned()
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
        if self.contains_comment(expr.range()) {
            return false;
        }
        let current = self.source.slice(expr.range());
        current.contains('\n') || column + current.chars().count() > self.line_length
    }
}

/// Describes how a contiguous slice of items should lay out.
enum Segment {
    /// Each item in the range goes on its own line.
    OnePerLine(Range<usize>),
    /// Items in the range flow across as few balanced lines as fit.
    Flow(Range<usize>),
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
    let by_width = total_slot_width(widths).div_ceil(available.max(1));
    let by_cap = n.div_ceil(max_atomics.max(1));
    let initial = by_width.max(by_cap).max(1);
    for num_lines in initial..=n {
        if let Some(lines) = try_even(widths, num_lines, available, max_atomics) {
            return lines;
        }
    }
    (0..n).map(|i| i..(i + 1)).collect()
}

/// Attempts an even distribution of `widths.len()` items across
/// `num_lines` lines. Returns `Some(lines)` when every line's item
/// count stays at or below `max_atomics` and every line's slot width
/// fits in `available`, `None` otherwise. First `n % num_lines` lines
/// carry one extra item so the earlier lines absorb the remainder
/// rather than the last line being the short one.
fn try_even(
    widths: &[usize],
    num_lines: usize,
    available: usize,
    max_atomics: usize,
) -> Option<Vec<Range<usize>>> {
    let n = widths.len();
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
        let sum: usize = widths[start..end].iter().sum();
        let slot = sum + 2 * size.saturating_sub(1);
        if slot > available {
            return None;
        }
        lines.push(start..end);
        start = end;
    }
    Some(lines)
}

/// Returns `true` for expressions that render as a single compact
/// token and therefore do not benefit from a dedicated line.
///
/// Covers literal values, bare names, attribute chains over atomic
/// bases (*`pkg.CONST`, `cls.name.upper`*), and unary operations over
/// atomic operands (*`-1`, `not flag`*). Starred expressions (`*name`,
/// `**name`) are intentionally non-atomic so a spread in the middle
/// of a list or set splits its surrounding atomics into two
/// independent runs that each flow on their own. Everything else
/// (calls, comparisons, arithmetic, nested collections, comprehensions)
/// is considered structured and earns a line of its own in the output.
fn is_atomic(expr: &Expr) -> bool {
    if expr.is_literal_expr() {
        return true;
    }
    match expr {
        Expr::Attribute(a) => is_atomic(&a.value),
        Expr::Name(_) => true,
        Expr::UnaryOp(u) => is_atomic(&u.operand),
        _ => false,
    }
}

/// Partitions `atomics` into segments. Every contiguous run of
/// atomic items becomes one `Flow` segment; every non-atomic item
/// becomes a singleton `OnePerLine` segment. Non-atomic items always
/// break atomic runs.
fn segments(atomics: &[bool]) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut i = 0;
    while i < atomics.len() {
        if atomics[i] {
            let run_start = i;
            while i < atomics.len() && atomics[i] {
                i += 1;
            }
            segments.push(Segment::Flow(run_start..i));
        } else {
            segments.push(Segment::OnePerLine(i..i + 1));
            i += 1;
        }
    }
    segments
}

/// Returns the total width `widths` occupy when laid out with
/// `", "` separators between consecutive items.
fn total_slot_width(widths: &[usize]) -> usize {
    let sum: usize = widths.iter().sum();
    sum + 2 * widths.len().saturating_sub(1)
}
