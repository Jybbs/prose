//! Explodes a call to one argument per line under two triggers. The
//! count trigger fires on a keyword-expressible call carrying more than
//! `max_args` arguments, rendering one keyword per line. The length
//! trigger fires on any call whose inline argument list crosses
//! `code_line_length` from the column that list lands at, exploding a
//! keyword-expressible call in keyword form and any other call
//! positionally. The closing `)` drops to the indent of the row carrying
//! the call, a nested call in an argument value explodes in the same
//! pass, and the walk continues through the callee so a chained receiver
//! settles there too. Order, `=` alignment, and trailing commas stay
//! with `alphabetize`, `align_equals`, and `strip_trailing_commas`.

use std::{borrow::Cow, collections::HashMap};

use itertools::Itertools;
use ruff_diagnostics::Edit;
use ruff_python_ast::{
    ArgOrKeyword, Arguments, Expr, ExprCall, Parameters, StringLike,
    helpers::any_over_expr,
    visitor::{Visitor as AstVisitor, walk_expr},
};
use ruff_text_size::{Ranged, TextRange, TextSize};
use unicode_width::UnicodeWidthStr;

use crate::{
    config::Config,
    primitives::{
        INDENT_STEP, aligner,
        call_keywords::{CallKeywords, keyword_args, module_call_params, resolve_call_params},
        edit::{apply_inline_edits, narrowed_replacement, singleton_groups},
        inline::{end_column, opening_width},
        layout::{explode_parens, is_layoutable, reindent_block, reindent_shift},
        reserve::reserved_columns,
    },
    rule::{Rule, RuleId},
    rules::align_equals::AlignEquals,
    source::Source,
};

pub(crate) struct CallLayout {
    align_equals: Option<aligner::Settings>,
    code_line_length: usize,
    max_args: Option<usize>,
}

impl CallLayout {
    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            align_equals: AlignEquals::reserve_settings(config),
            code_line_length: config.code_width(),
            max_args: config.rules.call_layout.max_args.cap(),
        }
    }
}

impl Rule for CallLayout {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let targets = module_call_params(source);
        let reservations = self.align_equals.map_or_else(HashMap::new, |settings| {
            reserved_columns(source, settings, AlignEquals::SLUG)
        });
        let mut exploder = Exploder {
            cap: self.max_args,
            code_line_length: self.code_line_length,
            edits: Vec::new(),
            indent: None,
            line_shift: 0,
            origin: TextSize::new(0),
            origin_column: 0,
            reservations: &reservations,
            source,
            targets: &targets,
        };
        exploder.visit_body(&source.ast().body);
        singleton_groups(exploder.edits)
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

/// Walks a module, or one relocated argument value, emitting the explode
/// edits its calls need. `origin` and `origin_column` place the walked
/// subtree's opening line, `line_shift` the columns every later line
/// moves by, and `indent` is the indent an exploded closing `)` drops to,
/// unset where each call answers to its own source line.
struct Exploder<'a> {
    cap: Option<usize>,
    code_line_length: usize,
    edits: Vec<Edit>,
    indent: Option<usize>,
    line_shift: isize,
    origin: TextSize,
    origin_column: usize,
    reservations: &'a HashMap<TextSize, usize>,
    source: &'a Source,
    targets: &'a HashMap<TextSize, &'a Parameters>,
}

impl<'a> Exploder<'a> {
    /// The column `offset` reaches once this walk's subtree is placed,
    /// which is the source column plus `line_shift` on every line past
    /// the opening one.
    fn column_of(&self, offset: TextSize) -> usize {
        if self.source.same_line(self.origin, offset) {
            self.origin_column
                + self
                    .source
                    .slice(TextRange::new(self.origin, offset))
                    .width()
        } else {
            self.source
                .column_of(offset)
                .saturating_add_signed(self.line_shift)
        }
    }

    /// Returns the exploded `(...)` text for `call` when the count or
    /// length trigger fires, the closing `)` landing at `indent` and the
    /// length trigger measured from `column`, where the `(` lands. A
    /// keyword-expressible call renders one keyword per line, while any
    /// other call renders positionally under the length trigger only. A
    /// nested call in an argument value explodes in the same text. `None`
    /// leaves the call inline.
    fn explode_args(&self, call: &'a ExprCall, indent: usize, column: usize) -> Option<String> {
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
        let last = count - 1;
        let trailing = self.source.trailing_comma(arguments.range()).is_some();
        explode_parens(
            self.source.newline_str(),
            indent,
            count,
            |out, i| render(out, i, item_indent),
            |i| trailing || i < last,
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

    /// The indent an exploded closing `)` drops to for `call`, this
    /// walk's own indent inside a relocated value and the call's source
    /// line indent otherwise.
    fn indent_for(&self, call: &ExprCall) -> usize {
        self.indent
            .unwrap_or_else(|| self.source.line_indent_width(call.start()))
    }

    /// `arguments` rendered on one line, joined by `", "` inside the
    /// parens. A named keyword measures at its canonical `name=value`
    /// rather than at whatever padding `align_equals` gave it.
    fn inline_args(&self, arguments: &Arguments) -> String {
        format!(
            "({})",
            arguments
                .iter_source_order()
                .map(|arg| match arg {
                    ArgOrKeyword::Arg(expr) => Cow::Borrowed(self.source.slice(expr)),
                    ArgOrKeyword::Keyword(kw) => match &kw.arg {
                        Some(name) => Cow::Owned(format!(
                            "{}={}",
                            name.as_str(),
                            self.source.slice(&kw.value),
                        )),
                        None => Cow::Borrowed(self.source.slice(kw)),
                    },
                })
                .join(", "),
        )
    }

    /// The column `call`'s `(` reaches. A call that is itself an aligned
    /// value whose callee carries no line break measures from the column
    /// `align_equals` shifts it to plus the callee's width, and every
    /// other call from this walk's own placement.
    fn open_paren_column(&self, call: &ExprCall) -> usize {
        let open = call.arguments.start();
        self.reservations
            .get(&call.start())
            .filter(|_| self.source.same_line(call.start(), open))
            .map_or_else(
                || self.column_of(open),
                |reserved| {
                    reserved
                        + self
                            .source
                            .slice(TextRange::new(call.start(), open))
                            .width()
                },
            )
    }

    /// True when `arguments` rendered inline from `column` crosses
    /// `code_line_length`. An argument that itself spans lines caps the
    /// measure at the row the join opens.
    fn overflows_line(&self, arguments: &Arguments, column: usize) -> bool {
        column + opening_width(&self.inline_args(arguments)) > self.code_line_length
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
    /// column and any other value for its nested-call reshape.
    fn render_value(&self, out: &mut String, value: &'a Expr, rendered: &str, indent: usize) {
        let slice = self.source.slice(value.range());
        let Some(head) = rendered.strip_suffix(slice) else {
            out.push_str(rendered);
            return;
        };
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
            cap: self.cap,
            code_line_length: self.code_line_length,
            edits: Vec::new(),
            indent,
            line_shift,
            origin: value.start(),
            origin_column: column,
            reservations: self.reservations,
            source: self.source,
            targets: self.targets,
        };
        nested.visit_expr(value);
        nested.edits.sort_unstable();
        apply_inline_edits(self.source, value.range(), &nested.edits)
    }
}

impl<'a> AstVisitor<'a> for Exploder<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Call(call) = expr {
            let indent = self.indent_for(call);
            let column = self.open_paren_column(call);
            if let Some(text) = self.explode_args(call, indent, column)
                && let Some(edit) = narrowed_replacement(self.source, call.arguments.range(), text)
            {
                self.edits.push(edit);
                // The edit spans the argument list, leaving the callee
                // the one part of the call the walk still reaches.
                self.visit_expr(&call.func);
                return;
            }
        }
        walk_expr(self, expr);
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use super::*;
    use crate::testing::{applied_text, parse};

    #[test]
    fn call_two_levels_inside_a_collection_value_measures_where_it_lands() {
        let src =
            "emit(alpha=1, beta=[\n    helper(aaaa, wrap(bbbbbb, cccccc)),\n], gamma=3, delta=4)\n";
        let source = parse(src);
        let config = Config {
            code_line_length: NonZeroUsize::new(30),
            ..Config::default()
        };
        let edits = CallLayout::from_config(&config)
            .apply(&source)
            .into_iter()
            .flatten()
            .collect();
        let text = applied_text(&source, edits);
        // The doubly-nested `wrap` answers the row it lands on, so it
        // explodes in this pass rather than the next one.
        assert!(
            text.contains("wrap(\n"),
            "nested call should explode in one pass:\n{text}",
        );
    }

    #[test]
    fn keyword_value_spanning_a_multiline_string_holds_the_floor() {
        let src =
            "emit(alpha=1, beta=2, gamma=3, note=[\n    \"x\",\n    \"\"\"multi\nline\"\"\",\n])\n";
        let source = parse(src);
        let edits = CallLayout::from_config(&Config::default())
            .apply(&source)
            .into_iter()
            .flatten()
            .collect();
        let text = applied_text(&source, edits);
        // The call explodes, yet the string-bearing list stays at the floor,
        // its rows unshifted so the string interior keeps its column.
        assert!(
            text.contains("    note=[\n    \"x\","),
            "string-bearing value should not re-indent:\n{text}",
        );
    }
}
