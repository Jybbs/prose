//! Strips the pre-`:` padding on aligned contexts whose `:`s have no
//! column to align to. The two cases are a singleton group (one
//! member, so no neighbor row) and a multi-member group whose `:`s
//! all share a source line (no column distinction across rows). Both
//! reduce to "alignment is not happening here," at which point the
//! pre-`:` gap is visual noise and the rule strips it. Multi-member
//! groups whose `:`s sit on distinct lines belong to `align_colons`
//! and pass through this rule untouched. Runs after the alignment
//! rules in `Pipeline::with_defaults` so it sees their output.

use ruff_diagnostics::Edit;
use ruff_python_ast::visitor::{walk_expr, walk_parameters, walk_stmt, Visitor as AstVisitor};
use ruff_python_ast::{Expr, ExprDict, Parameters, Stmt};

use crate::config::Config;
use crate::pipeline::Rule;
use crate::primitives::{aligner, colon_targets};
use crate::source::Source;

pub struct SingletonRule;

impl SingletonRule {
    pub fn from_config(_config: &Config) -> Self {
        Self
    }
}

impl Rule for SingletonRule {
    fn apply(&self, source: &Source) -> Vec<Edit> {
        let mut visitor = Visitor {
            edits: Vec::new(),
            source,
        };
        visitor.visit_body(&source.ast().body);
        visitor.edits
    }

    fn name(&self) -> &'static str {
        "singleton-rule"
    }
}

struct Visitor<'a> {
    edits: Vec<Edit>,
    source: &'a Source,
}

impl Visitor<'_> {
    /// Emits a deletion edit per member when alignment is not
    /// happening for the group. Singleton groups always qualify, since
    /// a single row has nothing to align against. Multi-member groups
    /// qualify when their `:`s share a source line, since no column
    /// distinguishes the rows. Multi-member groups on distinct lines
    /// belong to `align_colons` and emit nothing here. The `width > 0`
    /// guard rejects the edge case where a `:` sits on its own
    /// indented line and the "gap" is leading indent rather than
    /// padding.
    fn emit(&mut self, members: &[aligner::Member]) {
        if members.is_empty() || (members.len() >= 2 && aligner::distinct_lines(members)) {
            return;
        }
        self.edits.extend(
            members
                .iter()
                .filter(|m| m.width > 0 && !m.gap.is_empty())
                .map(|m| Edit::range_deletion(m.gap)),
        );
    }

    fn process_class_fields(&mut self, body: &[Stmt]) {
        for group in colon_targets::class_field_groups(self.source, body) {
            self.emit(&group);
        }
    }

    fn process_dict(&mut self, d: &ExprDict) {
        self.emit(&colon_targets::dict_members(self.source, d));
    }

    fn process_docstring_args(&mut self, body: &[Stmt]) {
        self.emit(&colon_targets::docstring_args(self.source, body));
    }

    fn process_parameters(&mut self, params: &Parameters) {
        for group in colon_targets::parameter_groups(self.source, params) {
            self.emit(&group);
        }
    }
}

impl<'a> AstVisitor<'a> for Visitor<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Dict(d) = expr {
            self.process_dict(d);
        }
        walk_expr(self, expr);
    }

    fn visit_parameters(&mut self, parameters: &'a Parameters) {
        self.process_parameters(parameters);
        walk_parameters(self, parameters);
    }

    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::ClassDef(cd) => {
                self.process_class_fields(&cd.body);
                self.process_docstring_args(&cd.body);
            }
            Stmt::FunctionDef(fd) => {
                self.process_docstring_args(&fd.body);
            }
            _ => {}
        }
        walk_stmt(self, stmt);
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use ruff_text_size::{Ranged, TextRange, TextSize};

    use super::*;

    /// Builds a synthetic source carrying `text_len` ASCII bytes, used
    /// as the backing text for fabricated `Member`s. Tests bypass the
    /// AST walk and call `emit` directly to pin its gating logic.
    fn synthetic_source(text_len: usize) -> Source {
        Source::from_str(&"x".repeat(text_len)).expect("synthetic ASCII source parses")
    }

    /// Drives `emit` against `members` and returns the resulting edits.
    fn run_emit(source: &Source, members: &[aligner::Member]) -> Vec<Edit> {
        let mut visitor = Visitor {
            edits: Vec::new(),
            source,
        };
        visitor.emit(members);
        visitor.edits
    }

    #[test]
    fn emit_handles_empty_members_slice() {
        // No members: `emit` returns immediately, no edits.
        let source = synthetic_source(0);
        assert!(run_emit(&source, &[]).is_empty());
    }

    #[test]
    fn emit_skips_multi_member_groups_on_distinct_lines() {
        // Two members on different source lines: `distinct_lines` is
        // true, alignment defers to `align_colons`, this rule emits
        // nothing.
        let source = Source::from_str("xx: 1\nyy: 2\n").expect("parses");
        let members = [
            aligner::Member {
                gap: TextRange::new(TextSize::new(2), TextSize::new(2)),
                line_start: TextSize::new(0),
                width: 2,
            },
            aligner::Member {
                gap: TextRange::new(TextSize::new(8), TextSize::new(8)),
                line_start: TextSize::new(6),
                width: 2,
            },
        ];
        assert!(run_emit(&source, &members).is_empty());
    }

    #[test]
    fn emit_skips_zero_width_member_with_empty_gap() {
        // The "anchor at line start" shape that `line_anchored_member`
        // produces for any token at column 0. Width and gap both zero,
        // nothing to delete, no edit emitted.
        let source = synthetic_source(1);
        let member = aligner::Member {
            gap: TextRange::new(TextSize::new(0), TextSize::new(0)),
            line_start: TextSize::new(0),
            width: 0,
        };
        assert!(run_emit(&source, &[member]).is_empty());
    }

    #[test]
    fn emit_skips_zero_width_member_with_indent_gap() {
        // The "`:` on its own indented line" shape: the line has only
        // whitespace before the colon, so the line-anchored width is
        // zero and the "gap" is leading indent rather than padding.
        let source = synthetic_source(8);
        let member = aligner::Member {
            gap: TextRange::new(TextSize::new(0), TextSize::new(4)),
            line_start: TextSize::new(0),
            width: 0,
        };
        assert!(run_emit(&source, &[member]).is_empty());
    }

    #[test]
    fn emit_strips_every_member_when_colons_share_a_line() {
        // Two members on the same source line: `distinct_lines` is
        // false, so alignment defers to this rule and every member's
        // gap is deleted.
        let source = synthetic_source(20);
        let members = [
            aligner::Member {
                gap: TextRange::new(TextSize::new(3), TextSize::new(5)),
                line_start: TextSize::new(0),
                width: 3,
            },
            aligner::Member {
                gap: TextRange::new(TextSize::new(8), TextSize::new(10)),
                line_start: TextSize::new(0),
                width: 5,
            },
        ];
        assert_eq!(run_emit(&source, &members).len(), 2);
    }

    #[test]
    fn emit_strips_singleton_with_content_and_gap() {
        // The canonical singleton: content on the same line as the
        // colon, non-empty pre-colon gap. Both guards pass, the gap
        // is deleted.
        let source = synthetic_source(8);
        let member = aligner::Member {
            gap: TextRange::new(TextSize::new(3), TextSize::new(5)),
            line_start: TextSize::new(0),
            width: 3,
        };
        let edits = run_emit(&source, &[member]);
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].start(), TextSize::new(3));
        assert_eq!(edits[0].end(), TextSize::new(5));
    }
}
