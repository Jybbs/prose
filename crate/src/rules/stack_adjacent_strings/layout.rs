//! The stacked layout an adjacent-string run settles to.

use std::borrow::Cow;

use super::*;
use crate::primitives::inline::display_width;

/// Emits the break each over-budget or raggedly stacked string run
/// needs, probing each expression the parent-tracking walk hands it
/// alongside its enclosing node, which recovers a run's grouping
/// parentheses, and the ancestor chain above it, which locates the
/// statement its bracket depth counts from.
pub(super) struct Layout<'a> {
    pub(super) code_line_length: usize,
    pub(super) docstrings: Vec<TextRange>,
    pub(super) edits: Vec<Edit>,
    pub(super) newline: &'static str,
    pub(super) reservations: Cow<'a, reserve::Columns>,
    pub(super) source: &'a Source,
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
            && column + display_width(self.source.slice(span)) <= self.code_line_length
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
