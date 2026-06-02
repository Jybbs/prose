//! Removes the trailing comma after the last element of any
//! bracketed container. The covered shapes are function calls,
//! function signatures, class base lists, generic-syntax
//! type-parameter lists on `def` / `class` / `type`, dict literals,
//! list literals, and set literals. Tuples and any container whose
//! final non-trivia token is not a comma are left unchanged.

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    Arguments, Expr, Stmt, TypeParams,
    visitor::{Visitor, walk_arguments, walk_expr, walk_stmt, walk_type_params},
};
use ruff_text_size::{Ranged, TextRange};

use crate::{
    config::Config,
    primitives::edit::singleton_groups,
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct StripTrailingCommas;

impl StripTrailingCommas {
    pub(crate) fn from_config(_: &Config) -> Self {
        Self
    }
}

impl Rule for StripTrailingCommas {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let mut visitor = Stripper {
            edits: Vec::new(),
            source,
        };
        visitor.visit_body(&source.ast().body);
        singleton_groups(visitor.edits)
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

struct Stripper<'a> {
    edits: Vec<Edit>,
    source: &'a Source,
}

impl Stripper<'_> {
    /// Pushes a deletion edit for the trailing comma in `container`
    /// when its final non-trivia token before the closing bracket
    /// is a comma.
    fn process_container(&mut self, container: TextRange) {
        self.edits.extend(
            self.source
                .trailing_comma(container)
                .map(Edit::range_deletion),
        );
    }
}

impl<'a> Visitor<'a> for Stripper<'a> {
    fn visit_arguments(&mut self, arguments: &'a Arguments) {
        self.process_container(arguments.range());
        walk_arguments(self, arguments);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if matches!(expr, Expr::Dict(_) | Expr::List(_) | Expr::Set(_)) {
            self.process_container(expr.range());
        }
        walk_expr(self, expr);
    }

    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::FunctionDef(fd) = stmt {
            self.process_container(fd.parameters.range());
        }
        walk_stmt(self, stmt);
    }

    fn visit_type_params(&mut self, type_params: &'a TypeParams) {
        self.process_container(type_params.range());
        walk_type_params(self, type_params);
    }
}
