//! The module-scope single-name assignments a lint rule scans, gathered
//! by a walk that skips a `def`, a `class`, and an `if TYPE_CHECKING:`
//! block while descending into every other compound body.

use ruff_python_ast::{
    Comprehension, ExceptHandler, Expr, ExprContext, ExprName, Stmt, StmtClassDef, StmtFunctionDef,
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

/// Collects the names a module binds while executing one statement.
struct BindingWalker<'src> {
    names: Vec<&'src str>,
}

impl<'src> Visitor<'src> for BindingWalker<'src> {
    fn visit_comprehension(&mut self, comprehension: &'src Comprehension) {
        self.visit_expr(&comprehension.iter);
        for condition in &comprehension.ifs {
            self.visit_expr(condition);
        }
    }

    fn visit_except_handler(&mut self, handler: &'src ExceptHandler) {
        let ExceptHandler::ExceptHandler(caught) = handler;
        if let Some(name) = &caught.name {
            self.names.push(name.as_str());
        }
        visitor::walk_except_handler(self, handler);
    }

    fn visit_expr(&mut self, expr: &'src Expr) {
        if let Expr::Name(name) = expr
            && matches!(name.ctx, ExprContext::Del | ExprContext::Store)
        {
            self.names.push(name.id.as_str());
        }
        visitor::walk_expr(self, expr);
    }

    fn visit_stmt(&mut self, stmt: &'src Stmt) {
        match stmt {
            Stmt::ClassDef(StmtClassDef { name, .. })
            | Stmt::FunctionDef(StmtFunctionDef { name, .. }) => {
                self.names.push(name.as_str());
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

/// Every name a module binds or unbinds when it executes `stmt`: the
/// targets of an assignment, an unpack, a `for`, a `with`, and a
/// walrus, each alias of an import, an `except ... as` name, and the
/// name of a definition. The walk descends the compound bodies a module
/// executes and stops at a `def`, a `class`, and an `if TYPE_CHECKING:`
/// block. A comprehension target stays out, whereas a walrus inside one
/// enters.
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

    #[test]
    fn module_bound_names_collects_an_except_alias() {
        let source = parse(indoc! {"
            try:
                import fast
            except ImportError as err:
                fast = None
        "});
        let names = module_bound_names(&source.ast().body[0]);
        assert!(names.contains(&"err"), "the except alias binds");
        assert!(
            names.contains(&"fast"),
            "the import and the fallback both bind"
        );
    }

    #[test]
    fn module_bound_names_covers_every_binding_form() {
        let source = parse(indoc! {"
            for key, value in pairs:
                with open(path) as handle:
                    import json as codec
                    from os import sep as divider
                    del stale
                    total = (running := 0)
                try:
                    pass
                except OSError as failure:
                    pass
                class Inner:
                    skipped = 1
                def helper():
                    hidden = 2
        "});
        let mut names = module_bound_names(&source.ast().body[0]);
        names.sort_unstable();
        assert_eq!(
            names,
            vec![
                "Inner", "codec", "divider", "failure", "handle", "helper", "key", "running",
                "stale", "total", "value",
            ]
        );
    }

    #[test]
    fn module_bound_names_drops_a_comprehension_target_and_keeps_its_walrus() {
        let source = parse("totals = [(seen := item) for item in rows if (kept := item)]");
        let mut names = module_bound_names(&source.ast().body[0]);
        names.sort_unstable();
        assert_eq!(names, vec!["kept", "seen", "totals"]);
    }

    #[test]
    fn module_bound_names_skips_a_type_checking_block() {
        let source = parse("if TYPE_CHECKING:\n    from pkg import Only\n");
        assert!(module_bound_names(&source.ast().body[0]).is_empty());
    }
}
