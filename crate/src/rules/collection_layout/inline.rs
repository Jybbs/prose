//! The one-line serializer behind every collection-layout rejoin.
//! Renders an expression and its children onto a single line, a held
//! column passing through at its source shape so the fit check
//! declines the join.

use std::borrow::Cow;

use ruff_python_ast::{AnyNodeRef, Comprehension, Expr, ExprDict};
use ruff_text_size::Ranged;
use unicode_width::UnicodeWidthStr;

use super::{classify::is_column_shaped, layouter::Layouter};
use crate::primitives::{inline::single_line_form, layout::is_layoutable};

impl<'a> Layouter<'a> {
    /// Builds the inline form of `expr`, recursively inlining any nested
    /// collection or subscript that is not itself a held column. Leaves
    /// and held columns pass through as their source slice, explicit
    /// parens recovered against `parent` so a precedence-bearing
    /// `(-a) ** 2` survives the collapse.
    fn inline_form(&self, expr: &Expr) -> String {
        let mut buf = String::new();
        self.write_inline(&mut buf, expr, AnyNodeRef::from(expr));
        buf
    }

    /// Appends a child `expr`'s inline serialization to `buf`, a held
    /// multi-line literal passing through at its source shape so the
    /// enclosing rejoin fails its line-break check.
    fn write_child(&self, buf: &mut String, expr: &Expr, parent: AnyNodeRef) {
        if self.keep_multiline_literals
            && is_layoutable(expr)
            && is_column_shaped(self.source.slice(expr.range()))
        {
            buf.push_str(self.slice_with_parens(expr, parent));
            return;
        }
        self.write_inline(buf, expr, parent);
    }

    /// Appends a comprehension's `for`/`if` clause chain to `buf`, each
    /// clause a space from the preceding text and an async generator
    /// carrying its `async` keyword. Targets, iterables, and conditions
    /// route through `write_child`.
    fn write_comprehension_clauses(
        &self,
        buf: &mut String,
        generators: &[Comprehension],
        parent: AnyNodeRef,
    ) {
        for generator in generators {
            buf.push_str(if generator.is_async {
                " async for "
            } else {
                " for "
            });
            self.write_child(buf, &generator.target, parent);
            buf.push_str(" in ");
            self.write_child(buf, &generator.iter, parent);
            for condition in &generator.ifs {
                buf.push_str(" if ");
                self.write_child(buf, condition, parent);
            }
        }
    }

    /// Appends `expr`'s one-line serialization to `buf`, dispatching on
    /// its shape and descending through `write_child`. `parent` is the
    /// immediate enclosing AST node, used for `paren_aware_range`
    /// recovery on non-collection leaves.
    fn write_inline(&self, buf: &mut String, expr: &Expr, parent: AnyNodeRef) {
        let here = AnyNodeRef::from(expr);
        match expr {
            Expr::Dict(d) => self.write_inline_dict(buf, d, here),
            Expr::DictComp(c) => {
                let brackets = Some(('{', '}'));
                self.write_inline_comprehension(
                    buf,
                    brackets,
                    c.key.as_deref(),
                    &c.value,
                    &c.generators,
                    here,
                );
            }
            Expr::Generator(c) => {
                let brackets = c.parenthesized.then_some(('(', ')'));
                self.write_inline_comprehension(buf, brackets, None, &c.elt, &c.generators, here);
            }
            Expr::List(l) => self.write_inline_seq(buf, Some(('[', ']')), &l.elts, here, false),
            Expr::ListComp(c) => {
                let brackets = Some(('[', ']'));
                self.write_inline_comprehension(buf, brackets, None, &c.elt, &c.generators, here);
            }
            Expr::Set(s) => self.write_inline_seq(buf, Some(('{', '}')), &s.elts, here, false),
            Expr::SetComp(c) => {
                let brackets = Some(('{', '}'));
                self.write_inline_comprehension(buf, brackets, None, &c.elt, &c.generators, here);
            }
            Expr::Subscript(s) => {
                self.write_child(buf, &s.value, here);
                buf.push('[');
                self.write_child(buf, &s.slice, here);
                buf.push(']');
            }
            Expr::Tuple(t) => {
                let brackets = t.parenthesized.then_some(('(', ')'));
                self.write_inline_seq(buf, brackets, &t.elts, here, t.len() == 1);
            }
            _ => {
                let slice = self.slice_with_parens(expr, parent);
                // An operator tree over atoms soft-wrapped across lines
                // rejoins by collapsing its break, where any other leaf
                // passes through with its source breaks intact for the
                // fit guard to reject.
                buf.push_str(&single_line_form(expr, slice).unwrap_or(Cow::Borrowed(slice)));
            }
        }
    }

    /// Appends a comprehension's bracketed inline form to `buf`: an
    /// optional `key: ` head, the element, then the clause chain, wrapped
    /// in `brackets`. A `None` bracket carries the bare generator whose
    /// call parens stand in, a `Some` key the dict comprehension's head.
    fn write_inline_comprehension(
        &self,
        buf: &mut String,
        brackets: Option<(char, char)>,
        key: Option<&Expr>,
        element: &Expr,
        generators: &[Comprehension],
        parent: AnyNodeRef,
    ) {
        let (open, close) = brackets.unzip();
        buf.extend(open);
        if let Some(key) = key {
            self.write_child(buf, key, parent);
            buf.push_str(": ");
        }
        self.write_child(buf, element, parent);
        self.write_comprehension_clauses(buf, generators, parent);
        buf.extend(close);
    }

    /// Writes `d`'s inline serialization into `buf` as `{k: v, ...}`,
    /// emitting `**v` for `None`-keyed unpacking items. `parent` is
    /// the dict itself, threaded into each child's `write_child` for
    /// paren recovery on non-collection leaves.
    fn write_inline_dict(&self, buf: &mut String, d: &ExprDict, parent: AnyNodeRef) {
        buf.push('{');
        for (i, item) in d.iter().enumerate() {
            if i > 0 {
                buf.push_str(", ");
            }
            match &item.key {
                Some(key) => {
                    self.write_child(buf, key, parent);
                    buf.push_str(": ");
                }
                None => buf.push_str("**"),
            }
            self.write_child(buf, &item.value, parent);
        }
        buf.push('}');
    }

    /// Writes `elts` joined by `", "` into `buf`, optionally wrapped
    /// in a bracket pair and optionally followed by a trailing comma.
    /// The trailing comma carries the 1-tuple `(x,)` case.
    fn write_inline_seq(
        &self,
        buf: &mut String,
        brackets: Option<(char, char)>,
        elts: &[Expr],
        parent: AnyNodeRef,
        trailing_comma: bool,
    ) {
        let (open, close) = brackets.unzip();
        buf.extend(open);
        for (i, e) in elts.iter().enumerate() {
            if i > 0 {
                buf.push_str(", ");
            }
            self.write_child(buf, e, parent);
        }
        if trailing_comma {
            buf.push(',');
        }
        buf.extend(close);
    }

    /// `expr`'s inline form when it joins without a residual line break
    /// and fits the budget from `column`, else `None`. A held column and
    /// a leaf the inline serializer cannot itself join each keep their
    /// break, so the enclosing construct falls to the expand path.
    pub(super) fn joined_if_fits(&self, expr: &Expr, column: usize) -> Option<String> {
        let inline = self.inline_form(expr);
        (!inline.contains('\n') && column + inline.width() <= self.code_line_length)
            .then_some(inline)
    }

    /// Returns the source slice covering `expr`, with explicit parens
    /// recovered against `parent` so precedence-bearing parens like
    /// `(-a) ** 2` survive a borrow.
    pub(super) fn slice_with_parens(&self, expr: &Expr, parent: AnyNodeRef) -> &'a str {
        let range = self.source.paren_aware_range(expr.into(), parent);
        self.source.slice(range)
    }
}
