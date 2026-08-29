//! Collects the names an expression reads at evaluation time.

use super::*;

/// Accumulates load-context names through `eval_time_refs`, pruning
/// function and lambda bodies and skipping deferred annotations.
pub(super) struct EvalRefVisitor<'src> {
    pub(super) defer_annotations: bool,
    pub(super) names: Vec<&'src str>,
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
                walk_parameters(self, &func.parameters);
                if let Some(returns) = &func.returns {
                    self.visit_annotation(returns);
                }
            }
            _ => walk_stmt(self, stmt),
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
pub(crate) fn eval_time_refs(stmt: &Stmt, defer_annotations: bool) -> Vec<&str> {
    let mut visitor = EvalRefVisitor {
        defer_annotations,
        names: Vec::new(),
    };
    visitor.visit_stmt(stmt);
    visitor.names
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

/// Returns the name a subscript or attribute chain reads from, or
/// `None` when the chain roots in anything other than a name.
fn root_name(expr: &Expr) -> Option<&str> {
    match expr {
        Expr::Attribute(attr) => root_name(&attr.value),
        Expr::Name(name) => Some(name.id.as_str()),
        Expr::Subscript(subscript) => root_name(&subscript.value),
        _ => None,
    }
}

/// Collects the names `expr` reads through a subscript or an attribute,
/// the references whose result depends on the object's state at
/// evaluation time rather than on the binding alone.
pub(crate) fn observed_refs(expr: &Expr) -> Vec<&str> {
    let mut visitor = ObservedRefVisitor { names: Vec::new() };
    visitor.visit_expr(expr);
    visitor.names
}

/// Walks a lambda's parameter defaults, pruning its body, the eval-time
/// surface a lambda contributes when it binds.
pub(crate) fn walk_lambda_defaults<'a>(visitor: &mut impl AstVisitor<'a>, lambda: &'a ExprLambda) {
    if let Some(params) = lambda.parameters.as_deref() {
        walk_parameters(visitor, params);
    }
}
