//! Lays out an implicitly concatenated string run one literal per line.
//! A single-line run breaks once the column it lands at carries it past
//! `code_line_length`, whereas a run already spanning lines with two
//! parts sharing one breaks whatever its width, the ragged seam being
//! the defect rather than the line count. A run already one per line
//! stays as written, so the rule only breaks a run and never rejoins
//! one. The run wraps in parentheses where no bracket already carries it
//! and breaks in place where one does. A docstring-slot run, a run whose
//! span holds a comment, and a run carrying a part with its own line
//! break each stay as written.

use std::borrow::Cow;

use ruff_diagnostics::Edit;
use ruff_python_ast::{AnyNodeRef, Expr, StringLike};
use ruff_text_size::{Ranged, TextRange};
use unicode_width::UnicodeWidthStr;

use crate::{
    config::Config,
    primitives::{
        docstring::docstring_slots,
        edit::{narrowed_replacement, singleton_groups},
        layout::{Separator, explode_parens},
        orderer::any_sibling_shares_line,
        reserve,
        tokens::{is_closer, is_opener},
        walk::{Descent, ParentedProbe, walk_parented_exprs},
    },
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct StackAdjacentStrings {
    code_line_length: usize,
    reservations: reserve::Reservations,
}

impl StackAdjacentStrings {
    pub(crate) const MESSAGE: &'static str =
        "stack an implicitly concatenated string run one literal per line";

    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            code_line_length: config.code_width(),
            reservations: config.equals_reservations(),
        }
    }
}

impl Rule for StackAdjacentStrings {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let mut layout = Layout {
            code_line_length: self.code_line_length,
            docstrings: docstring_slots(&source.ast().body),
            edits: Vec::new(),
            newline: source.newline_str(),
            reservations: source.columns(self.reservations),
            source,
        };
        walk_parented_exprs(source.ast(), &mut layout);
        singleton_groups(layout.edits)
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

/// Emits the break each over-budget or raggedly stacked string run
/// needs, probing each expression the parent-tracking walk hands it
/// alongside its enclosing node, which recovers a run's grouping
/// parentheses, and the ancestor chain above it, which locates the
/// statement its bracket depth counts from.
struct Layout<'a> {
    code_line_length: usize,
    docstrings: Vec<TextRange>,
    edits: Vec<Edit>,
    newline: &'static str,
    reservations: Cow<'a, reserve::Columns>,
    source: &'a Source,
}

impl<'a> Layout<'a> {
    /// True when the bracket immediately enclosing `span` has already
    /// put it on a later row, the shape that takes the break in place
    /// rather than in parentheses of its own. A bracket still sharing
    /// the run's row leaves no deeper indent for a continuation to land
    /// at, and an unbracketed run has none at all.
    fn breaks_in_place(&self, span: TextRange, ancestors: &[AnyNodeRef]) -> bool {
        let statement = ancestors
            .iter()
            .rev()
            .find(|node| node.is_statement())
            .expect("an expression is visited inside a statement");
        let mut open = Vec::new();
        for token in self
            .source
            .tokens()
            .in_range(TextRange::new(statement.start(), span.start()))
        {
            let kind = token.kind();
            if is_opener(kind) {
                open.push(token.start());
            } else if is_closer(kind) {
                open.pop();
            }
        }
        open.last()
            .is_some_and(|&bracket| !self.source.same_line(bracket, span.start()))
    }

    /// Emits the one-literal-per-line rewrite `run` needs, or nothing
    /// where the run already reads that way, sits within budget, or falls
    /// under one of the holds.
    fn process_run(&mut self, run: StringLike<'a>, parent: AnyNodeRef, ancestors: &[AnyNodeRef]) {
        let span = run.range();
        if self.docstrings.contains(&span) {
            return;
        }
        let parts: Vec<TextRange> = run.parts().map(|part| part.range()).collect();
        if parts
            .iter()
            .any(|part| self.source.contains_line_break(part))
            || !any_sibling_shares_line(self.source, &parts)
        {
            return;
        }
        let pair = self
            .source
            .paren_aware_range(run.as_expression_ref(), parent);
        if self.source.intersects_comment(pair) {
            return;
        }
        let start = span.start();
        let column = self.reservations.column_in(self.source, start);
        if !self.source.contains_line_break(span)
            && column + self.source.slice(span).width() <= self.code_line_length
        {
            return;
        }
        let texts: Vec<&str> = parts.iter().map(|part| self.source.slice(part)).collect();
        let indent = self.source.line_indent_width(pair.start());
        let text = if pair == span && self.breaks_in_place(span, ancestors) {
            texts.join(&format!("{}{}", self.newline, " ".repeat(indent)))
        } else {
            explode_parens(
                self.newline,
                indent,
                texts.len(),
                |out, i| out.push_str(texts[i]),
                Separator::None,
            )
        };
        self.edits
            .extend(narrowed_replacement(self.source, pair, text));
    }
}

impl<'a> ParentedProbe<'a> for Layout<'a> {
    const INTERPOLATIONS: Descent = Descent::Over;

    /// Probes each expression for a run to break.
    fn probe(
        &mut self,
        expr: &'a Expr,
        parent: AnyNodeRef<'a>,
        ancestors: &[AnyNodeRef<'a>],
    ) -> Descent {
        if let Some(run) = concatenated_run(expr) {
            self.process_run(run, parent, ancestors);
        }
        Descent::Into
    }
}

/// `expr` as a string run when it holds implicitly concatenated parts,
/// covering `str`, `bytes`, f-string, and t-string runs alike.
pub(crate) fn concatenated_run(expr: &Expr) -> Option<StringLike<'_>> {
    StringLike::try_from(expr)
        .ok()
        .filter(|run| run.is_implicit_concatenated())
}
