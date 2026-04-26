//! Aligns `:` vertically in dict/mapping literals, Pydantic-style
//! class fields, annotated function parameters, and Google/numpy
//! docstring `Args:` sections. Single-line groups and single-item
//! groups pass through, leaving the latter to `singleton_rule`
//! downstream. Each aligned `:` keeps a one-space buffer before the
//! colon.

use ruff_diagnostics::Edit;
use ruff_python_ast::visitor::{walk_expr, walk_parameters, walk_stmt, Visitor as AstVisitor};
use ruff_python_ast::{Expr, ExprDict, Parameters, Stmt};
use ruff_text_size::Ranged;

use crate::config::Config;
use crate::pipeline::Rule;
use crate::primitives::{aligner, colon_targets};
use crate::source::Source;

pub struct AlignColons {
    settings: aligner::Settings,
}

impl AlignColons {
    pub fn from_config(config: &Config) -> Self {
        Self {
            settings: aligner::Settings::from(&config.rules.align_colons)
                .with_singleton_subgroup_strip(),
        }
    }
}

impl Rule for AlignColons {
    fn apply(&self, source: &Source) -> Vec<Edit> {
        let mut visitor = Visitor {
            edits: Vec::new(),
            settings: self.settings,
            source,
        };
        visitor.visit_body(&source.ast().body);
        visitor.edits
    }

    fn name(&self) -> &'static str {
        "align-colons"
    }
}

struct Visitor<'a> {
    edits: Vec<Edit>,
    settings: aligner::Settings,
    source: &'a Source,
}

impl Visitor<'_> {
    /// Emits alignment edits for a qualifying group of members.
    fn emit(&mut self, members: &[aligner::Member]) {
        if members.len() < 2 || !aligner::distinct_lines(members) {
            return;
        }
        aligner::emit_group(self.source, members, self.settings, &mut self.edits);
    }

    fn process_class_fields(&mut self, body: &[Stmt]) {
        for group in colon_targets::class_field_groups(self.source, body) {
            self.emit(&group);
        }
    }

    fn process_dict(&mut self, d: &ExprDict) {
        if d.len() < 2 || self.source.intersects_comment(d.range()) {
            return;
        }
        self.emit(&colon_targets::dict_members(self.source, d));
    }

    fn process_docstring_args(&mut self, body: &[Stmt]) {
        self.emit(&colon_targets::docstring_args(self.source, body));
    }

    fn process_parameters(&mut self, params: &Parameters) {
        for group in colon_targets::parameter_groups(self.source, params) {
            self.emit(&group);
        }
    }
}

impl<'a> AstVisitor<'a> for Visitor<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Dict(d) = expr {
            self.process_dict(d);
        }
        walk_expr(self, expr);
    }

    fn visit_parameters(&mut self, parameters: &'a Parameters) {
        self.process_parameters(parameters);
        walk_parameters(self, parameters);
    }

    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::ClassDef(cd) => {
                self.process_class_fields(&cd.body);
                self.process_docstring_args(&cd.body);
            }
            Stmt::FunctionDef(fd) => self.process_docstring_args(&fd.body),
            _ => {}
        }
        walk_stmt(self, stmt);
    }
}
