//! Explodes a keyword-expressible call carrying more than
//! `max_inline_args` arguments to one keyword argument per line, leaving
//! shorter calls and calls that cannot take keyword form inline. The
//! closing `)` drops to the call's own indent, and a nested call in an
//! argument value explodes in the same pass. Argument order, `=`
//! alignment, and trailing-comma policy stay with `alphabetize`,
//! `align_equals`, and `strip_trailing_commas`.

use std::{collections::HashMap, num::NonZeroUsize};

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    Expr, ExprCall, Parameters, StringLike,
    helpers::any_over_expr,
    visitor::{Visitor as AstVisitor, walk_expr},
};
use ruff_text_size::{Ranged, TextSize};

use crate::{
    config::Config,
    primitives::{
        INDENT_STEP,
        call_keywords::{keyword_args, module_call_params, resolve_call_params},
        edit::{narrowed_replacement, singleton_groups},
        layout::{explode_parens, is_layoutable, reindent_block},
    },
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct CallLayout {
    max_inline_args: Option<usize>,
}

impl CallLayout {
    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            max_inline_args: config
                .rules
                .call_layout
                .max_inline_args
                .map(NonZeroUsize::get),
        }
    }
}

impl Rule for CallLayout {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let Some(cap) = self.max_inline_args else {
            return Vec::new();
        };
        let targets = module_call_params(source);
        let mut exploder = Exploder {
            cap,
            edits: Vec::new(),
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
    cap: usize,
    edits: Vec<Edit>,
    source: &'a Source,
    targets: &'a HashMap<TextSize, &'a Parameters>,
}

impl Exploder<'_> {
    /// Returns the exploded `(...)` text for `call` when it carries more
    /// than `cap` keyword-expressible arguments, the closing `)` landing
    /// at `indent`. A nested call in an argument value explodes in the
    /// same text. `None` leaves the call inline.
    fn explode_args(&self, call: &ExprCall, indent: usize) -> Option<String> {
        let arguments = &call.arguments;
        if arguments.args.len() + arguments.keywords.len() <= self.cap {
            return None;
        }
        if self.source.intersects_comment(arguments.inner_range()) {
            return None;
        }
        let keywords = keyword_args(self.source, call, resolve_call_params(call, self.targets))?;
        // A positional-only prefix cannot take keyword form, so the call
        // stays inline rather than exploding only part of its arguments.
        if keywords.has_posonly_prefix {
            return None;
        }
        let item_indent = indent + INDENT_STEP;
        let last = keywords.args.len() - 1;
        let trailing = self.source.trailing_comma(call.arguments.range()).is_some();
        let out = explode_parens(
            self.source.newline_str(),
            indent,
            keywords.args.len(),
            |out, i| {
                let arg = &keywords.args[i];
                self.render_value(out, arg.value, &arg.rendered, item_indent);
            },
            |i| trailing || i < last,
        );
        Some(out)
    }

    /// True for a multi-line `dict`, `list`, `set`, or `tuple` value
    /// whose already-exploded block re-indents to the keyword column. A
    /// value spanning a string literal is excluded, leaving it at the
    /// verbatim floor, since re-indenting would pad the string interior.
    fn reindentable(&self, value: &Expr) -> bool {
        is_layoutable(value)
            && self.source.contains_line_break(value.range())
            && !any_over_expr(value, |e| {
                StringLike::try_from(e).is_ok() && self.source.contains_line_break(e.range())
            })
    }

    /// Appends `rendered` to `out`, swapping a nested call value's
    /// argument list for its own exploded form and a multi-line
    /// collection value for that block re-indented to the keyword
    /// column, keeping everything before the value verbatim so nesting
    /// resolves in one pass.
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
        }
        walk_expr(self, expr);
    }
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
