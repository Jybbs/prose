//! Normalizes function signatures to a binary shape, one line or one
//! parameter per line, gated by `code_line_length`, `max_params`, and a
//! parameter whose annotation or default spans rows while hanging from
//! the parameter's own row. Comments inside `()` pin the existing shape.

use std::borrow::Cow;

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    AnyParameterRef, ParameterWithDefault, Parameters, Stmt, StmtFunctionDef,
    statement_visitor::{StatementVisitor, walk_stmt},
    token::TokenKind,
};
use ruff_python_parser::parse_module;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::{
    config::Config,
    primitives::{
        edit::{narrowed_replacement, singleton_groups, splice_parses},
        inline::opening_width,
        layout::{Separator, explode_parens, hangs_from_its_row, item_indent, placed_block},
        range::return_annotation_range,
    },
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct SignatureLayout {
    code_line_length: usize,
    max_params: Option<usize>,
}

impl SignatureLayout {
    pub(crate) const MESSAGE: &'static str =
        "normalize function signature to one-line or one-per-line shape";

    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            code_line_length: config.code_width(),
            max_params: config.rules.signature_layout.max_params.cap(),
        }
    }
}

impl Rule for SignatureLayout {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let mut visitor = Layout {
            code_line_length: self.code_line_length,
            edits: Vec::new(),
            max_params: self.max_params,
            newline: source.newline_str(),
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
            [param.annotation(), param.default()].into_iter().flatten(),
            indent,
        )
    }

    fn place_params<'p>(
        &'p self,
        params: &'p [ParameterWithDefault],
        indent: usize,
    ) -> impl Iterator<Item = Cow<'p, str>> + 'p {
        params
            .iter()
            .map(move |p| self.place(AnyParameterRef::NonVariadic(p), indent))
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
        let parts: Vec<Cow<str>> = self.signature_parts(params, item_indent(indent)).collect();
        let replacement_range = self.replacement_range(fd);
        let inline = self.build_inline(fd, &parts);
        let count_trips = self.max_params.is_some_and(|cap| params.len() > cap);
        // A part aligned under an interior bracket would land against
        // nothing once its row moves, so it leaves the signature alone.
        let span_trips = params.iter().any(|param| {
            hangs_from_its_row(self.source, param.start(), self.source.slice(param.range()))
        });
        let length_trips = self.source.column_overflows(
            params.range().start(),
            opening_width(&inline),
            self.code_line_length,
        );
        let replacement = if count_trips || length_trips || span_trips {
            self.build_expanded(fd, &parts, indent)
        } else if self.source.contains_line_break(replacement_range) {
            inline
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

    /// Returns each parameter's text in source order, with `/` and bare
    /// `*` separators inserted at their canonical positions and a
    /// row-spanning annotation or default hung from `indent`. Variadic
    /// parameters are placed the same way and carry their `*` or `**`
    /// prefix.
    fn signature_parts<'p>(
        &'p self,
        params: &'p Parameters,
        indent: usize,
    ) -> impl Iterator<Item = Cow<'p, str>> + 'p {
        let posonly_sep = (!params.posonlyargs.is_empty()).then_some(Cow::Borrowed("/"));
        let star = params
            .vararg
            .as_deref()
            .map(|va| self.place(AnyParameterRef::Variadic(va), indent))
            .or((!params.kwonlyargs.is_empty()).then_some(Cow::Borrowed("*")));
        let kwarg = params
            .kwarg
            .as_deref()
            .map(|kw| self.place(AnyParameterRef::Variadic(kw), indent));
        self.place_params(&params.posonlyargs, indent)
            .chain(posonly_sep)
            .chain(self.place_params(&params.args, indent))
            .chain(star)
            .chain(self.place_params(&params.kwonlyargs, indent))
            .chain(kwarg)
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
