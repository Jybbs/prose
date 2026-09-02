//! The canonical text a signature lands as, inline or one parameter per
//! line, each parameter placed where its row falls.

use std::borrow::Cow;

use ruff_python_ast::{AnyParameterRef, Parameters, StmtFunctionDef};
use ruff_text_size::{Ranged, TextRange};

use super::{Layout, terms::Expansion};
use crate::primitives::{
    inline::{end_column, opening_width, spans_rows},
    layout::{Separator, explode_parens},
    params::parameter_sites,
    range::return_annotation_range,
    travel::{Landing, placed_block},
};

impl Expansion<'_> {
    /// Builds the canonical inline text spanning `(` through `:` from
    /// `parts`.
    pub(super) fn build_inline(&self, fd: &StmtFunctionDef, parts: &[Cow<str>]) -> String {
        let mut out = format!("({})", parts.join(", "));
        self.push_return_and_colon(&mut out, fd);
        out
    }

    fn push_return_and_colon(&self, out: &mut String, fd: &StmtFunctionDef) {
        if let Some(ret) = fd.returns.as_deref() {
            out.push_str(" -> ");
            let range = return_annotation_range(ret, fd, self.source);
            out.push_str(self.source.slice(range));
        }
        out.push(':');
    }
}

impl Layout<'_> {
    /// Builds the canonical expanded text spanning `(` through `:` from
    /// `parts`, one parameter per line.
    pub(super) fn build_expanded(
        &self,
        fd: &StmtFunctionDef,
        parts: &[Cow<str>],
        indent: usize,
    ) -> String {
        let mut out = explode_parens(
            self.newline,
            indent,
            parts.len(),
            |out, i| out.push_str(&parts[i]),
            Separator::Comma,
        );
        self.expansion.push_return_and_colon(&mut out, fd);
        out
    }

    /// `param`'s text placed at `indent` with `tail` columns closing its
    /// last row, every call inside its annotation and default reshaped
    /// where it lands, or the source text moved whole where none
    /// reshapes. A variadic parameter carries its `*` or `**` prefix and
    /// holds no default.
    pub(super) fn place<'p>(
        &'p self,
        param: AnyParameterRef,
        indent: usize,
        tail: usize,
    ) -> Cow<'p, str> {
        self.reshaped(param, indent, tail).map_or_else(
            || {
                placed_block(
                    self.source,
                    param.range(),
                    Landing::own_row(param.start(), indent),
                )
            },
            Cow::Owned,
        )
    }

    /// `param`'s text at `indent` with the calls inside its annotation
    /// and its default reshaped where each lands, `tail` the columns
    /// closing the last row. Each site measures from the column the text
    /// ahead of it ends at and across the opening row of the text after
    /// it, and one no call inside reshapes moves whole. `None` where no
    /// site reshapes, or where text between two sites spans rows.
    fn reshaped(&self, param: AnyParameterRef, indent: usize, tail: usize) -> Option<String> {
        let mut out = String::new();
        let mut cursor = param.start();
        let mut reshaped = false;
        for (expr, parent) in parameter_sites(param) {
            let held = self.source.paren_aware_range(expr.into(), parent);
            let gap = TextRange::new(cursor, held.start());
            if self.source.contains_line_break(gap) {
                return None;
            }
            out.push_str(self.source.slice(gap));
            let landing = Landing {
                column: end_column(&out, indent),
                indent,
                item: param.start(),
            };
            let rest = self.source.slice(TextRange::new(held.end(), param.end()));
            let site_tail = opening_width(rest) + if spans_rows(rest) { 0 } else { tail };
            match self.reshaper.reshaped(expr, held, landing, site_tail) {
                Some(text) => {
                    reshaped = true;
                    out.push_str(&text);
                }
                None => out.push_str(&placed_block(self.source, held, landing)),
            }
            cursor = held.end();
        }
        if !reshaped {
            return None;
        }
        out.push_str(self.source.slice(TextRange::new(cursor, param.end())));
        Some(out)
    }
}

/// The parameter closing `params`, the one no comma follows in the
/// one-per-line form, `None` where a `/` marker closes the list instead.
pub(super) fn closing_parameter(params: &Parameters) -> Option<TextRange> {
    params
        .kwarg
        .as_deref()
        .map(Ranged::range)
        .or_else(|| params.kwonlyargs.last().map(Ranged::range))
        .or_else(|| params.vararg.as_deref().map(Ranged::range))
        .or_else(|| params.args.last().map(Ranged::range))
}

/// Every parameter of `params` rendered in source order through
/// `render`, with `/` and bare `*` seated at their canonical positions,
/// `None` where any parameter renders to `None`.
pub(super) fn rendered_parts<'p>(
    params: &'p Parameters,
    mut render: impl FnMut(AnyParameterRef<'p>) -> Option<Cow<'p, str>>,
) -> Option<Vec<Cow<'p, str>>> {
    let mut parts = Vec::new();
    for param in params.posonlyargs.iter().map(AnyParameterRef::NonVariadic) {
        parts.push(render(param)?);
    }
    if !params.posonlyargs.is_empty() {
        parts.push(Cow::Borrowed("/"));
    }
    for param in params.args.iter().map(AnyParameterRef::NonVariadic) {
        parts.push(render(param)?);
    }
    if let Some(va) = params.vararg.as_deref() {
        parts.push(render(AnyParameterRef::Variadic(va))?);
    } else if !params.kwonlyargs.is_empty() {
        parts.push(Cow::Borrowed("*"));
    }
    for param in params.kwonlyargs.iter().map(AnyParameterRef::NonVariadic) {
        parts.push(render(param)?);
    }
    if let Some(kw) = params.kwarg.as_deref() {
        parts.push(render(AnyParameterRef::Variadic(kw))?);
    }
    Some(parts)
}
