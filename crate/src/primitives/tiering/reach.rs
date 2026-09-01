//! The module-scope names a call into each definition can reach, read
//! off the binding table and widened along the call edges between
//! definitions.

use std::{collections::VecDeque, slice};

use itertools::Itertools;

use ruff_python_ast::{
    Expr, Stmt, StmtClassDef, StmtFunctionDef,
    helpers::any_over_body,
    visitor::{Visitor as AstVisitor, walk_expr},
};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::{FxHashMap, FxHashSet};

use super::refs::root_name;
use crate::primitives::{binding::BindingAnalysis, group_map};

/// Per module-level definition, the module-scope names a call into it
/// can reach.
pub(crate) type CallReach<'src> = FxHashMap<&'src str, FxHashSet<&'src str>>;

/// Per module-level definition, every module-scope name a call into it
/// can reach, being the module bindings `analysis` resolves inside it
/// widened along call edges between definitions to a fixed point. A
/// class contributes every name its whole body reads.
pub(crate) fn call_reachable<'src>(
    analysis: &'src BindingAnalysis,
    body: &'src [Stmt],
) -> CallReach<'src> {
    let defs: Vec<(&str, &Stmt)> = body
        .iter()
        .filter_map(|stmt| match stmt {
            Stmt::ClassDef(StmtClassDef { name, .. })
            | Stmt::FunctionDef(StmtFunctionDef { name, .. }) => Some((name.as_str(), stmt)),
            _ => None,
        })
        .collect();
    let ranges: Vec<TextRange> = defs.iter().map(|(_, stmt)| stmt.range()).collect();
    let mut reads: FxHashMap<&str, FxHashSet<&str>> = defs
        .iter()
        .map(|(name, _)| *name)
        .zip(analysis.module_names_read_within(&ranges))
        .collect();
    let mut edges: FxHashMap<&str, Vec<&str>> = defs
        .iter()
        .map(|(name, stmt)| (*name, called_names(stmt)))
        .collect();
    for edge in edges.values_mut() {
        edge.retain(|callee| reads.contains_key(callee));
    }
    let callers = group_map(edges.iter().flat_map(|(&name, callees)| {
        callees
            .iter()
            .filter(move |callee| **callee != name)
            .map(move |&callee| (callee, name))
    }));
    let mut queue: VecDeque<&str> = reads.keys().copied().collect();
    while let Some(callee) = queue.pop_front() {
        for &caller in callers.get(callee).into_iter().flatten() {
            let [Some(reached), Some(set)] = reads.get_disjoint_mut([callee, caller]) else {
                continue;
            };
            let before = set.len();
            set.extend(reached.iter().copied());
            if set.len() > before {
                queue.push_back(caller);
            }
        }
    }
    reads
}

/// The chain a name roots that `expr` runs, covering a call and a
/// subscript alike, since `__class_getitem__` runs a class body the way
/// `__call__` does. `None` for every other expression.
fn invoked(expr: &Expr) -> Option<&Expr> {
    match expr {
        Expr::Call(call) => Some(&call.func),
        Expr::Subscript(subscript) => Some(&subscript.value),
        _ => None,
    }
}

/// Every name `stmt` runs, an attribute or subscript chain contributing
/// the name it roots in, each name once. A subscripted callee roots the
/// same name twice, once for the subscript and once for the call.
pub(super) fn called_names(stmt: &Stmt) -> Vec<&str> {
    struct Calls<'src>(Vec<&'src str>);
    impl<'src> AstVisitor<'src> for Calls<'src> {
        fn visit_expr(&mut self, expr: &'src Expr) {
            if let Some(name) = invoked(expr).and_then(root_name) {
                self.0.push(name);
            }
            walk_expr(self, expr);
        }
    }
    let mut calls = Calls(Vec::new());
    calls.visit_stmt(stmt);
    calls.0.into_iter().unique().collect()
}

/// True where `stmt` runs anything a name roots.
pub(super) fn calls_a_name(stmt: &Stmt) -> bool {
    any_over_body(slice::from_ref(stmt), |expr| {
        invoked(expr).and_then(root_name).is_some()
    })
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::*;
    use crate::testing::parse;

    #[test]
    fn call_reachable_covers_a_name_a_constructor_reaches_through_self() {
        let source = parse(indoc! {"
            def zzz_helper():
                return 1

            class C:
                def __init__(self):
                    self.value = self.compute()

                def compute(self):
                    return zzz_helper()
        "});
        let reachable = call_reachable(source.binding_analysis(), &source.ast().body);
        assert!(
            reachable["C"].contains("zzz_helper"),
            "instantiating C runs compute through a self call, evaluating zzz_helper"
        );
    }

    #[test]
    fn call_reachable_follows_call_edges_to_a_fixed_point() {
        let source = parse(concat!(
            "LEAF_READ = 1\n\n\n",
            "def leaf():\n    return LEAF_READ\n\n\n",
            "def middle():\n    return leaf()\n\n\n",
            "def top():\n    return middle()\n"
        ));
        let reach = call_reachable(source.binding_analysis(), &source.ast().body);
        assert!(
            reach["middle"].contains("LEAF_READ"),
            "one call edge reaches the leaf read"
        );
        assert!(
            reach["top"].contains("LEAF_READ"),
            "two call edges reach the leaf read"
        );
    }

    #[test]
    fn called_names_roots_an_attribute_call_in_its_receiver() {
        let source = parse("Coroutine.register(coroutine)\nhandlers[0](event)\nrun()\n");
        let called: Vec<&str> = source.ast().body.iter().flat_map(called_names).collect();
        assert_eq!(called, vec!["Coroutine", "handlers", "run"]);
    }
}
