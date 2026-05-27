//! Aligns the `import` keyword across the unified block of consecutive
//! bare and `from`-import statements at the same indent. The block
//! absorbs a single blank line between adjacent imports, so a bare run
//! and a `from` run separated only by `blank_lines`'s kind-boundary
//! gap key into one alignment group. Bare `import M as A` statements
//! anchor on their `as` keyword and right-align it against the
//! `import` keyword of `from` statements, so the post-keyword name
//! starts at one shared column across the entire block. Bare imports
//! without an alias and multi-name or multi-line imports sit in the
//! block without participating in alignment.
//!
//! Block scope mirrors `alphabetize::import_block_ranges`, so the two
//! rules see the same notion of "import block" and key on the same
//! gap semantics.

use std::ops::Range;

use ruff_diagnostics::Edit;
use ruff_python_ast::statement_visitor::{walk_body, StatementVisitor};
use ruff_python_ast::token::TokenKind;
use ruff_python_ast::Stmt;
use ruff_text_size::{Ranged, TextRange};

use crate::config::Config;
use crate::primitives::aligner;
use crate::rule::{Rule, RuleId};
use crate::rules::alphabetize::import_block_ranges;
use crate::source::Source;

pub(crate) struct AlignImports {
    settings: aligner::Settings,
}

impl AlignImports {
    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            settings: aligner::Settings::from(&config.rules.align_imports),
        }
    }
}

impl Rule for AlignImports {
    fn apply(&self, source: &Source) -> Vec<Edit> {
        let mut visitor = Visitor {
            walker: aligner::AlignWalker::new(source, self.settings),
        };
        visitor.visit_body(&source.ast().body);
        visitor.walker.edits
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

struct Visitor<'a> {
    walker: aligner::AlignWalker<'a>,
}

impl Visitor<'_> {
    /// Walks `body` once through `import_block_ranges`, collecting
    /// alignment members for every qualifying statement in the block
    /// and emitting one alignment group per block whose members
    /// total at least two.
    fn process_body(&mut self, body: &[Stmt]) {
        for Range { start, end } in import_block_ranges(self.walker.source, body) {
            let members: Vec<aligner::Member> = body[start..end]
                .iter()
                .filter_map(|s| self.qualify(s))
                .collect();
            if members.len() >= 2 {
                self.walker.emit_group(&members);
            }
        }
    }

    /// Builds an alignment member for an import statement. `from M
    /// import N` anchors at the `import` keyword with op-width 6.
    /// `import M as A` (single-name, single-line) anchors at the `as`
    /// keyword with op-width 2. Bare imports without an alias,
    /// multi-name imports, and multi-line imports return `None` and
    /// sit in the block without contributing to or receiving
    /// alignment.
    fn qualify(&self, stmt: &Stmt) -> Option<aligner::Member> {
        let source = self.walker.source;
        if source.contains_line_break(stmt.range()) {
            return None;
        }
        let (range, kind, op_width) = match stmt {
            Stmt::Import(s) => {
                let [alias] = s.names.as_slice() else {
                    return None;
                };
                let asname = alias.asname.as_ref()?;
                (
                    TextRange::new(alias.name.end(), asname.start()),
                    TokenKind::As,
                    "as".len(),
                )
            }
            Stmt::ImportFrom(s) => (s.range, TokenKind::Import, "import".len()),
            _ => unreachable!("qualify is only called on statements inside an import block"),
        };
        aligner::line_anchored_member_at_kind(source, range, kind)
            .map(|m| m.with_op_width(op_width))
    }
}

impl<'a> StatementVisitor<'a> for Visitor<'a> {
    fn visit_body(&mut self, body: &'a [Stmt]) {
        self.process_body(body);
        walk_body(self, body);
    }
}
