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
    Expr, ExprCall, Parameters,
    visitor::{Visitor as AstVisitor, walk_expr},
};
use ruff_python_trivia::{BackwardsTokenizer, SimpleTokenKind};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::{
    config::Config,
    primitives::{
        INDENT_STEP,
        call_keywords::{keyword_args, module_call_params},
        edit::{narrowed_replacement, singleton_groups},
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
        let targets = module_call_params(source, |_| true);
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

impl<'a> Exploder<'a> {
    /// Returns the exploded `(...)` text for `call` when it carries more
    /// than `cap` keyword-expressible arguments, the closing `)` landing
    /// at `indent`. A nested call in an argument value explodes in the
    /// same text. `None` leaves the call inline.
    fn explode_args(&self, call: &ExprCall, indent: usize) -> Option<String> {
        let arguments = &call.arguments;
        if arguments.args.len() + arguments.keywords.len() <= self.cap {
            return None;
        }
        let one = TextSize::from(1u32);
        if self
            .source
            .intersects_comment(arguments.range().add_start(one).sub_end(one))
        {
            return None;
        }
        let params = call
            .func
            .as_name_expr()
            .and_then(|callee| self.targets.get(&callee.range().start()).copied());
        let keywords = keyword_args(self.source, call, params)?;
        // A positional-only prefix cannot take keyword form, so the call
        // stays inline rather than exploding only part of its arguments.
        if keywords.posonly_prefix != 0 {
            return None;
        }
        let item_indent = indent + INDENT_STEP;
        let prefix = " ".repeat(item_indent);
        let newline = self.source.newline_str();
        let last = keywords.args.len() - 1;
        let trailing = self.source_trailing_comma(call);
        let mut out = String::from("(");
        for (i, arg) in keywords.args.iter().enumerate() {
            out.push_str(newline);
            out.push_str(&prefix);
            self.render_value(&mut out, arg.value, &arg.rendered, item_indent);
            if trailing || i < last {
                out.push(',');
            }
        }
        out.push_str(newline);
        out.extend(std::iter::repeat_n(' ', indent));
        out.push(')');
        Some(out)
    }

    /// Appends `rendered` to `out`, swapping a nested call value for its
    /// own exploded form while keeping the `name=` prefix verbatim, so
    /// nesting resolves in one pass.
    fn render_value(&self, out: &mut String, value: &Expr, rendered: &str, indent: usize) {
        if let Expr::Call(inner) = value
            && let Some(args_text) = self.explode_args(inner, indent)
        {
            let inline = self.source.slice(value);
            let head = rendered.strip_suffix(inline).unwrap_or(rendered);
            let callee = self
                .source
                .slice(TextRange::new(inner.start(), inner.arguments.start()));
            out.push_str(head);
            out.push_str(callee);
            out.push_str(&args_text);
        } else {
            out.push_str(rendered);
        }
    }

    /// True when the last non-trivia token before the closing `)` is a
    /// comma, the trailing-comma state `explode_args` carries through so
    /// the explode neither adds nor drops one.
    fn source_trailing_comma(&self, call: &ExprCall) -> bool {
        BackwardsTokenizer::up_to(
            call.arguments.end() - TextSize::from(1u32),
            self.source.text(),
            self.source.comment_ranges(),
        )
        .skip_trivia()
        .next()
        .is_some_and(|token| token.kind() == SimpleTokenKind::Comma)
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
