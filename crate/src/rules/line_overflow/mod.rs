//! Flags each physical line still over its governing cap once no legal
//! reshape remains. A line inside an import statement answers to
//! `import_line_length`, every other line to `code_line_length`. A line
//! a layout rule could still split (an inline call carrying arguments,
//! a multi-element collection, a multi-name `from` import, a signature
//! carrying parameters, a single-statement match arm) is left for that
//! rule. No rule reaches a construct inside an f-string or t-string
//! replacement field, so its line surfaces here as well. A line whose
//! overflow sits inside one string literal holding interior whitespace
//! carries the [`split`] form as a display-only suggestion, gated by
//! `suggest_string_splits`. Lint-only, emits no edits.

use ruff_python_ast::{
    Expr, ExprStringLiteral, InterpolatedStringElement, Stmt, StmtFunctionDef, StmtMatch,
    StringLiteral,
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

mod split;

pub(crate) struct LineOverflow {
    code_line_length: usize,
    import_line_length: usize,
    suggest_string_splits: bool,
}

impl LineOverflow {
    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            code_line_length: config.code_width(),
            import_line_length: config.import_width(),
            suggest_string_splits: config.rules.line_overflow.suggest_string_splits,
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
            strings: Vec::new(),
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
            if width <= cap
                || spans
                    .reshapeable
                    .iter()
                    .any(|r| r.intersect(range).is_some())
            {
                continue;
            }
            let report = format!("Line is {width} columns, over the {cap}-column budget");
            let lit = spans.straddling(range, cap);
            diagnostics.push(
                match lit.and_then(|lit| split::concatenation(source, lit, cap)) {
                    Some(edit) if self.suggest_string_splits => Diagnostic::suggestion(
                        self.id(),
                        range,
                        format!("{report}, with a legal reshape at the string literal"),
                        edit,
                    ),
                    Some(_) => Diagnostic::lint(self.id(), range, report),
                    None if lit.is_some_and(|lit| split::has_interior_break(source, lit)) => {
                        Diagnostic::lint(self.id(), range, report)
                    }
                    None => Diagnostic::lint(
                        self.id(),
                        range,
                        format!("{report}, with no legal reshape"),
                    ),
                },
            );
        }
        diagnostics
    }
}

/// Gathers the import-statement ranges that shift a line to the import
/// budget, the still-collapsible construct ranges a layout rule could
/// shorten so a line intersecting one is left for that rule, and the
/// one-line string literals a suggested reshape can split.
struct Spans<'a> {
    imports: Vec<TextRange>,
    reshapeable: Vec<TextRange>,
    source: &'a Source,
    strings: Vec<&'a StringLiteral>,
}

impl<'a> Spans<'a> {
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

    /// Records a string literal written as one part on one source line,
    /// the form the adjacent-literal suggestion splits.
    fn note_string(&mut self, expr: &'a ExprStringLiteral) {
        if let [lit] = expr.value.as_slice()
            && !self.source.contains_line_break(lit)
        {
            self.strings.push(lit);
        }
    }

    /// The single-part string literal on `line` whose span crosses the
    /// `cap` column.
    fn straddling(&self, line: TextRange, cap: usize) -> Option<&StringLiteral> {
        self.strings.iter().copied().find(|lit| {
            line.contains_range(lit.range())
                && self.source.width_between(line.start(), lit.start()) <= cap
                && self.source.width_between(line.start(), lit.end()) > cap
        })
    }
}

impl<'a> Visitor<'a> for Spans<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Call(call) if !call.arguments.is_empty() => self.note_inline(expr.range()),
            Expr::Dict(d) if d.len() >= 2 => self.note_inline(expr.range()),
            Expr::List(l) if l.len() >= 2 => self.note_inline(expr.range()),
            Expr::Set(s) if s.len() >= 2 => self.note_inline(expr.range()),
            Expr::StringLiteral(s) => self.note_string(s),
            Expr::Tuple(t) if t.len() >= 2 => self.note_inline(expr.range()),
            _ => {}
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
