//! Serializes an expression tree onto one row.

use std::borrow::Cow;

use ruff_python_ast::{
    AnyNodeRef, Comprehension, Expr, ExprDict, helpers::any_over_expr,
    visitor::Visitor as AstVisitor,
};
use ruff_text_size::TextRange;

use super::{Column, Settings, walk::Joiner};
use crate::{
    primitives::{edit::apply_inline_edits, fracture::outermost, inline::folded_line_form},
    source::Source,
};

/// Serializes an expression tree onto one row, each method writing into
/// the caller's buffer and answering `None` where its subtree reaches no
/// one-row form.
pub(super) struct Writer<'a> {
    pub(super) settings: Settings<'a>,
    pub(super) source: &'a Source,
}

impl<'a> Writer<'a> {
    /// True where no one-row form is written for `expr` over `range`,
    /// covering its own flush column under `Column::Holds`, a comment
    /// falling inside the range, and a later rule reopening it.
    fn blocked(&self, expr: &Expr, range: TextRange, hold: Column) -> bool {
        (hold == Column::Holds && self.settings.holds_its_column(self.source, expr))
            || self.source.intersects_comment(range)
            || self.reopens(expr)
    }

    /// The one-row form of a leaf, meaning an expression the dispatch in
    /// [`Self::write`] does not itself rebuild. An operator tree over
    /// atoms soft-wrapped across rows folds its break, and every other
    /// leaf reaches one row by closing each bracketed construct beneath
    /// it, whether that is a call's argument list or a literal the
    /// author fractured. A break no close reaches leaves `None`.
    fn leaf_form(&self, expr: &Expr, range: TextRange) -> Option<Cow<'a, str>> {
        if let Some(folded) = folded_line_form(self.source, expr, self.source.slice(range)) {
            return Some(folded);
        }
        let mut joiner = Joiner {
            edits: Vec::new(),
            reachable: true,
            writer: self,
        };
        joiner.visit_expr(expr);
        if !joiner.reachable {
            return None;
        }
        let joined = apply_inline_edits(self.source, range, &outermost(joiner.edits));
        (!joined.contains('\n')).then_some(joined)
    }

    /// `expr` rewritten from its children onto one row, `None` where
    /// the rewrite reaches no single row.
    fn rebuilt(&self, expr: &Expr) -> Option<Cow<'a, str>> {
        let mut out = String::new();
        self.write(&mut out, expr, expr.into())?;
        (!out.contains('\n')).then_some(Cow::Owned(out))
    }

    /// True where a later rule reopens `expr` whatever its current
    /// shape. A dict past `max_dict_entries` explodes on its own count
    /// trigger, and so does an argument list past `max_args` that
    /// `reflow-calls` can name, so no one-row form written around either
    /// survives the pipeline. A call the count trigger claims but cannot
    /// rewrite into keyword form stays inline, leaving its one-row form
    /// standing.
    fn reopens(&self, expr: &Expr) -> bool {
        any_over_expr(expr, |e| {
            e.as_call_expr()
                .is_some_and(|call| self.settings.rejoin.explodes(self.source, call))
                || e.as_dict_expr().is_some_and(|dict| {
                    self.settings
                        .max_dict_entries
                        .is_some_and(|cap| dict.len() > cap)
                })
        })
    }

    /// Appends `expr`'s one-row serialization to `out`, dispatching on
    /// its shape and descending through [`Self::write_child`]. `parent`
    /// is the immediate enclosing AST node, read for paren recovery on
    /// non-collection leaves.
    fn write(&self, out: &mut String, expr: &Expr, parent: AnyNodeRef) -> Option<()> {
        let here = AnyNodeRef::from(expr);
        match expr {
            Expr::Dict(d) => self.write_dict(out, d, here),
            Expr::DictComp(c) => self.write_comprehension(
                out,
                Some(('{', '}')),
                c.key.as_deref(),
                &c.value,
                &c.generators,
                here,
            ),
            Expr::Generator(c) => {
                let brackets = c.parenthesized.then_some(('(', ')'));
                self.write_comprehension(out, brackets, None, &c.elt, &c.generators, here)
            }
            Expr::List(l) => self.write_seq(out, Some(('[', ']')), &l.elts, here, false),
            Expr::ListComp(c) => {
                self.write_comprehension(out, Some(('[', ']')), None, &c.elt, &c.generators, here)
            }
            Expr::Set(s) => self.write_seq(out, Some(('{', '}')), &s.elts, here, false),
            Expr::SetComp(c) => {
                self.write_comprehension(out, Some(('{', '}')), None, &c.elt, &c.generators, here)
            }
            Expr::Subscript(s) => {
                self.write_child(out, &s.value, here)?;
                out.push('[');
                self.write_child(out, &s.slice, here)?;
                out.push(']');
                Some(())
            }
            Expr::Tuple(t) => {
                let brackets = t.parenthesized.then_some(('(', ')'));
                self.write_seq(out, brackets, &t.elts, here, t.len() == 1)
            }
            _ => {
                let range = self.source.paren_aware_range(expr.into(), parent);
                out.push_str(&self.leaf_form(expr, range)?);
                Some(())
            }
        }
    }

    /// Appends a child `expr`'s one-row serialization to `out`, a held
    /// flush column leaving the enclosing form unreachable.
    fn write_child(&self, out: &mut String, expr: &Expr, parent: AnyNodeRef) -> Option<()> {
        if self.settings.holds_its_column(self.source, expr) {
            return None;
        }
        self.write(out, expr, parent)
    }

    /// Appends a comprehension's bracketed one-row form to `out`: an
    /// optional `key: ` head, the element, then the clause chain,
    /// wrapped in `brackets`. A `None` bracket carries the bare
    /// generator whose call parens stand in, a `Some` key the dict
    /// comprehension's head.
    fn write_comprehension(
        &self,
        out: &mut String,
        brackets: Option<(char, char)>,
        key: Option<&Expr>,
        element: &Expr,
        generators: &[Comprehension],
        parent: AnyNodeRef,
    ) -> Option<()> {
        let (open, close) = brackets.unzip();
        out.extend(open);
        if let Some(key) = key {
            self.write_child(out, key, parent)?;
            out.push_str(": ");
        }
        self.write_child(out, element, parent)?;
        for generator in generators {
            out.push_str(if generator.is_async {
                " async for "
            } else {
                " for "
            });
            self.write_child(out, &generator.target, parent)?;
            out.push_str(" in ");
            self.write_child(out, &generator.iter, parent)?;
            for condition in &generator.ifs {
                out.push_str(" if ");
                self.write_child(out, condition, parent)?;
            }
        }
        out.extend(close);
        Some(())
    }

    /// Writes `d`'s one-row form into `out` as `{k: v, ...}`, emitting
    /// `**v` for a `None`-keyed unpacking item. A key always joins
    /// whereas a value may hold its column. `parent` is the dict itself,
    /// threaded into each child for paren recovery on non-collection
    /// leaves.
    fn write_dict(&self, out: &mut String, d: &ExprDict, parent: AnyNodeRef) -> Option<()> {
        out.push('{');
        for (i, item) in d.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            match &item.key {
                Some(key) => {
                    self.write(out, key, parent)?;
                    out.push_str(": ");
                }
                None => out.push_str("**"),
            }
            self.write_child(out, &item.value, parent)?;
        }
        out.push('}');
        Some(())
    }

    /// Writes `elts` joined by `", "` into `out`, optionally wrapped in
    /// a bracket pair and optionally followed by a trailing comma. The
    /// trailing comma carries the 1-tuple `(x,)` case.
    fn write_seq(
        &self,
        out: &mut String,
        brackets: Option<(char, char)>,
        elts: &[Expr],
        parent: AnyNodeRef,
        trailing_comma: bool,
    ) -> Option<()> {
        let (open, close) = brackets.unzip();
        out.extend(open);
        for (i, e) in elts.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            self.write_child(out, e, parent)?;
        }
        if trailing_comma {
            out.push(',');
        }
        out.extend(close);
        Some(())
    }

    /// `expr` rebuilt from its children at the canonical spacing over
    /// `range`, whatever the source wrote inside it. `None` where a
    /// guard blocks the form or the rebuild reaches no single row.
    pub(super) fn condensed(
        &self,
        expr: &Expr,
        range: TextRange,
        hold: Column,
    ) -> Option<Cow<'a, str>> {
        if self.blocked(expr, range, hold) {
            return None;
        }
        self.rebuilt(expr)
    }

    /// `expr`'s one-row form over `range`, borrowing the source slice
    /// where that range is already written flat and rebuilding it from
    /// the children otherwise. `hold` reaches `expr` itself, every child
    /// beneath it holding its own flush column either way.
    pub(super) fn formed(
        &self,
        expr: &Expr,
        range: TextRange,
        hold: Column,
    ) -> Option<Cow<'a, str>> {
        if self.blocked(expr, range, hold) {
            return None;
        }
        let slice = self.source.slice(range);
        if !slice.contains('\n') {
            return Some(Cow::Borrowed(slice));
        }
        self.rebuilt(expr)
    }

    /// Appends one top-level call argument to `out`, its grouping parens
    /// recovered only where its own text spans rows.
    pub(super) fn write_argument(
        &self,
        out: &mut String,
        expr: &Expr,
        parent: AnyNodeRef,
    ) -> Option<()> {
        let range = self.source.spanning_paren_range(expr.into(), parent);
        out.push_str(&self.formed(expr, range, Column::Holds)?);
        Some(())
    }
}
