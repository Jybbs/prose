//! The text an argument list is replaced by, either exploded one
//! argument per line or rejoined onto one row, with a nested call
//! reshaped and a value the source wrote across rows re-indented to the
//! keyword column in the same pass.

use std::borrow::Cow;

use ruff_python_ast::{ArgOrKeyword, Arguments, Expr, ExprCall, visitor::Visitor as AstVisitor};
use ruff_text_size::{Ranged, TextRange, TextSize};
use unicode_width::UnicodeWidthStr;

use super::Exploder;
use crate::primitives::{
    call_keywords::{CallKeywords, keyword_args, resolve_call_params},
    edit::apply_inline_edits,
    inline::end_column,
    layout::{Separator, explode_parens, is_fractured, item_indent},
    tokens::is_opener,
    travel::{Landing, Travel, block_shift, shifted_block, spans_a_string_part},
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
        let item_indent = item_indent(indent);
        explode_parens(
            self.source.newline_str(),
            indent,
            count,
            |out, i| render(out, i, item_indent),
            Separator::comma(self.source.trailing_comma(arguments.range()).is_some()),
        )
    }

    /// Renders each of `keywords`'s arguments as `name=value` one per
    /// line at `indent`, re-exploding a nested call and re-indenting a
    /// row-spanning value through [`Self::render_value`].
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
                self.render_value(out, arg.value, &arg.rendered, arg.start, item_indent);
            },
        )
    }

    /// Renders `call`'s arguments verbatim in source order, one per line
    /// at `indent`, the fallback for a call that cannot take keyword
    /// form. A nested call or row-spanning value still resolves through
    /// [`Self::render_value`]. An argument whose own text spans rows
    /// carries the grouping pair recovered against the list, the pair
    /// holding those rows together, which the join path recovers the
    /// same way, covered with the argument's own range so a keyword
    /// keeps its `name=` head.
    fn explode_source_order(&self, call: &'a ExprCall, indent: usize) -> String {
        let args: Vec<ArgOrKeyword> = call.arguments.iter_source_order().collect();
        self.explode_items(
            &call.arguments,
            indent,
            args.len(),
            |out, i, item_indent| {
                let value = args[i].value();
                let range = if self.source.contains_line_break(value.range()) {
                    args[i].range().cover(
                        self.source
                            .paren_aware_range(value.into(), (&call.arguments).into()),
                    )
                } else {
                    args[i].range()
                };
                self.render_value(
                    out,
                    value,
                    self.source.slice(range),
                    range.start(),
                    item_indent,
                );
            },
        )
    }

    /// The columns trailing this call on its own physical row, which a
    /// joined or exploded row lands beside. A walk inside a relocated
    /// value carries its own region rather than the source row, whose
    /// tail belongs to the text the outer walk assembles. A tail holding
    /// a bracket of its own is charged only through that bracket, since
    /// exploding the construct it opens ends the row there and leaves
    /// the rest of the tail on rows of its own.
    fn row_tail(&self, end: TextSize) -> usize {
        if self.indent.is_some() {
            return 0;
        }
        match self.first_opener(self.source.row_tail(end)) {
            Some(offset) => self.source.width_between(end, offset + TextSize::from(1)),
            None => self.source.row_tail_width(end),
        }
    }

    /// The offset of the first opening bracket inside `range`, the token
    /// that marks a construct whose own layout has yet to settle.
    fn first_opener(&self, range: TextRange) -> Option<TextSize> {
        self.source
            .tokens_overlapping(range)
            .find(|token| range.contains(token.start()) && is_opener(token.kind()))
            .map(Ranged::start)
    }

    /// The move `rendered`, the text of the argument opening at `start`
    /// with head `head`, makes over its continuation rows when the
    /// argument lands at `indent`, read through [`block_shift`]. `None`
    /// where the argument holds no continuation row, or where a
    /// row-spanning string part inside `value` holds the whole argument,
    /// whose interior a move would pad, leaving no row frozen for the
    /// shift to skip.
    fn argument_shift(
        &self,
        value: &Expr,
        rendered: &str,
        head: &str,
        start: TextSize,
        indent: usize,
    ) -> Option<Travel> {
        if spans_a_string_part(self.source, value) {
            return None;
        }
        let landing = Landing {
            column: end_column(head, indent),
            indent,
            item: start,
        };
        block_shift(self.source, rendered, &[], value.start(), landing)
    }

    /// Appends `rendered`, the text of the argument opening at `start`,
    /// to `out`, its nested calls reshaped and, where the argument
    /// travels, its continuation rows moved by whatever
    /// [`Self::argument_shift`] reads. A grouping pair around the value
    /// stays outside the reshape and moves with the rest of the
    /// argument, whether the source carries it or `keyword_args` adds it.
    fn render_value(
        &self,
        out: &mut String,
        value: &'a Expr,
        rendered: &str,
        start: TextSize,
        indent: usize,
    ) {
        let slice = self.source.slice(value.range());
        let (head, tail) = rendered
            .rsplit_once(slice)
            .expect("a rendered argument carries its value's source text");
        let Some(travel) = self.argument_shift(value, rendered, head, start, indent) else {
            let column = end_column(head, indent).saturating_add_signed(self.line_shift);
            out.push_str(head);
            out.push_str(&self.reshape_value(value, Some(indent), column, self.line_shift));
            out.push_str(tail);
            return;
        };
        let shift = self.line_shift + travel.rows;
        // The value opens on the argument's own row while the head holds
        // no break, and on a row the move carries otherwise.
        let opening_shift = if head.contains('\n') {
            shift
        } else {
            self.line_shift
        };
        let column = end_column(head, indent).saturating_add_signed(opening_shift);
        // The nested walk writes its rows where the source wrote them,
        // so the move below carries its exploded closer to `indent`.
        let landing = indent.saturating_add_signed(-travel.rows);
        let reshaped = self.reshape_value(value, Some(landing), column, shift);
        out.push_str(&shifted_block(&format!("{head}{reshaped}{tail}"), travel));
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
    /// length trigger measured from `column`, where the `(` lands. The
    /// length trigger asks `primitives::one_row` whether the list
    /// reaches one row at all and whether that row fits, so a list
    /// holding an argument no join closes explodes whatever its first
    /// row measures. A keyword-expressible call renders one keyword per
    /// line, while any other call renders positionally under the length
    /// trigger. A nested call in an argument value explodes in the same
    /// text. Where no trigger fires, a fractured list rejoins onto one
    /// line through that same one-row form and every other call is left
    /// inline.
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
        let count_trips = self.one_row.count_explodes(self.source, call);
        let tail = self.row_tail(arguments.range().end());
        let form = self
            .one_row
            .arguments_form(self.source, arguments)
            .filter(|form| self.one_row.fits(column + form.width() + tail));
        let length_trips = form.is_none();
        // The filter above already fit the joined row, leaving only the
        // fracture question.
        if !count_trips && let Some(form) = form {
            return is_fractured(self.source, arguments.range()).then_some(form);
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
