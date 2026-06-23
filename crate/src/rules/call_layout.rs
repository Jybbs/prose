//! Explodes a keyword-expressible call carrying more than
//! `max_args` arguments to one keyword argument per line, leaving
//! shorter calls and calls that cannot take keyword form inline. The
//! closing `)` drops to the call's own indent, and a nested call in an
//! argument value explodes in the same pass. Argument order, `=`
//! alignment, and trailing-comma policy stay with `alphabetize`,
//! `align_equals`, and `strip_trailing_commas`.

use std::collections::HashMap;

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    Expr, ExprCall, Parameters,
    visitor::{Visitor as AstVisitor, walk_expr},
};
use ruff_text_size::{Ranged, TextSize};

use crate::{
    config::Config,
    primitives::{
        INDENT_STEP,
        call_keywords::{keyword_args, module_call_params, resolve_call_params},
        edit::{narrowed_replacement, singleton_groups},
        layout::explode_parens,
    },
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct CallLayout {
    max_args: Option<usize>,
}

impl CallLayout {
    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            max_args: config.rules.call_layout.max_args.cap(),
        }
    }
}

impl Rule for CallLayout {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let Some(cap) = self.max_args else {
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

    /// Appends `rendered` to `out`, swapping a nested call value's
    /// argument list for its own exploded form while keeping everything
    /// before it verbatim, so nesting resolves in one pass.
    fn render_value(&self, out: &mut String, value: &Expr, rendered: &str, indent: usize) {
        if let Expr::Call(inner) = value
            && let Some(args_text) = self.explode_args(inner, indent)
        {
            let inner_args = self.source.slice(inner.arguments.range());
            let head = rendered.strip_suffix(inner_args).unwrap_or(rendered);
            out.push_str(head);
            out.push_str(&args_text);
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
