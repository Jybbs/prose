//! Aligns the `import` keyword across consecutive `from M import N`
//! statements and the `as` keyword across consecutive `import M as A`
//! statements at the same block indentation. Group boundaries are
//! blank lines, own-line comments, form changes, bare imports, and
//! multi-name imports, the two forms aligning independently so a
//! stranded `import M as A` splits a `from`-import group rather than
//! fusing all three. A multi-line import skips alignment, since
//! shifting the keyword would break the continuation indent.

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    Stmt,
    statement_visitor::{StatementVisitor, walk_body},
    token::TokenKind,
};
use ruff_text_size::Ranged;

use crate::{
    config::Config,
    primitives::aligner,
    rule::{Preserves, Rule, RuleId},
    source::Source,
};

pub(crate) struct AlignImports {
    settings: aligner::Settings,
}

impl AlignImports {
    pub(crate) const MESSAGE: &'static str = "align consecutive `import`s";

    pub(crate) const PRESERVES: Preserves = Preserves::Bindings;

    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            settings: config.import_align_settings(),
        }
    }
}

impl Rule for AlignImports {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let mut visitor = Visitor {
            walker: aligner::AlignWalker::new(source, self.settings, Self::SLUG),
        };
        visitor.visit_body(&source.ast().body);
        visitor.walker.groups
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

#[derive(Eq, PartialEq)]
enum Form {
    As,
    From,
}

struct Visitor<'a> {
    walker: aligner::AlignWalker<'a>,
}

impl Visitor<'_> {
    /// Walks `body` once through `aligner::keyed_line_adjacent_groups`,
    /// tagging each qualifying statement with its form. The keyed
    /// grouper closes an active run whenever the form changes at an
    /// otherwise-adjacent boundary, so a stranded `import M as A`
    /// between two `from`-imports splits the surrounding run without
    /// merging its neighbors and without re-walking the body.
    fn process_body(&mut self, body: &[Stmt]) {
        let source = self.walker.source;
        let groups = aligner::keyed_line_adjacent_groups(source, body, self.walker.rule, |s| {
            qualify(source, s)
        });
        for members in groups {
            self.walker.emit_if_candidate(&members);
        }
    }
}

impl<'a> StatementVisitor<'a> for Visitor<'a> {
    fn visit_body(&mut self, body: &'a [Stmt]) {
        self.process_body(body);
        walk_body(self, body);
    }
}

/// Builds an alignment member for a `from M import N` statement,
/// anchored at the `import` keyword. Returns `None` for any other
/// statement shape and for multi-line imports whose continuation
/// indent would misalign if the keyword shifted. Read by the
/// `reflow-imports` forecast as well, so both rules seat one row alike.
pub(crate) fn qualify_from(source: &Source, stmt: &Stmt) -> Option<aligner::Member> {
    let s = stmt.as_import_from_stmt()?;
    if source.contains_line_break(s.range) {
        return None;
    }
    aligner::line_anchored_member_at_kind(source, s.range.start(), s.range, TokenKind::Import)
}

/// Tags a statement with its import form and alignment member, or
/// `None` for a statement that joins no alignment run.
fn qualify(source: &Source, stmt: &Stmt) -> Option<(Form, aligner::Member)> {
    qualify_from(source, stmt)
        .map(|m| (Form::From, m))
        .or_else(|| qualify_import_as(source, stmt).map(|m| (Form::As, m)))
}

/// Builds an alignment member for a single-name aliased import
/// (`import M as A`), anchored at the `as` keyword. Returns `None`
/// for bare imports, multi-name imports, multi-line imports, and any
/// other statement shape.
fn qualify_import_as(source: &Source, stmt: &Stmt) -> Option<aligner::Member> {
    let s = stmt.as_import_stmt()?;
    if source.contains_line_break(s.range) {
        return None;
    }
    let [alias] = s.names.as_slice() else {
        return None;
    };
    let asname = alias.asname.as_ref()?;
    aligner::line_anchored_member_between(source, alias.name.range(), asname.start(), TokenKind::As)
}
