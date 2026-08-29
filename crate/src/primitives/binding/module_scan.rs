//! The module-scope single-name assignments a lint rule scans, gathered
//! by a walk that skips a `def`, a `class`, and an `if TYPE_CHECKING:`
//! block while descending into every other compound body.

use ruff_python_ast::{
    ExceptHandler, Expr, ExprContext, ExprName, Stmt,
    statement_visitor::{StatementVisitor, walk_stmt},
    visitor::{self, Visitor},
};

use super::names::{
    bare_import_bound_name, from_import_bound_name, single_name_assignment, skips_module_scan,
};

/// A module-scope single-name assignment. The value is `None` for a bare
/// annotation (`X: int`).
pub(crate) struct ModuleAssignment<'src> {
    pub(crate) stmt: &'src Stmt,
    pub(crate) target: &'src ExprName,
    pub(crate) value: Option<&'src Expr>,
}

struct Walker<'src> {
    sites: Vec<ModuleAssignment<'src>>,
}

impl<'src> StatementVisitor<'src> for Walker<'src> {
    fn visit_stmt(&mut self, stmt: &'src Stmt) {
        if skips_module_scan(stmt) {
            return;
        }
        if let Some((target, value)) = single_name_assignment(stmt) {
            self.sites.push(ModuleAssignment {
                stmt,
                target,
                value,
            });
        }
        walk_stmt(self, stmt);
    }
}

/// Every module-scope single-name assignment in `body`, in source order.
pub(crate) fn module_assignments(body: &[Stmt]) -> Vec<ModuleAssignment<'_>> {
    let mut walker = Walker { sites: Vec::new() };
    walker.visit_body(body);
    walker.sites
}

/// Collects the names a module binds while executing one statement.
struct BindingWalker<'src> {
    names: Vec<&'src str>,
}

impl<'src> Visitor<'src> for BindingWalker<'src> {
    fn visit_stmt(&mut self, stmt: &'src Stmt) {
        match stmt {
            Stmt::FunctionDef(func) => {
                self.names.push(func.name.as_str());
                return;
            }
            Stmt::ClassDef(class) => {
                self.names.push(class.name.as_str());
                return;
            }
            Stmt::Import(import) => self
                .names
                .extend(import.names.iter().map(bare_import_bound_name)),
            Stmt::ImportFrom(import) => self
                .names
                .extend(import.names.iter().map(from_import_bound_name)),
            _ => {}
        }
        if skips_module_scan(stmt) {
            return;
        }
        visitor::walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'src Expr) {
        if let Expr::Name(name) = expr
            && matches!(name.ctx, ExprContext::Del | ExprContext::Store)
        {
            self.names.push(name.id.as_str());
        }
        visitor::walk_expr(self, expr);
    }

    fn visit_except_handler(&mut self, handler: &'src ExceptHandler) {
        let ExceptHandler::ExceptHandler(caught) = handler;
        if let Some(name) = &caught.name {
            self.names.push(name.as_str());
        }
        visitor::walk_except_handler(self, handler);
    }
}

/// Every name a module binds or unbinds when it executes `stmt`, covering the
/// store target of an assignment, an unpack, a `for`, a `with`, and a
/// walrus, each alias of an import, an `except ... as` name, and the
/// name of a definition. The walk descends the compound bodies a module
/// executes and stops at a `def`, a `class`, and an `if TYPE_CHECKING:`
/// block. A comprehension target binds in its own scope and is
/// collected all the same, so the set names more than the module binds.
/// A lambda parameter is an `Identifier` rather than a store-context
/// name and never enters it.
pub(crate) fn module_bound_names(stmt: &Stmt) -> Vec<&str> {
    let mut walker = BindingWalker { names: Vec::new() };
    walker.visit_stmt(stmt);
    walker.names
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::*;
    use crate::testing::parse;

    #[test]
    fn module_assignments_descends_compound_bodies_and_skips_scopes() {
        let source = parse(indoc! {"
            X = 1
            if flag:
                inner = 2
            def f():
                local = 3
            class C:
                attr = 4
            y: int
        "});
        let names: Vec<(&str, bool)> = module_assignments(&source.ast().body)
            .iter()
            .map(|site| (site.target.id.as_str(), site.value.is_some()))
            .collect();
        assert_eq!(names, vec![("X", true), ("inner", true), ("y", false)]);
    }
}
