//! Collects the names an expression reads at evaluation time.

use ruff_python_ast::{
    Expr, ExprLambda, Stmt,
    visitor::{Visitor as AstVisitor, walk_expr, walk_parameters},
};

use crate::primitives::walk::walk_stmt;

/// Accumulates load-context names through `eval_time_refs`, pruning
/// function and lambda bodies and skipping deferred annotations.
struct EvalRefVisitor<'src> {
    defer_annotations: bool,
    names: Vec<&'src str>,
}

impl<'src> AstVisitor<'src> for EvalRefVisitor<'src> {
    fn visit_annotation(&mut self, annotation: &'src Expr) {
        if !self.defer_annotations {
            self.visit_expr(annotation);
        }
    }

    fn visit_expr(&mut self, expr: &'src Expr) {
        match expr {
            Expr::Lambda(lambda) => walk_lambda_defaults(self, lambda),
            Expr::Name(name) if name.ctx.is_load() => self.names.push(name.id.as_str()),
            _ => walk_expr(self, expr),
        }
    }

    fn visit_stmt(&mut self, stmt: &'src Stmt) {
        match stmt {
            Stmt::AnnAssign(ann) => {
                self.visit_annotation(&ann.annotation);
                if let Some(value) = &ann.value {
                    self.visit_expr(value);
                }
            }
            Stmt::FunctionDef(func) => {
                for decorator in &func.decorator_list {
                    self.visit_expr(&decorator.expression);
                }
                if let Some(type_params) = &func.type_params {
                    self.visit_type_params(type_params);
                }
                walk_parameters(self, &func.parameters);
                if let Some(returns) = &func.returns {
                    self.visit_annotation(returns);
                }
            }
            _ => walk_stmt(self, stmt),
        }
    }
}

/// Accumulates the names an expression reads through a subscript or an
/// attribute, pruning lambda bodies.
struct ObservedRefVisitor<'src> {
    names: Vec<&'src str>,
}

impl<'src> AstVisitor<'src> for ObservedRefVisitor<'src> {
    fn visit_expr(&mut self, expr: &'src Expr) {
        match expr {
            Expr::Lambda(lambda) => walk_lambda_defaults(self, lambda),
            Expr::Attribute(_) | Expr::Subscript(_) => {
                if let Some(name) = root_name(expr) {
                    self.names.push(name);
                }
                walk_expr(self, expr);
            }
            _ => walk_expr(self, expr),
        }
    }
}

/// Collects the load-context names in `expr`, pruning every function
/// and lambda body, the reference set a module constant's value or
/// annotation contributes to the hoist graph.
pub(crate) fn eval_refs(expr: &Expr) -> Vec<&str> {
    let mut visitor = EvalRefVisitor {
        defer_annotations: true,
        names: Vec::new(),
    };
    visitor.visit_expr(expr);
    visitor.names
}

/// Collects the load-context names in a definition's evaluation-time
/// surface: its decorators, base classes and class keywords, parameter
/// defaults, non-deferred annotations, and the top level of a class
/// body, descending into nested definitions but pruning every function
/// and lambda body. Annotation positions are skipped when
/// `defer_annotations` holds.
pub(super) fn eval_time_refs(stmt: &Stmt, defer_annotations: bool) -> Vec<&str> {
    let mut visitor = EvalRefVisitor {
        defer_annotations,
        names: Vec::new(),
    };
    visitor.visit_stmt(stmt);
    visitor.names
}

/// Collects the names `expr` reads through a subscript or an attribute.
pub(crate) fn observed_refs(expr: &Expr) -> Vec<&str> {
    let mut visitor = ObservedRefVisitor { names: Vec::new() };
    visitor.visit_expr(expr);
    visitor.names
}

/// Returns the name a subscript or attribute chain reads from, or
/// `None` when the chain roots in anything other than a name.
pub(super) fn root_name(expr: &Expr) -> Option<&str> {
    match expr {
        Expr::Attribute(attr) => root_name(&attr.value),
        Expr::Name(name) => Some(name.id.as_str()),
        Expr::Subscript(subscript) => root_name(&subscript.value),
        _ => None,
    }
}

/// Walks a lambda's parameter defaults, pruning its body, the eval-time
/// surface a lambda contributes when it binds.
pub(crate) fn walk_lambda_defaults<'a>(visitor: &mut impl AstVisitor<'a>, lambda: &'a ExprLambda) {
    if let Some(params) = lambda.parameters.as_deref() {
        walk_parameters(visitor, params);
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use rustc_hash::FxHashSet;

    use super::*;
    use crate::testing::parse;

    #[test]
    fn eval_time_refs_collects_eval_surface_and_skips_bodies() {
        let source = parse(indoc! {"
            class Probe(BaseRef):
                field: AnnotRef

                def method(self, p: ParamRef = DefaultRef) -> ReturnRef:
                    return BodyRef
        "});
        let collected: FxHashSet<&str> = eval_time_refs(&source.ast().body[0], false)
            .into_iter()
            .collect();
        assert_eq!(
            collected,
            FxHashSet::from_iter(["AnnotRef", "BaseRef", "DefaultRef", "ParamRef", "ReturnRef"]),
        );
    }

    #[test]
    fn eval_time_refs_prunes_a_lambda_body() {
        let source = parse("class Probe:\n    factory = lambda seed=SeedRef: BodyRef\n");
        let collected: FxHashSet<&str> = eval_time_refs(&source.ast().body[0], false)
            .into_iter()
            .collect();
        assert_eq!(collected, FxHashSet::from_iter(["SeedRef"]));
    }

    #[test]
    fn eval_time_refs_skips_annotations_when_deferred() {
        let source = parse(indoc! {"
            class Probe(BaseRef):
                field: AnnotRef

                def method(self, p: ParamRef = DefaultRef) -> ReturnRef: ...
        "});
        let collected: FxHashSet<&str> = eval_time_refs(&source.ast().body[0], true)
            .into_iter()
            .collect();
        assert_eq!(collected, FxHashSet::from_iter(["BaseRef", "DefaultRef"]));
    }
}
