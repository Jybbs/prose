//! The text an argument list is replaced by, either exploded one
//! argument per line or rejoined onto one row, with a nested call
//! reshaped and a value the source wrote across rows re-indented to the
//! keyword column in the same pass.

use std::borrow::Cow;

use ruff_python_ast::{ArgOrKeyword, Arguments, Expr, ExprCall, visitor::Visitor as AstVisitor};
use ruff_text_size::{Ranged, TextRange};
use unicode_width::UnicodeWidthStr;

use super::Exploder;
use crate::primitives::{
    call_keywords::{CallKeywords, keyword_args, resolve_call_params},
    edit::apply_inline_edits,
    inline::{end_column, opening_width},
    layout::{
        Separator, explode_parens, hangs_from_its_row, is_fractured, item_indent,
        reindent_continuation, reindent_shift, spans_a_string_part,
    },
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
                self.render_value(out, arg.value, &arg.rendered, item_indent);
            },
        )
    }

    /// Renders `call`'s arguments verbatim in source order, one per line
    /// at `indent`, the fallback for a call that cannot take keyword
    /// form. A nested call or row-spanning value still resolves through
    /// [`Self::render_value`].
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

    /// True where `rendered`, the text of one argument, re-indents as a
    /// block to the item column. It hangs from its own row, and no
    /// string part inside `value` spans rows, whose interior the move
    /// would pad.
    fn placeable(&self, value: &Expr, rendered: &str) -> bool {
        hangs_from_its_row(self.source, value.start(), rendered)
            && !spans_a_string_part(self.source, value)
    }

    /// The one-line `(...)` text for an argument list the author
    /// fractured, or `None` where it holds no break or carries the flush
    /// column shape the explode path emits. The joined row measures from
    /// `column` across the text trailing the call to the end of its
    /// logical line, so a rejoin never lands a row the length trigger
    /// would explode again.
    fn rejoined(&self, arguments: &Arguments, column: usize, joined: String) -> Option<String> {
        let range = arguments.range();
        if !is_fractured(self.source, range) {
            return None;
        }
        let tail = self.source.logical_line_tail(range.end());
        let width = column + joined.width() + self.settled_width(tail);
        (width <= self.code_line_length).then_some(joined)
    }

    /// Appends `rendered` to `out`, its nested calls reshaped and, where
    /// the argument re-indents as a block, its continuation lines moved
    /// so the whole argument hangs from `indent`. A grouping pair around
    /// the value stays outside the reshape and moves with the rest of the
    /// argument, whether the source carries it or `keyword_args` adds it.
    fn render_value(&self, out: &mut String, value: &'a Expr, rendered: &str, indent: usize) {
        let slice = self.source.slice(value.range());
        let (head, tail) = rendered
            .rsplit_once(slice)
            .expect("a rendered argument carries its value's source text");
        if !self.placeable(value, rendered) {
            let column = end_column(head, indent).saturating_add_signed(self.line_shift);
            out.push_str(head);
            out.push_str(&self.reshape_value(value, Some(indent), column, self.line_shift));
            out.push_str(tail);
            return;
        }
        let shift = self.line_shift + reindent_shift(rendered, indent);
        // The value opens on the argument's own row while the head holds
        // no break, and on a row the re-indent moves otherwise.
        let opening_shift = if head.contains('\n') {
            shift
        } else {
            self.line_shift
        };
        let column = end_column(head, indent).saturating_add_signed(opening_shift);
        let reshaped = self.reshape_value(value, None, column, shift);
        out.push_str(&reindent_continuation(
            &format!("{head}{reshaped}{tail}"),
            indent,
        ));
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

    /// The display width `range` reaches once the fractures inside it
    /// close up, each continuation line shedding its indent and one
    /// column standing in for the separator a join restores. A range
    /// holding a break that never closes counts the lines beneath it
    /// too and so measures long, declining a rejoin whose result would
    /// otherwise move once the call around it explodes.
    fn settled_width(&self, range: TextRange) -> usize {
        let text = self.source.slice(range);
        let mut lines = text.lines();
        let head = lines.next().map_or(0, UnicodeWidthStr::width);
        head + lines
            .map(|line| 1 + line.trim_start().width())
            .sum::<usize>()
    }

    /// True where an argument still spans rows once every closable
    /// fracture inside the list shuts and that argument hangs from its
    /// own row, so the explode carries it whole. A span aligned under an
    /// interior bracket leaves the list alone.
    fn spans_movable_rows(&self, arguments: &Arguments) -> bool {
        arguments.iter_source_order().any(|arg| {
            let settled = self.rejoin.text(self.source, arg.value(), arg.range());
            hangs_from_its_row(self.source, arg.value().start(), &settled)
        })
    }

    /// Returns the exploded `(...)` text for `call` when the count,
    /// length, or span trigger fires, the closing `)` landing at `indent`
    /// and the length trigger measured from `column`, where the `(`
    /// lands. The span trigger reads the joined list, which still carries
    /// a break where an argument's own text spans rows. A
    /// keyword-expressible call renders one keyword per line, while any
    /// other call renders positionally under the length and span triggers.
    /// A nested call in an argument value explodes in the same text. Where
    /// no trigger fires, a fractured list rejoins onto one line and every
    /// other call is left inline.
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
        let count_trips = self.max_args.is_some_and(|cap| arguments.len() > cap);
        let joined = self.rejoin.joined(self.source, arguments);
        let span_trips = joined.contains('\n') && self.spans_movable_rows(arguments);
        let length_trips = column + opening_width(&joined) > self.code_line_length;
        if !count_trips && !length_trips && !span_trips {
            return self.rejoined(arguments, column, joined);
        }
        match keyword_args(self.source, call, resolve_call_params(call, self.targets)) {
            Some(keywords) if !keywords.has_posonly_prefix => {
                Some(self.explode_keywords(&keywords, arguments, indent))
            }
            // A call that cannot take keyword form explodes positionally,
            // but only on the length and span triggers, so the count
            // trigger keeps leaving such calls inline.
            _ => (length_trips || span_trips).then(|| self.explode_source_order(call, indent)),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use rstest::rstest;
    use ruff_text_size::TextSize;

    use super::*;
    use crate::{config::Config, testing::parse};

    #[rstest]
    #[case::no_break("value = f(a) + tail_name\n", 12)]
    #[case::fracture_that_closes("value = f(a) + g(b,\n               c)\n", 10)]
    #[case::held_break_measures_long("value = f(a) + [\n    b,\n]\n", 9)]
    fn settled_width_counts_the_opening_line_whole(#[case] src: &str, #[case] expected: usize) {
        let source = parse(src);
        let config = Config::default();
        let reservations = config.equals_reservations().columns(&source);
        let targets = HashMap::new();
        let exploder = Exploder {
            code_line_length: 88,
            edits: Vec::new(),
            indent: None,
            line_shift: 0,
            max_args: None,
            origin: TextSize::new(0),
            origin_column: 0,
            rejoin: config.fracture_settings(),
            reservations: &reservations,
            source: &source,
            targets: &targets,
        };
        let start = TextSize::try_from(src.find(')').expect("a closing paren") + 1)
            .expect("the offset fits");
        let tail = source.logical_line_tail(start);
        assert_eq!(exploder.settled_width(tail), expected);
    }
}
