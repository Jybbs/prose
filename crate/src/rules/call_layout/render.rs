//! The exploded text an argument list is replaced by, one argument per
//! line, with a nested call reshaped and a multi-line collection value
//! re-indented to the keyword column in the same pass.

use std::borrow::Cow;

use ruff_python_ast::{
    ArgOrKeyword, Arguments, Expr, ExprCall, StringLike, helpers::any_over_expr,
    visitor::Visitor as AstVisitor,
};
use ruff_text_size::Ranged;

use super::Exploder;
use crate::primitives::{
    INDENT_STEP,
    call_keywords::{CallKeywords, keyword_args, resolve_call_params},
    edit::apply_inline_edits,
    inline::end_column,
    layout::{explode_parens, is_layoutable, reindent_block, reindent_shift},
};

impl<'a> Exploder<'a> {
    /// Renders `count` arguments one per line at `indent` through
    /// `render`, closing each row under the trailing-comma policy over
    /// `arguments`. `render` receives the row index and the item indent.
    fn explode_items(
        &self,
        arguments: &Arguments,
        indent: usize,
        count: usize,
        render: impl Fn(&mut String, usize, usize),
    ) -> String {
        let item_indent = indent + INDENT_STEP;
        explode_parens(
            self.source.newline_str(),
            indent,
            count,
            |out, i| render(out, i, item_indent),
            self.source.trailing_comma(arguments.range()).is_some(),
        )
    }

    /// Renders each of `keywords`'s arguments as `name=value` one per
    /// line at `indent`, re-exploding a nested call and re-indenting a
    /// multi-line collection value through [`Self::render_value`].
    fn explode_keywords(
        &self,
        keywords: &CallKeywords<'a>,
        arguments: &Arguments,
        indent: usize,
    ) -> String {
        self.explode_items(
            arguments,
            indent,
            keywords.args.len(),
            |out, i, item_indent| {
                let arg = &keywords.args[i];
                self.render_value(out, arg.value, &arg.rendered, item_indent);
            },
        )
    }

    /// Renders `call`'s arguments verbatim in source order, one per line
    /// at `indent`, the fallback for a call that cannot take keyword
    /// form. A nested call or multi-line collection value still resolves
    /// through [`Self::render_value`].
    fn explode_source_order(&self, call: &'a ExprCall, indent: usize) -> String {
        let args: Vec<ArgOrKeyword> = call.arguments.iter_source_order().collect();
        self.explode_items(
            &call.arguments,
            indent,
            args.len(),
            |out, i, item_indent| {
                let rendered = self.source.slice(args[i].range());
                self.render_value(out, args[i].value(), rendered, item_indent);
            },
        )
    }

    /// True for a multi-line collection or comprehension value whose
    /// already-bracketed block re-indents to the keyword column. A value
    /// spanning a multi-line string is excluded, leaving it at the
    /// verbatim floor, since re-indenting would pad the string interior.
    fn reindentable(&self, value: &Expr) -> bool {
        (is_layoutable(value)
            || matches!(
                value,
                Expr::ListComp(_) | Expr::SetComp(_) | Expr::DictComp(_)
            ))
            && self.source.contains_line_break(value.range())
            && !any_over_expr(value, |e| {
                StringLike::try_from(e).is_ok() && self.source.contains_line_break(e.range())
            })
    }

    /// Appends `rendered` to `out`, swapping a multi-line collection or
    /// comprehension value for that block re-indented to the keyword
    /// column and any other value for its nested-call reshape. A grouping
    /// pair around the value stays outside the reshape, whether the source
    /// carries it or `keyword_args` adds it.
    fn render_value(&self, out: &mut String, value: &'a Expr, rendered: &str, indent: usize) {
        let slice = self.source.slice(value.range());
        let (head, tail) = rendered
            .rsplit_once(slice)
            .expect("a rendered argument carries its value's source text");
        out.push_str(head);
        let column = end_column(head, indent).saturating_add_signed(self.line_shift);
        if self.reindentable(value) {
            let shift = self.line_shift + reindent_shift(slice, indent);
            out.push_str(&reindent_block(
                &self.reshape_value(value, None, column, shift),
                indent,
            ));
        } else {
            out.push_str(&self.reshape_value(value, Some(indent), column, self.line_shift));
        }
        out.push_str(tail);
    }

    /// `value`'s text with every call inside it exploded, its opening
    /// line placed at `column`, every later line moving by `line_shift`,
    /// and an exploded closing `)` dropping to `indent` or to its own
    /// source line where `indent` is `None`. Borrowed where none reshapes.
    fn reshape_value(
        &self,
        value: &'a Expr,
        indent: Option<usize>,
        column: usize,
        line_shift: isize,
    ) -> Cow<'a, str> {
        let mut nested = Exploder {
            edits: Vec::new(),
            indent,
            line_shift,
            origin: value.start(),
            origin_column: column,
            ..*self
        };
        nested.visit_expr(value);
        apply_inline_edits(self.source, value.range(), &nested.edits)
    }

    /// Returns the exploded `(...)` text for `call` when the count or
    /// length trigger fires, the closing `)` landing at `indent` and the
    /// length trigger measured from `column`, where the `(` lands. A
    /// keyword-expressible call renders one keyword per line, while any
    /// other call renders positionally under the length trigger only. A
    /// nested call in an argument value explodes in the same text. `None`
    /// leaves the call inline.
    pub(super) fn explode_args(
        &self,
        call: &'a ExprCall,
        indent: usize,
        column: usize,
    ) -> Option<String> {
        let arguments = &call.arguments;
        if arguments.is_empty() || self.source.intersects_comment(arguments.inner_range()) {
            return None;
        }
        let count_trips = self.cap.is_some_and(|cap| arguments.len() > cap);
        let length_trips = self.overflows_line(arguments, column);
        if !count_trips && !length_trips {
            return None;
        }
        match keyword_args(self.source, call, resolve_call_params(call, self.targets)) {
            Some(keywords) if !keywords.has_posonly_prefix => {
                Some(self.explode_keywords(&keywords, arguments, indent))
            }
            // A call that cannot take keyword form explodes positionally,
            // but only on the length trigger, so the count trigger keeps
            // leaving such calls inline.
            _ => length_trips.then(|| self.explode_source_order(call, indent)),
        }
    }
}
