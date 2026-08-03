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

use std::collections::HashMap;

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    AnyNodeRef, Arguments, Expr, InterpolatedStringElement, Stmt, StringLike,
    visitor::{Visitor, walk_arguments, walk_expr, walk_stmt},
};
use ruff_text_size::{Ranged, TextRange, TextSize};
use unicode_width::UnicodeWidthStr;

use crate::{
    config::Config,
    primitives::{
        aligner,
        docstring::docstring_slots,
        edit::{narrowed_replacement, singleton_groups},
        layout::{Separator, explode_parens},
        orderer::any_sibling_shares_line,
        reserve::{reserved_columns, settled_column},
        tokens::{is_closer, is_opener},
    },
    rule::{Rule, RuleId},
    rules::align_equals::AlignEquals,
    source::Source,
};

pub(crate) struct StackAdjacentStrings {
    align_equals: Option<aligner::Settings>,
    code_line_length: usize,
}

impl StackAdjacentStrings {
    pub(crate) const MESSAGE: &'static str =
        "stack an implicitly concatenated string run one literal per line";

    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            align_equals: AlignEquals::reserve_settings(config),
            code_line_length: config.code_width(),
        }
    }
}

impl Rule for StackAdjacentStrings {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let body = &source.ast().body;
        let mut layout = Layout {
            code_line_length: self.code_line_length,
            docstrings: docstring_slots(body),
            edits: Vec::new(),
            newline: source.newline_str(),
            parents: vec![AnyNodeRef::from(source.ast())],
            reservations: reserved_columns(source, self.align_equals, AlignEquals::SLUG),
            source,
        };
        layout.visit_body(body);
        singleton_groups(layout.edits)
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

/// Walks a module emitting the break each over-budget or raggedly
/// stacked string run needs. `parents` carries the ancestor chain, which
/// recovers a run's grouping parentheses and locates the statement its
/// bracket depth counts from.
struct Layout<'a> {
    code_line_length: usize,
    docstrings: Vec<TextRange>,
    edits: Vec<Edit>,
    newline: &'static str,
    parents: Vec<AnyNodeRef<'a>>,
    reservations: HashMap<TextSize, usize>,
    source: &'a Source,
}

impl<'a> Layout<'a> {
    /// True when the bracket immediately enclosing `span` has already
    /// put it on a later row, the shape that takes the break in place
    /// rather than in parentheses of its own. A bracket still sharing
    /// the run's row leaves no deeper indent for a continuation to land
    /// at, and an unbracketed run has none at all.
    fn breaks_in_place(&self, span: TextRange) -> bool {
        let statement = self
            .parents
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
    fn process_run(&mut self, run: StringLike<'a>, parent: AnyNodeRef) {
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
        if !self.source.contains_line_break(span)
            && settled_column(&self.reservations, start, || self.source.column_of(start))
                + self.source.slice(span).width()
                <= self.code_line_length
        {
            return;
        }
        let texts: Vec<&str> = parts.iter().map(|part| self.source.slice(part)).collect();
        let indent = self.source.line_indent_width(pair.start());
        let text = if pair == span && self.breaks_in_place(span) {
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

impl<'a> Visitor<'a> for Layout<'a> {
    fn visit_arguments(&mut self, arguments: &'a Arguments) {
        self.parents.push(arguments.into());
        walk_arguments(self, arguments);
        self.parents.pop();
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Some(run) = concatenated_run(expr) {
            let parent = *self.parents.last().expect("seeded with the module node");
            self.process_run(run, parent);
        }
        self.parents.push(expr.into());
        walk_expr(self, expr);
        self.parents.pop();
    }

    /// Leaves a replacement field unwalked.
    fn visit_interpolated_string_element(&mut self, _: &'a InterpolatedStringElement) {}

    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        self.parents.push(stmt.into());
        walk_stmt(self, stmt);
        self.parents.pop();
    }
}

/// `expr` as a string run when it holds implicitly concatenated parts,
/// covering `str`, `bytes`, f-string, and t-string runs alike.
pub(crate) fn concatenated_run(expr: &Expr) -> Option<StringLike<'_>> {
    StringLike::try_from(expr)
        .ok()
        .filter(|run| run.is_implicit_concatenated())
}
