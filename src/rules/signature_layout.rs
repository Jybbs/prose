//! Normalizes function-signature layout to a binary shape. A signature
//! whose canonical inline form fits under `Config::code_line_length`
//! and whose parameter count fits under `max_inline_params` collapses
//! to one line. A signature that trips either threshold expands to
//! one parameter per line with a trailing comma, opening paren at the
//! end of the `def` line, and closing paren returning to the `def`'s
//! own indent. A comment anywhere between `(` and `)` pins the
//! existing shape.

use std::num::NonZeroUsize;

use ruff_diagnostics::Edit;
use ruff_python_ast::statement_visitor::{walk_stmt, StatementVisitor};
use ruff_python_ast::token::TokenKind;
use ruff_python_ast::{Parameters, Stmt, StmtFunctionDef};
use ruff_text_size::{Ranged, TextRange, TextSize};
use unicode_width::UnicodeWidthStr;

use crate::config::Config;
use crate::primitives::{edit::narrowed_replacement, INDENT_STEP};
use crate::rule::{Rule, RuleId};
use crate::source::Source;

pub(crate) struct SignatureLayout {
    code_line_length: usize,
    max_inline_params: Option<usize>,
}

impl SignatureLayout {
    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            code_line_length: config
                .code_line_length
                .expect("Config::default synthesizes Some(88)")
                .get(),
            max_inline_params: config
                .rules
                .signature_layout
                .max_inline_params
                .map(NonZeroUsize::get),
        }
    }
}

impl Rule for SignatureLayout {
    fn apply(&self, source: &Source) -> Vec<Edit> {
        let mut visitor = Layout {
            code_line_length: self.code_line_length,
            edits: Vec::new(),
            max_inline_params: self.max_inline_params,
            newline: source.newline_str(),
            source,
        };
        visitor.visit_body(&source.ast().body);
        visitor.edits
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

struct Layout<'a> {
    code_line_length: usize,
    edits: Vec<Edit>,
    max_inline_params: Option<usize>,
    newline: &'static str,
    source: &'a Source,
}

impl Layout<'_> {
    /// Builds the canonical expanded text starting at `(` and running
    /// through `:`, with each parameter on its own line at one indent
    /// step beyond `indent` and the closing `)` at `indent`.
    fn build_expanded(&self, fd: &StmtFunctionDef, indent: usize) -> String {
        let prefix = " ".repeat(indent + INDENT_STEP);
        let mut out = String::from("(");
        for part in self.signature_parts(&fd.parameters) {
            out.push_str(self.newline);
            out.push_str(&prefix);
            out.push_str(part);
            out.push(',');
        }
        out.push_str(self.newline);
        out.extend(std::iter::repeat_n(' ', indent));
        out.push(')');
        self.push_return_and_colon(&mut out, fd);
        out
    }

    /// Builds the canonical inline text starting at `(` and running
    /// through `:`, joining parameters with `, ` separators.
    fn build_inline(&self, fd: &StmtFunctionDef) -> String {
        let parts: Vec<&str> = self.signature_parts(&fd.parameters).collect();
        let mut out = format!("({})", parts.join(", "));
        self.push_return_and_colon(&mut out, fd);
        out
    }

    /// Emits one expand or collapse edit for `fd` when its current
    /// shape diverges from the canonical form implied by the parameter
    /// count, the spread already in source, and the `code_line_length`
    /// / `max_inline_params` thresholds.
    fn process_def(&mut self, fd: &StmtFunctionDef) {
        let params = &fd.parameters;
        let pin = TextRange::new(
            params.range().start() + TextSize::from(1u32),
            params.range().end() - TextSize::from(1u32),
        );
        if self.source.intersects_comment(pin) {
            return;
        }
        let replacement_range = self.replacement_range(fd);
        let inline = self.build_inline(fd);
        let count_trips = self.max_inline_params.is_some_and(|cap| params.len() > cap);
        let length_trips =
            self.source.column_of(params.range().start()) + inline.width() > self.code_line_length;
        let replacement = if count_trips || length_trips {
            self.build_expanded(fd, self.source.line_indent_width(fd.start()))
        } else if self.source.contains_line_break(replacement_range) {
            inline
        } else {
            return;
        };
        self.edits.extend(narrowed_replacement(
            self.source,
            replacement_range,
            replacement,
        ));
    }

    fn push_return_and_colon(&self, out: &mut String, fd: &StmtFunctionDef) {
        if let Some(ret) = &fd.returns {
            out.push_str(" -> ");
            out.push_str(self.source.slice(ret.range()));
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

    /// Returns each parameter's source slice in source order, with
    /// `/` and bare `*` separators inserted at their canonical
    /// positions. Variadic parameters carry their `*` or `**` prefix.
    fn signature_parts<'p>(&'p self, params: &'p Parameters) -> impl Iterator<Item = &'p str> + 'p {
        let posonly_sep = (!params.posonlyargs.is_empty()).then_some("/");
        let star = params
            .vararg
            .as_deref()
            .map(|va| self.source.slice(va.range()))
            .or((!params.kwonlyargs.is_empty()).then_some("*"));
        params
            .posonlyargs
            .iter()
            .map(move |p| self.source.slice(p.range()))
            .chain(posonly_sep)
            .chain(
                params
                    .args
                    .iter()
                    .map(move |p| self.source.slice(p.range())),
            )
            .chain(star)
            .chain(
                params
                    .kwonlyargs
                    .iter()
                    .map(move |p| self.source.slice(p.range())),
            )
            .chain(
                params
                    .kwarg
                    .as_deref()
                    .map(|kw| self.source.slice(kw.range())),
            )
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
