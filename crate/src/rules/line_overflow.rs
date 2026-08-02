//! Flags each physical line still over its governing cap once no legal
//! reshape remains. A line inside an import statement answers to
//! `import_line_length`, every other line to `code_line_length`. A line
//! a layout rule could still split (an inline call carrying arguments,
//! a multi-element collection, a multi-name `from` import, a signature
//! carrying parameters, a single-statement match arm) is left for that
//! rule, so only the narrowest legal form that no split can shorten
//! surfaces here. No rule reaches a construct inside an f-string or
//! t-string replacement field, so its line surfaces here as well.
//! Lint-only, emits no edits.

use ruff_python_ast::{
    Expr, InterpolatedStringElement, Stmt, StmtFunctionDef, StmtMatch,
    helpers::is_compound_statement,
    visitor::{Visitor, walk_expr, walk_stmt},
};
use ruff_source_file::UniversalNewlines;
use ruff_text_size::{Ranged, TextRange};
use unicode_width::UnicodeWidthStr;

use crate::{
    config::Config,
    diagnostics::Diagnostic,
    primitives::docstring::body_docstring,
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct LineOverflow {
    code_line_length: usize,
    import_line_length: usize,
}

impl LineOverflow {
    pub(crate) const MESSAGE: &'static str =
        "Flag a line over its length budget that no reshape can bring within";

    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            code_line_length: config.code_width(),
            import_line_length: config.import_width(),
        }
    }
}

impl Rule for LineOverflow {
    fn id(&self) -> RuleId {
        Self::SLUG
    }

    fn lint(&self, source: &Source) -> Vec<Diagnostic> {
        let mut spans = Spans {
            imports: Vec::new(),
            reshapeable: Vec::new(),
            source,
        };
        spans.note_docstring(&source.ast().body);
        spans.visit_body(&source.ast().body);
        let mut diagnostics = Vec::new();
        for line in source.text().universal_newlines() {
            let range = line.range();
            let cap = if spans.imports.iter().any(|r| r.contains_range(range)) {
                self.import_line_length
            } else {
                self.code_line_length
            };
            let width = line.as_str().width();
            if width > cap
                && !spans
                    .reshapeable
                    .iter()
                    .any(|r| r.intersect(range).is_some())
            {
                diagnostics.push(Diagnostic::lint(
                    self.id(),
                    range,
                    format!("Line is {width} columns, over the {cap}-column budget, with no legal reshape"),
                ));
            }
        }
        diagnostics
    }
}

/// Gathers the import-statement ranges that shift a line to the import
/// budget and the still-collapsible construct ranges a layout rule
/// could shorten, so a line intersecting one is left for that rule.
struct Spans<'a> {
    imports: Vec<TextRange>,
    reshapeable: Vec<TextRange>,
    source: &'a Source,
}

impl Spans<'_> {
    /// Records a leading docstring's whole range, the prose
    /// `docstring-wrap` reflows to the budget.
    fn note_docstring(&mut self, body: &[Stmt]) {
        if let Some(lit) = body_docstring(body) {
            self.reshapeable.push(lit.range());
        }
    }

    /// Records `range` as reshapeable when it sits on one source line,
    /// the form a layout rule can still explode.
    fn note_inline(&mut self, range: TextRange) {
        if !self.source.contains_line_break(range) {
            self.reshapeable.push(range);
        }
    }

    /// Records a single-statement match arm on one source line, the
    /// form `align-match-case` splits onto the next line.
    fn note_match(&mut self, m: &StmtMatch) {
        for case in &m.cases {
            if let [body] = case.body.as_slice()
                && !is_compound_statement(body)
            {
                self.note_inline(case.range());
            }
        }
    }

    /// Records a signature carrying parameters, the form
    /// `signature-layout` explodes one parameter per line.
    fn note_signature(&mut self, fd: &StmtFunctionDef) {
        if !fd.parameters.is_empty() {
            self.note_inline(fd.parameters.range());
        }
    }
}

impl<'a> Visitor<'a> for Spans<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        let splittable = match expr {
            Expr::Call(call) => !call.arguments.is_empty(),
            Expr::Dict(d) => d.len() >= 2,
            Expr::List(l) => l.len() >= 2,
            Expr::Set(s) => s.len() >= 2,
            Expr::Tuple(t) => t.len() >= 2,
            _ => false,
        };
        if splittable {
            self.note_inline(expr.range());
        }
        walk_expr(self, expr);
    }

    /// Leaves a replacement field unwalked.
    fn visit_interpolated_string_element(&mut self, _: &'a InterpolatedStringElement) {}

    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::Import(i) => self.imports.push(i.range()),
            Stmt::ImportFrom(i) => {
                self.imports.push(i.range());
                if i.names.len() >= 2 {
                    self.note_inline(i.range());
                }
            }
            Stmt::ClassDef(cd) => self.note_docstring(&cd.body),
            Stmt::FunctionDef(fd) => {
                self.note_signature(fd);
                self.note_docstring(&fd.body);
            }
            Stmt::Match(m) => self.note_match(m),
            _ => {}
        }
        walk_stmt(self, stmt);
    }
}
