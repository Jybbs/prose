//! The module-scope single-name assignments a lint rule scans, gathered
//! by a walk that skips a `def`, a `class`, and an `if TYPE_CHECKING:`
//! block while descending into every other compound body.

use ruff_python_ast::{
    Expr, ExprName, Stmt,
    statement_visitor::{StatementVisitor, walk_stmt},
};

use super::names::{single_name_assignment, skips_module_scan};

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
