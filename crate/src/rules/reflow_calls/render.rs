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
    inline::{end_column, opening_width},
    layout::{Separator, explode_parens, is_fractured, item_indent},
    tokens::is_opener,
    travel::{Landing, Travel, block_shift, shifted_block, spans_a_string_part},
};

/// Where an exploded argument's value lands: the column a later
/// alignment settles it at where one does, the indent of the row it
/// opens on, and the columns the row carries after it.
#[derive(Clone, Copy)]
struct Slot {
    aligned: Option<usize>,
    indent: usize,
    tail: usize,
}

impl<'a> Exploder<'a> {
    /// Renders `count` arguments one per line at `indent` through
    /// `render`, closing each row under the trailing-comma policy over
    /// `arguments`. `render` receives the row index, the item indent,
    /// and the separator width charged against the row, one column for
    /// every row a comma closes and, where `moves` forecasts a later
    /// sort, for the row the sort leaves anywhere but last.
    fn explode_items(
        &self,
        arguments: &Arguments,
        indent: usize,
        count: usize,
        moves: impl Fn(usize) -> Option<bool>,
        render: impl Fn(&mut String, usize, usize, usize),
    ) -> String {
        let item_indent = item_indent(indent);
        let trailing = self.source.trailing_comma(arguments.range()).is_some();
        explode_parens(
            self.source.newline_str(),
            indent,
            count,
            |out, i| {
                let closes = moves(i).unwrap_or(i + 1 < count);
                render(out, i, item_indent, usize::from(trailing || closes));
            },
            Separator::comma(trailing),
        )
    }

    /// Renders each of `keywords`'s arguments as `name=value` one per
    /// line at `indent`, re-exploding a nested call and re-indenting a
    /// row-spanning value through [`Self::render_value`]. Every keyword
    /// lands alone on its row and so takes the buffer `align-equals`
    /// seats around its `=`, leaving each value measured from the column
    /// that buffer settles it at rather than the one `name=` writes it at.
    fn explode_keywords(
        &self,
        keywords: &CallKeywords<'a>,
        arguments: &Arguments,
        indent: usize,
    ) -> String {
        let last = self
            .reorders
            .sorted_last_keyword(keywords.args.iter().map(|arg| (arg.name, arg.value)));
        self.explode_items(
            arguments,
            indent,
            keywords.args.len(),
            |i| last.map(|last| last != i),
            |out, i, item_indent, tail| {
                let arg = &keywords.args[i];
                let slot = self.slot(Some(arg.name), item_indent, tail);
                self.render_value(out, arg.value, &arg.rendered, arg.start, slot);
            },
        )
    }

    /// Renders `call`'s arguments verbatim in source order, one per line
    /// at `indent`, the fallback for a call that cannot take keyword
    /// form. A nested call or row-spanning value still resolves through
    /// [`Self::render_value`], and a keyword's value measures from the
    /// column the `align-equals` buffer settles it at the way the
    /// keyword form does. An argument whose own text spans rows carries
    /// the grouping pair recovered against the list, the pair holding
    /// those rows together, which the join path recovers the same way,
    /// covered with the argument's own range so a keyword keeps its
    /// `name=` head.
    fn explode_source_order(&self, call: &'a ExprCall, indent: usize) -> String {
        let args: Vec<ArgOrKeyword> = call.arguments.iter_source_order().collect();
        let last = self
            .reorders
            .sorted_last(self.source, (&call.arguments).into(), call.into());
        self.explode_items(
            &call.arguments,
            indent,
            args.len(),
            |i| last.map(|last| !last.contains_range(args[i].range())),
            |out, i, item_indent, tail| {
                let value = args[i].value();
                let range = if self.source.contains_line_break(value.range()) {
                    args[i].range().cover(
                        self.source
                            .paren_aware_range(value.into(), (&call.arguments).into()),
                    )
                } else {
                    args[i].range()
                };
                let name = args[i]
                    .as_keyword()
                    .and_then(|keyword| keyword.arg.as_deref());
                let slot = self.slot(name, item_indent, tail);
                self.render_value(out, value, self.source.slice(range), range.start(), slot);
            },
        )
    }

    /// The columns trailing this call on its row: the text to the end
    /// of the physical row, or to the region's end plus the columns the
    /// enclosing text writes there where the region closes first. A
    /// tail holding a bracket of its own is charged only through that
    /// bracket, since exploding the construct it opens ends the row
    /// there.
    fn row_tail(&self, end: TextSize) -> usize {
        let row_end = self.source.row_tail(end).end();
        let clipped = self.region.end() <= row_end;
        let tail = TextRange::new(end, row_end.min(self.region.end()));
        if let Some(offset) = self.first_opener(tail) {
            return self.source.width_between(end, offset + TextSize::from(1));
        }
        if clipped {
            self.source.slice(tail).trim_end().width() + self.tail
        } else {
            self.source.row_tail_width(end)
        }
    }

    /// The width `arguments` leaves on its row, which is `form` for a
    /// list written across rows, since closing it writes that text, and
    /// the source slice for one already on a single row, whose spacing
    /// the rule leaves as the author wrote it rather than at the
    /// normalized gap `form` seats after each comma.
    fn written_width(&self, arguments: &Arguments, form: &str) -> usize {
        if self.source.contains_line_break(arguments.range()) {
            form.width()
        } else {
            self.source.slice(arguments.range()).width()
        }
    }

    /// The slot an argument lands in at `indent` with `tail` columns
    /// closing its row, a keyword `name` measuring its value from the
    /// column the `align-equals` buffer settles it at.
    fn slot(&self, name: Option<&str>, indent: usize, tail: usize) -> Slot {
        Slot {
            aligned: name
                .and_then(|name| self.reservations.keyword_value_column(indent, name.width())),
            indent,
            tail,
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
    /// `slot.aligned` is the column a later alignment run settles the
    /// value at, which the nested reshape measures from, and `None`
    /// leaves the measure at the column `rendered` writes. A value
    /// wrapped in a grouping pair opens one column past that gap, and
    /// the pair's closer joins `slot.tail` as the columns following the
    /// value on its last row.
    fn render_value(
        &self,
        out: &mut String,
        value: &'a Expr,
        rendered: &str,
        start: TextSize,
        slot: Slot,
    ) {
        let slice = self.source.slice(value.range());
        let (head, tail) = rendered
            .rsplit_once(slice)
            .expect("a rendered argument carries its value's source text");
        let settled = slot.aligned.map_or_else(
            || end_column(head, slot.indent),
            |column| column + usize::from(head.ends_with('(')),
        );
        // Only the tail's opening row closes the value's last row, a pair
        // closing on a later row taking the row's comma with it.
        let appended = if tail.contains('\n') {
            opening_width(tail)
        } else {
            tail.width() + slot.tail
        };
        let Some(travel) = self.argument_shift(value, rendered, head, start, slot.indent) else {
            let column = settled.saturating_add_signed(self.line_shift);
            out.push_str(head);
            out.push_str(&self.reshape_value(
                value,
                Some(slot.indent),
                column,
                self.line_shift,
                appended,
            ));
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
        let column = settled.saturating_add_signed(opening_shift);
        // The nested walk writes its rows where the source wrote them,
        // so the move below carries its exploded closer to `indent`.
        let landing = slot.indent.saturating_add_signed(-travel.rows);
        let reshaped = self.reshape_value(value, Some(landing), column, shift, appended);
        out.push_str(&shifted_block(&format!("{head}{reshaped}{tail}"), travel));
    }

    /// `value`'s text with every call inside it exploded, its opening
    /// line placed at `column`, every later line moving by `line_shift`,
    /// `tail` columns following its last row, and an exploded closing
    /// `)` dropping to `indent` or to its own source line where `indent`
    /// is `None`. Borrowed where none reshapes.
    fn reshape_value(
        &self,
        value: &'a Expr,
        indent: Option<usize>,
        column: usize,
        line_shift: isize,
        tail: usize,
    ) -> Cow<'a, str> {
        let mut nested = Exploder {
            edits: Vec::new(),
            indent,
            line_shift,
            origin_column: column,
            region: value.range(),
            tail,
            ..*self
        };
        nested.visit_expr(value);
        apply_inline_edits(self.source, value.range(), &nested.edits)
    }

    /// Returns the exploded `(...)` text for `call` when the count or
    /// length trigger fires, the closing `)` landing at the indent
    /// [`Self::indent_for`] reads and the length trigger measured from
    /// `column`, where the `(` lands. The length trigger asks
    /// `primitives::one_row` whether the list reaches one row at all and
    /// whether that row fits, so a list holding an argument no join
    /// closes explodes whatever its first row measures. A
    /// keyword-expressible call renders one keyword per line, while any
    /// other call renders positionally under the length trigger. A
    /// nested call in an argument value explodes in the same text. Where
    /// no trigger fires, a fractured list rejoins onto one line through
    /// that same one-row form and every other call is left inline.
    pub(super) fn explode_args(&self, call: &'a ExprCall, column: usize) -> Option<String> {
        let arguments = &call.arguments;
        if arguments.is_empty() || self.source.intersects_comment(arguments.inner_range()) {
            return None;
        }
        let count_trips = self.one_row.count_explodes(self.source, call);
        let tail = self.row_tail(arguments.range().end());
        let form = self
            .one_row
            .arguments_form(self.source, arguments)
            .filter(|form| {
                self.one_row
                    .fits(column + self.written_width(arguments, form) + tail)
            });
        let length_trips = form.is_none();
        // The filter above already fit the joined row, leaving only the
        // fracture question.
        if !count_trips && let Some(form) = form {
            return is_fractured(self.source, arguments.range()).then_some(form);
        }
        match keyword_args(self.source, call, resolve_call_params(call, self.targets)) {
            Some(keywords) if !keywords.has_posonly_prefix => {
                Some(self.explode_keywords(&keywords, arguments, self.indent_for(call)))
            }
            // A call that cannot take keyword form explodes positionally,
            // but only on the length trigger, so the count trigger keeps
            // leaving such calls inline.
            _ => length_trips.then(|| self.explode_source_order(call, self.indent_for(call))),
        }
    }
}
