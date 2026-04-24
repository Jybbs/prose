//! Aligns the `=` character vertically within consecutive assignment
//! groups.
//!
//! A group is a run of `Stmt::Assign` (single-target), `Stmt::AugAssign`,
//! and `Stmt::AnnAssign` (with an initializer value) statements at the
//! same block indentation on directly adjacent source lines. Any blank
//! line, comment, or non-assignment statement breaks the group. The
//! alignment target is the `=` character, meaning `+=` rows place
//! their `+` one column before the shared `=` column rather than
//! pushing the `=` right.
//!
//! Chained assignments (`a = b = 1`) carry two equals signs and are
//! skipped. Annotated assignments without an initializer (`x: int`)
//! are skipped because they have no `=` to align.
//!
//! A lone assignment, or any singleton sub-group produced by the
//! shift-limit policy, still has its pre-`=` whitespace collapsed to
//! a single space, matching the whitespace-normalization default for
//! all `=` operators.
//!
//! When a group's widest padding exceeds `max_shift`, the configured
//! policy takes over. `Split` greedily partitions the group so each
//! contiguous sub-group satisfies the cap. `Drop` excludes the widest
//! member(s) from alignment math while leaving their spacing
//! untouched. `Skip` leaves the whole group alone.

use ruff_diagnostics::Edit;
use ruff_python_ast::statement_visitor::{walk_body, StatementVisitor};
use ruff_python_ast::token::TokenKind;
use ruff_python_ast::Stmt;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};
use unicode_width::UnicodeWidthStr;

use crate::config::{Config, MaxAlignShiftPolicy};
use crate::pipeline::Rule;
use crate::source::Source;

pub struct AlignEquals {
    max_shift: usize,
    policy: MaxAlignShiftPolicy,
}

impl AlignEquals {
    pub fn from_config(config: &Config) -> Self {
        Self {
            max_shift: config.max_align_shift.get(),
            policy: config.max_align_shift_policy,
        }
    }
}

impl Default for AlignEquals {
    fn default() -> Self {
        Self::from_config(&Config::default())
    }
}

impl Rule for AlignEquals {
    fn name(&self) -> &'static str {
        "align-equals"
    }

    fn apply(&self, source: &Source) -> Vec<Edit> {
        let mut visitor = Visitor {
            edits: Vec::new(),
            max_shift: self.max_shift,
            policy: self.policy,
            source,
        };
        visitor.visit_body(&source.ast().body);
        visitor.edits
    }
}

#[derive(Clone, Copy)]
struct Member {
    /// Display-column width from `target.start()` up to the `=`.
    effective_width: usize,
    /// Whitespace between the effective-LHS region and the operator.
    gap: TextRange,
    /// Full source range of the originating statement, for adjacency checks.
    stmt_range: TextRange,
}

struct Visitor<'a> {
    edits: Vec<Edit>,
    max_shift: usize,
    policy: MaxAlignShiftPolicy,
    source: &'a Source,
}

impl<'a> StatementVisitor<'a> for Visitor<'a> {
    fn visit_body(&mut self, body: &'a [Stmt]) {
        self.process_body(body);
        walk_body(self, body);
    }
}

impl<'a> Visitor<'a> {
    /// Sort by width, then keep only members whose width is within
    /// `max_shift` of the minimum. Retained members align among
    /// themselves. Dropped members get no edit, leaving their
    /// original spacing.
    fn emit_drop(&mut self, members: &[Member]) {
        let mut sorted = members.to_vec();
        sorted.sort_unstable_by_key(|m| m.effective_width);
        let Some(min) = sorted.first().map(|m| m.effective_width) else {
            return;
        };
        let kept_end = sorted.partition_point(|m| m.effective_width <= min + self.max_shift);
        let kept = &sorted[..kept_end];
        if kept.len() < 2 {
            return;
        }
        let max_w = kept.last().expect("kept non-empty").effective_width;
        let paddings: Vec<usize> = kept.iter().map(|m| max_w - m.effective_width).collect();
        self.emit_with_paddings(kept, &paddings);
    }

    fn emit_group(&mut self, members: &[Member]) {
        let max_w = members.iter().map(|m| m.effective_width).max().unwrap_or(0);
        let min_w = members.iter().map(|m| m.effective_width).min().unwrap_or(0);
        if max_w - min_w <= self.max_shift {
            let paddings: Vec<usize> = members.iter().map(|m| max_w - m.effective_width).collect();
            self.emit_with_paddings(members, &paddings);
            return;
        }
        match self.policy {
            MaxAlignShiftPolicy::Skip => {}
            MaxAlignShiftPolicy::Drop => self.emit_drop(members),
            MaxAlignShiftPolicy::Split => self.emit_split(members),
        }
    }

    /// Greedy partitioning: extend the current sub-group while its
    /// widest padding stays under the cap, then start a new sub-group.
    /// Each contiguous sub-group aligns independently. An outlier that
    /// falls into a singleton sub-group collapses to a single space,
    /// matching the whitespace-normalization default for lone `=`.
    fn emit_split(&mut self, members: &[Member]) {
        let mut cursor = 0;
        while cursor < members.len() {
            let mut min_w = members[cursor].effective_width;
            let mut max_w = min_w;
            let mut end = cursor + 1;
            while end < members.len() {
                let w = members[end].effective_width;
                let new_min = min_w.min(w);
                let new_max = max_w.max(w);
                if new_max - new_min > self.max_shift {
                    break;
                }
                min_w = new_min;
                max_w = new_max;
                end += 1;
            }
            let sub = &members[cursor..end];
            let paddings: Vec<usize> = sub.iter().map(|m| max_w - m.effective_width).collect();
            self.emit_with_paddings(sub, &paddings);
            cursor = end;
        }
    }

    fn emit_with_paddings(&mut self, members: &[Member], paddings: &[usize]) {
        let source = self.source;
        self.edits
            .extend(members.iter().zip(paddings).filter_map(|(m, &p)| {
                let target_len = 1 + p;
                let gap_text = source.slice(m.gap);
                let already_correct =
                    gap_text.len() == target_len && gap_text.bytes().all(|b| b == b' ');
                (!already_correct).then(|| Edit::range_replacement(" ".repeat(target_len), m.gap))
            }));
    }

    /// Returns `true` when the gap between adjacent statements carries
    /// exactly one logical or physical newline and no comment, meaning
    /// the surrounding statements sit on directly adjacent source lines.
    fn is_line_adjacent(&self, gap: TextRange) -> bool {
        self.source
            .tokens()
            .in_range(gap)
            .iter()
            .try_fold(0usize, |n, t| match t.kind() {
                k if k.is_comment() => None,
                k if k.is_any_newline() => Some(n + 1),
                _ => Some(n),
            })
            == Some(1)
    }

    fn process_body(&mut self, body: &[Stmt]) {
        let mut iter = body.iter().peekable();
        while let Some(stmt) = iter.next() {
            let Some(first) = self.qualify(stmt) else {
                continue;
            };
            let mut cursor_end = first.stmt_range.end();
            let mut members = vec![first];
            while let Some(next) = iter
                .peek()
                .and_then(|&s| self.qualify(s))
                .filter(|m| self.is_line_adjacent(TextRange::new(cursor_end, m.stmt_range.start())))
            {
                cursor_end = next.stmt_range.end();
                members.push(next);
                iter.next();
            }
            self.emit_group(&members);
        }
    }

    /// Returns the alignment member for `stmt` when it is a shape this
    /// rule can rewrite, or `None` otherwise.
    ///
    /// Three AST shapes qualify. Plain `x = 1` (single-target
    /// `Stmt::Assign`), augmented `x += 1` (`Stmt::AugAssign`), and
    /// annotated `x: int = 1` (`Stmt::AnnAssign` with a value). For
    /// each, the `effective_width` is the display-column distance from
    /// `target.start()` to the `=` character, and the `gap` is the
    /// whitespace the rule may rewrite. Returns `None` when the region
    /// between `target.start()` and the `=` contains a line break,
    /// since rewriting across a continuation would flatten the
    /// author's multi-line layout.
    fn qualify(&self, stmt: &Stmt) -> Option<Member> {
        let text = self.source.text();
        let tokens = self.source.tokens();
        let (gap, effective_width) = match stmt {
            Stmt::Assign(a) => {
                let [target] = a.targets.as_slice() else {
                    return None;
                };
                let target_range = target.range();
                let equal = tokens
                    .in_range(TextRange::new(target_range.end(), a.value.range().start()))
                    .iter()
                    .find(|t| t.kind() == TokenKind::Equal)?;
                if text.contains_line_break(TextRange::new(target_range.start(), equal.start())) {
                    return None;
                }
                (
                    TextRange::new(target_range.end(), equal.start()),
                    self.source.slice(target_range).width(),
                )
            }
            Stmt::AugAssign(a) => {
                let target_range = a.target.range();
                let op = tokens
                    .in_range(TextRange::new(target_range.end(), a.value.range().start()))
                    .iter()
                    .find(|t| t.kind().as_augmented_assign_operator().is_some())?;
                let op_prefix = op.range().sub_end(TextSize::new(1));
                if text.contains_line_break(TextRange::new(target_range.start(), op_prefix.end())) {
                    return None;
                }
                (
                    TextRange::new(target_range.end(), op.start()),
                    self.source.slice(target_range).width() + op_prefix.len().to_usize(),
                )
            }
            Stmt::AnnAssign(a) => {
                let value = a.value.as_deref()?;
                let target_range = a.target.range();
                let annotation_range = a.annotation.range();
                let equal = tokens
                    .in_range(TextRange::new(
                        annotation_range.end(),
                        value.range().start(),
                    ))
                    .iter()
                    .find(|t| t.kind() == TokenKind::Equal)?;
                if text.contains_line_break(TextRange::new(target_range.start(), equal.start())) {
                    return None;
                }
                (
                    TextRange::new(annotation_range.end(), equal.start()),
                    self.source
                        .slice(target_range.cover(annotation_range))
                        .width(),
                )
            }
            _ => return None,
        };
        Some(Member {
            effective_width,
            gap,
            stmt_range: stmt.range(),
        })
    }
}
