//! Aligns the `=` character vertically across runs of same-indent,
//! line-adjacent `Stmt::Assign` (single-target), `Stmt::AugAssign`,
//! and `Stmt::AnnAssign` (with initializer) statements, and across
//! runs of annotated function parameters carrying default values.
//! Chained assignments, annotated assignments without an initializer,
//! unannotated parameter defaults, and single-line signatures are
//! skipped. Lone rows and singleton sub-groups still collapse their
//! pre-`=` whitespace to one space. `+=` rows place `+` one column
//! before the shared `=` column rather than pushing the `=` right.
//! Parameter widths reflect the post-`align_colons` source.

use ruff_diagnostics::Edit;
use ruff_python_ast::statement_visitor::{walk_body, walk_stmt, StatementVisitor};
use ruff_python_ast::token::TokenKind;
use ruff_python_ast::{AnyParameterRef, Parameters, Stmt};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::config::Config;
use crate::primitives::aligner;
use crate::rule::{Rule, RuleId};
use crate::source::Source;

pub(crate) struct AlignEquals {
    settings: aligner::Settings,
}

impl AlignEquals {
    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            settings: aligner::Settings::from(&config.rules.align_equals),
        }
    }
}

impl Rule for AlignEquals {
    fn apply(&self, source: &Source) -> Vec<Edit> {
        let mut visitor = Visitor {
            edits: Vec::new(),
            settings: self.settings,
            source,
        };
        visitor.visit_body(&source.ast().body);
        visitor.edits
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

struct Visitor<'a> {
    edits: Vec<Edit>,
    settings: aligner::Settings,
    source: &'a Source,
}

impl Visitor<'_> {
    /// Builds an `=`-anchored member with `target` as the LHS span and
    /// the search range running from `target.end()` to `value_start`.
    fn equal_member(&self, target: TextRange, value_start: TextSize) -> Option<aligner::Member> {
        aligner::range_anchored_member_single_line(
            self.source,
            target,
            TextRange::new(target.end(), value_start),
            |t| t.kind() == TokenKind::Equal,
            0,
        )
    }

    fn process_body(&mut self, body: &[Stmt]) {
        for members in aligner::line_adjacent_groups(self.source, body, |s| self.qualify(s)) {
            aligner::emit_group(self.source, &members, self.settings, &mut self.edits);
        }
    }

    /// Walks `params` through [`aligner::parameter_split_groups`] with
    /// [`Self::qualify_parameter`], emitting an alignment pass for
    /// each sub-group that clears [`aligner::is_alignment_candidate`].
    fn process_parameters(&mut self, params: &Parameters) {
        for members in aligner::parameter_split_groups(params, |p| self.qualify_parameter(p)) {
            if aligner::is_alignment_candidate(&members) {
                aligner::emit_group(self.source, &members, self.settings, &mut self.edits);
            }
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
    /// `=` contains a line break.
    fn qualify(&self, stmt: &Stmt) -> Option<aligner::Member> {
        match stmt {
            Stmt::AnnAssign(a) => {
                let value = a.value.as_deref()?;
                self.equal_member(a.target.range().cover(a.annotation.range()), value.start())
            }
            Stmt::Assign(a) => {
                let [target] = a.targets.as_slice() else {
                    return None;
                };
                self.equal_member(target.range(), a.value.start())
            }
            Stmt::AugAssign(a) => {
                let target_range = a.target.range();
                aligner::range_anchored_member_single_line(
                    self.source,
                    target_range,
                    TextRange::new(target_range.end(), a.value.start()),
                    |t| t.kind().as_augmented_assign_operator().is_some(),
                    a.op.as_str().len(),
                )
            }
            _ => None,
        }
    }

    /// Returns the alignment member for an annotated function parameter
    /// carrying a default value, or `None` for any other shape. Width
    /// spans the parameter name through the annotation end, and the
    /// gap is the whitespace between annotation end and the `=` token.
    fn qualify_parameter(&self, param: AnyParameterRef<'_>) -> Option<aligner::Member> {
        let annotation = param.annotation()?;
        let default = param.default()?;
        self.equal_member(
            TextRange::new(param.name().start(), annotation.end()),
            default.start(),
        )
    }
}

impl<'a> StatementVisitor<'a> for Visitor<'a> {
    fn visit_body(&mut self, body: &'a [Stmt]) {
        self.process_body(body);
        walk_body(self, body);
    }

    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::FunctionDef(fd) = stmt {
            self.process_parameters(&fd.parameters);
        }
        walk_stmt(self, stmt);
    }
}
