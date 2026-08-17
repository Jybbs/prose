//! Normalizes function signatures to a binary shape, one line or one
//! parameter per line, gated by `code_line_length`, `max_params`, and a
//! parameter whose annotation or default spans rows while hanging from
//! the parameter's own row. Comments inside `()` pin the existing shape.

use std::borrow::Cow;

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    AnyParameterRef, Parameters, Stmt, StmtFunctionDef,
    statement_visitor::{StatementVisitor, walk_stmt},
    token::TokenKind,
};
use ruff_python_parser::parse_module;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::{
    config::Config,
    primitives::{
        call_keywords::module_call_params,
        edit::{narrowed_replacement, singleton_groups},
        inline::opening_width,
        layout::{Separator, explode_parens, item_indent},
        one_row,
        range::return_annotation_range,
        splice::splice_parses,
        travel::{Landing, placed_block},
    },
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct ReflowSignatures {
    code_line_length: usize,
    max_params: Option<usize>,
    one_row: one_row::Settings<'static>,
}

impl ReflowSignatures {
    pub(crate) const MESSAGE: &'static str =
        "normalize function signature to one-line or one-per-line shape";

    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            code_line_length: config.code_width(),
            max_params: config.rules.reflow_signatures.max_params.cap(),
            one_row: config.one_row_settings(),
        }
    }
}

impl Rule for ReflowSignatures {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let targets = module_call_params(source);
        let mut visitor = Layout {
            code_line_length: self.code_line_length,
            edits: Vec::new(),
            max_params: self.max_params,
            newline: source.newline_str(),
            one_row: self.one_row.against(&targets),
            source,
        };
        visitor.visit_body(&source.ast().body);
        singleton_groups(visitor.edits)
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

struct Layout<'a> {
    code_line_length: usize,
    edits: Vec<Edit>,
    max_params: Option<usize>,
    newline: &'static str,
    one_row: one_row::Settings<'a>,
    source: &'a Source,
}

impl Layout<'_> {
    /// Builds the canonical expanded text spanning `(` through `:` from
    /// `parts`, one parameter per line.
    fn build_expanded(&self, fd: &StmtFunctionDef, parts: &[Cow<str>], indent: usize) -> String {
        let mut out = explode_parens(
            self.newline,
            indent,
            parts.len(),
            |out, i| out.push_str(&parts[i]),
            Separator::Comma,
        );
        self.push_return_and_colon(&mut out, fd);
        out
    }

    /// Builds the canonical inline text spanning `(` through `:` from
    /// `parts`.
    fn build_inline(&self, fd: &StmtFunctionDef, parts: &[Cow<str>]) -> String {
        let mut out = format!("({})", parts.join(", "));
        self.push_return_and_colon(&mut out, fd);
        out
    }

    /// `param`'s source text placed at `indent`, its annotation and its
    /// default the expressions a move must not pad. A variadic parameter
    /// carries its `*` or `**` prefix and holds no default.
    fn place<'p>(&'p self, param: AnyParameterRef, indent: usize) -> Cow<'p, str> {
        placed_block(
            self.source,
            param.range(),
            Landing::own_row(param.start(), indent),
        )
    }

    /// Emits one expand or collapse edit when `fd`'s signature
    /// diverges from the canonical inline-or-expanded form.
    fn process_def(&mut self, fd: &StmtFunctionDef) {
        let params = &fd.parameters;
        let one = TextSize::from(1u32);
        if self
            .source
            .intersects_comment(params.range().add_start(one).sub_end(one))
        {
            return;
        }
        let indent = self.source.line_indent_width(fd.start());
        let replacement_range = self.replacement_range(fd);
        let count_trips = self.max_params.is_some_and(|cap| params.len() > cap);
        // The one-row reading answers whether every parameter reaches a
        // single row at all, so a signature holding one that cannot is
        // laid out one per line whatever its width would have been.
        let inline = rendered_parts(params, |p| {
            self.one_row.parameter_form(self.source, p).map(Cow::Owned)
        })
        .map(|parts| self.build_inline(fd, &parts));
        let fits = inline.as_ref().is_some_and(|text| {
            !self.source.column_overflows(
                params.range().start(),
                opening_width(text),
                self.code_line_length,
            )
        });
        let replacement = if count_trips || !fits {
            let item = item_indent(indent);
            let parts = rendered_parts(params, |p| Some(self.place(p, item)))
                .expect("placing a parameter always renders");
            self.build_expanded(fd, &parts, indent)
        } else if self.source.contains_line_break(replacement_range) {
            inline.expect("a fitting signature carries its one-row parts")
        } else {
            return;
        };
        // Emit the reshape only when the spliced signature re-parses, the
        // safety net for return types the rewrite cannot reassemble.
        if splice_parses(
            self.source,
            fd.range(),
            replacement_range,
            &replacement,
            parse_module,
        ) {
            self.edits.extend(narrowed_replacement(
                self.source,
                replacement_range,
                replacement,
            ));
        }
    }

    fn push_return_and_colon(&self, out: &mut String, fd: &StmtFunctionDef) {
        if let Some(ret) = fd.returns.as_deref() {
            out.push_str(" -> ");
            let range = return_annotation_range(ret, fd, self.source.tokens());
            out.push_str(self.source.slice(range));
        }
        out.push(':');
    }

    /// Returns the range covering the signature's `(` through `:`,
    /// the surface this rule rewrites.
    ///
    /// # Panics
    ///
    /// Panics if `fd.body` is empty or the `:` token cannot be located
    /// between `)` and the body.
    fn replacement_range(&self, fd: &StmtFunctionDef) -> TextRange {
        let body_start = fd
            .body
            .first()
            .expect("function def has a non-empty body")
            .start();
        let colon = self
            .source
            .first_token_offset_in_range(
                TextRange::new(fd.parameters.range().end(), body_start),
                |t| t.kind() == TokenKind::Colon,
            )
            .expect("function def carries a `:` between `)` and the body");
        TextRange::new(fd.parameters.range().start(), colon + TextSize::from(1u32))
    }
}

impl<'a> StatementVisitor<'a> for Layout<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::FunctionDef(fd) = stmt {
            self.process_def(fd);
        }
        walk_stmt(self, stmt);
    }
}

/// Every parameter of `params` rendered in source order through
/// `render`, with `/` and bare `*` seated at their canonical positions,
/// `None` where any parameter renders to `None`.
fn rendered_parts<'p>(
    params: &'p Parameters,
    mut render: impl FnMut(AnyParameterRef<'p>) -> Option<Cow<'p, str>>,
) -> Option<Vec<Cow<'p, str>>> {
    let mut parts = Vec::new();
    for param in params.posonlyargs.iter().map(AnyParameterRef::NonVariadic) {
        parts.push(render(param)?);
    }
    if !params.posonlyargs.is_empty() {
        parts.push(Cow::Borrowed("/"));
    }
    for param in params.args.iter().map(AnyParameterRef::NonVariadic) {
        parts.push(render(param)?);
    }
    if let Some(va) = params.vararg.as_deref() {
        parts.push(render(AnyParameterRef::Variadic(va))?);
    } else if !params.kwonlyargs.is_empty() {
        parts.push(Cow::Borrowed("*"));
    }
    for param in params.kwonlyargs.iter().map(AnyParameterRef::NonVariadic) {
        parts.push(render(param)?);
    }
    if let Some(kw) = params.kwarg.as_deref() {
        parts.push(render(AnyParameterRef::Variadic(kw))?);
    }
    Some(parts)
}
