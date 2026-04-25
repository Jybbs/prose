//! Aligns the `=` character vertically across runs of same-indent,
//! line-adjacent `Stmt::Assign` (single-target), `Stmt::AugAssign`,
//! and `Stmt::AnnAssign` (with initializer) statements. Chained
//! assignments and annotated assignments without an initializer are
//! skipped. Lone rows and singleton sub-groups still collapse their
//! pre-`=` whitespace to one space. `+=` rows place `+` one column
//! before the shared `=` column rather than pushing the `=` right.

use ruff_diagnostics::Edit;
use ruff_python_ast::statement_visitor::{walk_body, StatementVisitor};
use ruff_python_ast::token::TokenKind;
use ruff_python_ast::Stmt;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};
use unicode_width::UnicodeWidthStr;

use crate::config::Config;
use crate::pipeline::Rule;
use crate::primitives::aligner;
use crate::source::Source;

pub struct AlignEquals {
    settings: aligner::Settings,
}

impl AlignEquals {
    pub fn from_config(config: &Config) -> Self {
        let rule = &config.rules.align_equals;
        Self {
            settings: aligner::Settings {
                max_shift: rule.max_shift.get(),
                policy: rule.max_shift_policy,
                suffix_len: 1,
            },
        }
    }
}

impl Rule for AlignEquals {
    fn name(&self) -> &'static str {
        "align-equals"
    }

    fn apply(&self, source: &Source) -> Vec<Edit> {
        let mut visitor = Visitor {
            edits: Vec::new(),
            settings: self.settings,
            source,
        };
        visitor.visit_body(&source.ast().body);
        visitor.edits
    }
}

struct Visitor<'a> {
    edits: Vec<Edit>,
    settings: aligner::Settings,
    source: &'a Source,
}

impl<'a> Visitor<'a> {
    fn process_body(&mut self, body: &[Stmt]) {
        for members in aligner::line_adjacent_groups(self.source, body, |s| self.qualify(s)) {
            aligner::emit_group(self.source, &members, self.settings, &mut self.edits);
        }
    }

    /// Returns the alignment member for `stmt` when it is a shape this
    /// rule can rewrite, or `None` otherwise.
    ///
    /// Three AST shapes qualify: annotated `x: int = 1` (`Stmt::AnnAssign`
    /// with a value), plain `x = 1` (single-target `Stmt::Assign`), and
    /// augmented `x += 1` (`Stmt::AugAssign`). For each, the `width` is
    /// the display-column distance from `target.start()` to the `=`
    /// character, and the `gap` is the whitespace the rule may rewrite.
    /// Returns `None` when the region between `target.start()` and the
    /// `=` contains a line break, since rewriting across a continuation
    /// would flatten the author's multi-line layout.
    fn qualify(&self, stmt: &Stmt) -> Option<aligner::Member> {
        let text = self.source.text();
        let tokens = self.source.tokens();
        let (gap, width) = match stmt {
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
                if text.contains_line_break(TextRange::new(target_range.start(), op.start())) {
                    return None;
                }
                (
                    TextRange::new(target_range.end(), op.start()),
                    self.source.slice(target_range).width() + a.op.as_str().len(),
                )
            }
            _ => return None,
        };
        Some(aligner::Member { gap, width })
    }
}

impl<'a> StatementVisitor<'a> for Visitor<'a> {
    fn visit_body(&mut self, body: &'a [Stmt]) {
        self.process_body(body);
        walk_body(self, body);
    }
}
