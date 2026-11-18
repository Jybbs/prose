//! Collects the names an expression reads at evaluation time.

use std::mem;

use ruff_python_ast::{
    Expr, ExprLambda, Stmt,
    visitor::{Visitor as AstVisitor, walk_expr, walk_parameters},
};
use rustc_hash::FxHashMap;

use super::Annotations;
use crate::primitives::walk::walk_stmt;

/// Accumulates load-context names through `eval_time_refs`, pruning
/// function and lambda bodies and skipping each annotation `evaluates`
/// leaves unread.
struct EvalRefVisitor<'src> {
    annotations: Annotations,
    evaluates: bool,
    names: Vec<&'src str>,
}

impl EvalRefVisitor<'_> {
    /// A visitor under `annotations`, reading the annotations outside any
    /// definition only under [`Annotations::Eager`].
    fn new(annotations: Annotations) -> Self {
        Self {
            annotations,
            evaluates: annotations == Annotations::Eager,
            names: Vec::new(),
        }
    }

    /// Runs `visit` with `evaluates` set as given, restoring the outer
    /// setting after.
    fn visit_evaluating(&mut self, evaluates: bool, visit: impl FnOnce(&mut Self)) {
        let outer = mem::replace(&mut self.evaluates, evaluates);
        visit(self);
        self.evaluates = outer;
    }
}

impl<'src> AstVisitor<'src> for EvalRefVisitor<'src> {
    fn visit_annotation(&mut self, annotation: &'src Expr) {
        if self.evaluates {
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
            Stmt::ClassDef(class) => {
                self.visit_evaluating(self.annotations.read_by(class), |visitor| {
                    walk_stmt(visitor, stmt);
                });
            }
            Stmt::FunctionDef(func) => {
                for decorator in &func.decorator_list {
                    self.visit_expr(&decorator.expression);
                }
                if let Some(type_params) = &func.type_params {
                    self.visit_type_params(type_params);
                }
                let reads = self.annotations == Annotations::Eager
                    || (self.annotations == Annotations::Lazy && !func.decorator_list.is_empty());
                self.visit_evaluating(reads, |visitor| {
                    walk_parameters(visitor, &func.parameters);
                    if let Some(returns) = &func.returns {
                        visitor.visit_annotation(returns);
                    }
                });
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
    let mut visitor = EvalRefVisitor::new(Annotations::Stringified);
    visitor.visit_expr(expr);
    visitor.names
}

/// Collects the load-context names in a definition's evaluation-time
/// surface: its decorators, base classes and class keywords, parameter
/// defaults, non-deferred annotations, and the top level of a class
/// body, descending into nested definitions but pruning every function
/// and lambda body. Each annotation counts only where `annotations`
/// runs it as the definition binds.
pub(super) fn eval_time_refs(stmt: &Stmt, annotations: Annotations) -> Vec<&str> {
    let mut visitor = EvalRefVisitor::new(annotations);
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

/// Collects the names each `type` alias of `body` evaluates its value
/// over, keyed by the alias name. Reading an alias as the module runs
/// can evaluate that value through the alias's `__value__`, so a reader
/// of the alias reads each of these names as well.
pub(crate) fn type_alias_values(body: &[Stmt]) -> FxHashMap<&str, Vec<&str>> {
    body.iter()
        .filter_map(|stmt| {
            let alias = stmt.as_type_alias_stmt()?;
            Some((
                alias.name.as_name_expr()?.id.as_str(),
                eval_refs(&alias.value),
            ))
        })
        .collect()
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
    use rstest::rstest;
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
        let collected: FxHashSet<&str> = eval_time_refs(&source.ast().body[0], Annotations::Eager)
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
        let collected: FxHashSet<&str> = eval_time_refs(&source.ast().body[0], Annotations::Eager)
            .into_iter()
            .collect();
        assert_eq!(collected, FxHashSet::from_iter(["SeedRef"]));
    }

    #[rstest]
    #[case::bare_class("class Probe:\n    field: AnnotRef\n", &[])]
    #[case::decorated_class("@DecoRef\nclass Probe:\n    field: AnnotRef\n", &["AnnotRef", "DecoRef"])]
    #[case::keyword_class("class Probe(metaclass=MetaRef):\n    field: AnnotRef\n", &["AnnotRef", "MetaRef"])]
    #[case::empty_parentheses("class Probe():\n    field: AnnotRef\n", &[])]
    #[case::nested_under_a_bare_class(
        "class Probe:\n    class Inner(BaseRef):\n        field: AnnotRef\n",
        &["AnnotRef", "BaseRef"],
    )]
    #[case::bare_class_nested_under_a_based_one(
        "class Probe(BaseRef):\n    class Inner:\n        field: AnnotRef\n",
        &["BaseRef"],
    )]
    #[case::field_after_a_method_under_a_based_class(
        "class Probe(BaseRef):\n    def method(self): ...\n    field: AnnotRef\n",
        &["AnnotRef", "BaseRef"],
    )]
    #[case::function_signature("def probe(p: ParamRef) -> ReturnRef: ...\n", &[])]
    #[case::decorated_function(
        "@DecoRef\ndef probe(p: ParamRef) -> ReturnRef: ...\n",
        &["DecoRef", "ParamRef", "ReturnRef"],
    )]
    #[case::module_annotation("field: AnnotRef = ValueRef\n", &["ValueRef"])]
    fn eval_time_refs_reads_a_lazy_annotation_only_under_a_header_running_code(
        #[case] src: &str,
        #[case] expected: &[&str],
    ) {
        let source = parse(src);
        let collected: FxHashSet<&str> = eval_time_refs(&source.ast().body[0], Annotations::Lazy)
            .into_iter()
            .collect();
        assert_eq!(collected, expected.iter().copied().collect());
    }

    #[test]
    fn eval_time_refs_reads_a_type_statement() {
        let source = parse(indoc! {"
            type Alias[T: BoundRef] = list[ValueRef]

            if FlagRef:
                type Nested = NestedRef
        "});
        let collected: FxHashSet<&str> = source
            .ast()
            .body
            .iter()
            .flat_map(|stmt| eval_time_refs(stmt, Annotations::Eager))
            .collect();
        assert_eq!(
            collected,
            FxHashSet::from_iter(["BoundRef", "FlagRef", "NestedRef", "ValueRef", "list"]),
        );
    }

    #[rstest]
    #[case::stringified(Annotations::Stringified, &["BaseRef", "DefaultRef"])]
    #[case::lazy_under_a_base(Annotations::Lazy, &["AnnotRef", "BaseRef", "DefaultRef"])]
    fn eval_time_refs_skips_annotations_the_class_creation_leaves_unread(
        #[case] annotations: Annotations,
        #[case] expected: &[&str],
    ) {
        let source = parse(indoc! {"
            class Probe(BaseRef):
                field: AnnotRef

                def method(self, p: ParamRef = DefaultRef) -> ReturnRef: ...
        "});
        let collected: FxHashSet<&str> = eval_time_refs(&source.ast().body[0], annotations)
            .into_iter()
            .collect();
        assert_eq!(collected, expected.iter().copied().collect());
    }
}
