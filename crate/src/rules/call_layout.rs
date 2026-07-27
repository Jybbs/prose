//! Explodes a call to one argument per line under two triggers. The
//! count trigger fires on a keyword-expressible call carrying more than
//! `max_args` arguments, rendering one keyword per line. The length
//! trigger fires on any call whose line crosses `code_line_length`,
//! exploding a keyword-expressible call in keyword form and any other
//! call positionally. The closing `)` drops to the call's own indent,
//! and a nested call in an argument value explodes in the same pass.
//! Argument order, `=` alignment, and trailing-comma policy stay with
//! `alphabetize`, `align_equals`, and `strip_trailing_commas`.

use std::collections::HashMap;

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
        INDENT_STEP,
        call_keywords::{CallKeywords, keyword_args, module_call_params, resolve_call_params},
        edit::{narrowed_replacement, singleton_groups},
        equal_targets::keyword_groups,
        layout::{explode_parens, is_layoutable, reindent_block},
    },
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct CallLayout {
    code_line_length: usize,
    max_args: Option<usize>,
}

impl CallLayout {
    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            code_line_length: config.code_width(),
            max_args: config.rules.call_layout.max_args.cap(),
        }
    }
}

impl Rule for CallLayout {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let targets = module_call_params(source);
        let mut exploder = Exploder {
            cap: self.max_args,
            code_line_length: self.code_line_length,
            edits: Vec::new(),
            shifts: Vec::new(),
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

struct Exploder<'a> {
    cap: Option<usize>,
    code_line_length: usize,
    edits: Vec<Edit>,
    shifts: Vec<(TextSize, usize)>,
    source: &'a Source,
    targets: &'a HashMap<TextSize, &'a Parameters>,
}

impl Exploder<'_> {
    /// The columns `align_equals` adds before `call` when it is the value
    /// of an enclosing keyword whose `=` gaps collapse to one space
    /// apiece, zero for a call reached by any other route.
    fn align_shift(&self, call: &ExprCall) -> usize {
        self.shifts
            .iter()
            .rev()
            .find(|(start, _)| *start == call.start())
            .map_or(0, |(_, shift)| *shift)
    }

    /// Returns the exploded `(...)` text for `call` when the count or
    /// length trigger fires, the closing `)` landing at `indent`. A
    /// keyword-expressible call renders one keyword per line, while any
    /// other call renders positionally under the length trigger only. A
    /// nested call in an argument value explodes in the same text. `None`
    /// leaves the call inline.
    fn explode_args(&self, call: &ExprCall, indent: usize) -> Option<String> {
        let arguments = &call.arguments;
        if arguments.is_empty() || self.source.intersects_comment(arguments.inner_range()) {
            return None;
        }
        let count_trips = self.cap.is_some_and(|cap| arguments.len() > cap);
        let length_trips = self.overflows_line(call);
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
        keywords: &CallKeywords,
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
    fn explode_source_order(&self, call: &ExprCall, indent: usize) -> String {
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

    /// True when `call` sits inline on its physical line and rendering it
    /// there crosses `code_line_length`, measured at the column
    /// [`Self::align_shift`] leaves it in.
    fn overflows_line(&self, call: &ExprCall) -> bool {
        let text = self.source.slice(call.range());
        !self.source.contains_line_break(call.range())
            && self.source.column_overflows(
                call.start(),
                text.width() + self.align_shift(call),
                self.code_line_length,
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

    /// Appends `rendered` to `out`, swapping a nested call value's
    /// argument list for its own exploded form and a multi-line
    /// collection or comprehension value for that block re-indented to
    /// the keyword column, keeping everything before the value verbatim
    /// so nesting resolves in one pass.
    fn render_value(&self, out: &mut String, value: &Expr, rendered: &str, indent: usize) {
        if let Expr::Call(inner) = value
            && let Some(args_text) = self.explode_args(inner, indent)
        {
            let inner_args = self.source.slice(inner.arguments.range());
            let head = rendered.strip_suffix(inner_args).unwrap_or(rendered);
            out.push_str(head);
            out.push_str(&args_text);
        } else if self.reindentable(value)
            && let Some(head) = rendered.strip_suffix(self.source.slice(value.range()))
        {
            out.push_str(head);
            out.push_str(&reindent_block(self.source.slice(value.range()), indent));
        } else {
            out.push_str(rendered);
        }
    }
}

impl<'a> AstVisitor<'a> for Exploder<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Call(call) = expr {
            let indent = self.source.line_indent_width(call.start());
            if let Some(text) = self.explode_args(call, indent) {
                self.edits.extend(narrowed_replacement(
                    self.source,
                    call.arguments.range(),
                    text,
                ));
                return;
            }
            let depth = self.shifts.len();
            self.shifts
                .extend(buffered_keyword_values(self.source, call));
            walk_expr(self, expr);
            self.shifts.truncate(depth);
            return;
        }
        walk_expr(self, expr);
    }
}

/// Each keyword value of `call` that `align_equals` shifts right, paired
/// with the columns it gains as the gaps on either side of its `=`
/// collapse to one space apiece. A keyword sharing its physical line
/// with another argument keeps its tight `name=value` and is absent.
fn buffered_keyword_values(source: &Source, call: &ExprCall) -> Vec<(TextSize, usize)> {
    keyword_groups(source, CallLayout::SLUG, call, true)
        .into_iter()
        .flatten()
        .filter_map(|m| {
            let value_gap = m.value_gap?;
            let gained = one_space_gain(m.gap) + one_space_gain(value_gap);
            Some((value_gap.end(), gained))
        })
        .collect()
}

/// The columns `gap` gains when it collapses to one space, zero for a
/// gap already at least that wide.
fn one_space_gain(gap: TextRange) -> usize {
    1usize.saturating_sub(gap.len().to_usize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{applied_text, parse};

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
