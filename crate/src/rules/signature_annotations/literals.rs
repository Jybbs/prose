//! The call-site literals a parameter's inferred type reads, collected
//! per callee, beside whether a definition returns a value at all.

use ruff_python_ast::{
    Expr, Parameters, Stmt, StmtFunctionDef,
    helpers::ReturnStatementVisitor,
    visitor::{Visitor as AstVisitor, walk_expr},
};
use ruff_text_size::{Ranged, TextSize};
use rustc_hash::FxHashMap;

use super::CallArgs;
use crate::{
    primitives::{
        call_keywords::{keyword_args, module_call_params, resolve_call_params},
        walk,
    },
    source::Source,
};

struct LiteralCollector<'a> {
    map: CallArgs<'a>,
    resolved: FxHashMap<TextSize, &'a Parameters>,
    source: &'a Source,
}

impl<'a> AstVisitor<'a> for LiteralCollector<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Call(call) = expr
            && let Some(params) = resolve_call_params(call, &self.resolved)
            && let Some(keywords) = keyword_args(self.source, call, Some(params))
        {
            let bound = self.map.entry(params.start()).or_default();
            for arg in keywords.args {
                bound.entry(arg.name).or_default().push(arg.value);
            }
        }
        walk_expr(self, expr);
    }

    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        walk::walk_stmt(self, stmt);
    }
}

pub(super) fn call_argument_literals(source: &Source) -> CallArgs<'_> {
    let mut collector = LiteralCollector {
        map: FxHashMap::default(),
        resolved: module_call_params(source),
        source,
    };
    collector.visit_body(&source.ast().body);
    collector.map
}

/// True when `fd`'s own body returns a value, a `return` carrying an
/// expression other than a bare `None`. A nested scope's returns and a
/// generator's `yield`s do not count.
pub(super) fn returns_value(fd: &StmtFunctionDef) -> bool {
    let mut visitor = ReturnStatementVisitor::default();
    visitor.visit_body(&fd.body);
    visitor
        .returns
        .iter()
        .filter_map(|ret| ret.value.as_deref())
        .any(|value| !value.is_none_literal_expr())
}
