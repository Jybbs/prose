//! The points of a function body that run code or delete a name as the
//! body executes, which an inline moving a value past them would
//! reorder.

use ruff_python_ast::{
    Expr, ExprAttribute, ExprSubscript, Stmt,
    visitor::{Visitor, walk_expr},
};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::primitives::{effect::runs_code, tiering::walk_lambda_defaults, walk::walk_stmt};

/// Every effect, name read, and statement span in one function body,
/// leaving out each nested function and lambda body, which runs nothing
/// where it is written.
#[derive(Default)]
pub(super) struct Effects<'src> {
    loads: Vec<(TextSize, &'src str)>,
    sites: Vec<(TextRange, Effect<'src>)>,
    statements: Vec<TextRange>,
}

impl<'src> Effects<'src> {
    pub(super) fn read(body: &'src [Stmt]) -> Self {
        let mut effects = Self::default();
        effects.visit_body(body);
        effects
    }

    /// Returns `true` when a statement running after `value` and before
    /// the statement holding `read` runs code, deletes a name `value`
    /// reads, or holds one of `rebinds`, the writes of the names
    /// `value` reads.
    pub(super) fn between(
        &self,
        value: TextRange,
        read: TextSize,
        mut rebinds: impl Iterator<Item = TextSize>,
    ) -> bool {
        let end = self
            .statements
            .iter()
            .filter(|statement| statement.contains(read))
            .map(|statement| statement.start())
            .max()
            .unwrap_or(read);
        let window = TextRange::new(value.end(), end.max(value.end()));
        rebinds.any(|write| window.contains(write))
            || self
                .sites
                .iter()
                .filter(|(site, _)| window.contains_range(*site))
                .any(|(_, effect)| match effect {
                    Effect::Deletes(name) => self
                        .loads
                        .iter()
                        .any(|(at, read)| value.contains(*at) && read == name),
                    Effect::Runs => true,
                })
    }
}

impl<'src> Visitor<'src> for Effects<'src> {
    fn visit_expr(&mut self, expr: &'src Expr) {
        match expr {
            Expr::Attribute(ExprAttribute { ctx, .. })
            | Expr::Subscript(ExprSubscript { ctx, .. })
                if !ctx.is_load() =>
            {
                self.sites.push((expr.range(), Effect::Runs));
            }
            Expr::Lambda(lambda) => {
                walk_lambda_defaults(self, lambda);
                return;
            }
            Expr::Name(name) if name.ctx.is_load() => {
                self.loads.push((name.start(), name.id.as_str()));
            }
            Expr::Name(name) if name.ctx.is_del() => self
                .sites
                .push((name.range, Effect::Deletes(name.id.as_str()))),
            _ if runs_code(expr) => self.sites.push((expr.range(), Effect::Runs)),
            _ => {}
        }
        walk_expr(self, expr);
    }

    fn visit_stmt(&mut self, stmt: &'src Stmt) {
        self.statements.push(stmt.range());
        match stmt {
            Stmt::Import(_) | Stmt::ImportFrom(_) => self.sites.push((stmt.range(), Effect::Runs)),
            Stmt::For(node) if node.is_async => self.sites.push((node.iter.range(), Effect::Runs)),
            Stmt::FunctionDef(node) => {
                for decorator in &node.decorator_list {
                    self.visit_expr(&decorator.expression);
                }
                for default in node
                    .parameters
                    .iter_non_variadic_params()
                    .filter_map(|parameter| parameter.default())
                {
                    self.visit_expr(default);
                }
                return;
            }
            Stmt::With(node) => self
                .sites
                .extend(node.items.iter().map(|item| (item.range(), Effect::Runs))),
            _ => {}
        }
        walk_stmt(self, stmt);
    }
}

/// The effect one point of a function body has as it runs.
enum Effect<'src> {
    Deletes(&'src str),
    Runs,
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{first_def, parse};

    fn effect_between(src: &str) -> bool {
        let source = parse(src);
        let body = &first_def(&source).body;
        let value = body[0]
            .as_assign_stmt()
            .expect("each case opens on an assignment")
            .value
            .range();
        let read = src.rfind('x').expect("the read names `x`");
        Effects::read(body).between(
            value,
            TextSize::try_from(read).expect("the offset fits"),
            source
                .binding_analysis()
                .writes_of_names_read_within(&source.ast().body[0], value),
        )
    }

    #[rstest]
    #[case::call_ahead_of_the_read("return g() + x")]
    #[case::call_holding_the_read("return g(x)")]
    fn between_leaves_the_read_statement_out(#[case] read: &str) {
        let src = format!("def f(n):\n    x = n.size\n    {read}\n");
        assert!(!effect_between(&src));
    }

    #[rstest]
    #[case::assignment_of_a_literal("y = 1", false)]
    #[case::assignment_of_another_name("y = m", false)]
    #[case::async_for("async for i in xs:\n        pass", true)]
    #[case::attribute_store("n.size = 0", true)]
    #[case::await_expression("await g()", true)]
    #[case::call("g()", true)]
    #[case::class_body_call("class C:\n        y = g()", true)]
    #[case::class_rebinding_a_name_the_value_reads("class n:\n        pass", true)]
    #[case::comprehension("ys = [i for i in m]", true)]
    #[case::delete_of_a_name_the_value_reads("del n", true)]
    #[case::delete_of_another_name("del m", false)]
    #[case::except_rebinding_a_name_the_value_reads(
        "try:\n        pass\n    except E as n:\n        pass",
        true
    )]
    #[case::import("import json", true)]
    #[case::import_from("from os import path", true)]
    #[case::lambda_body_call("h = lambda: g()", false)]
    #[case::lambda_default_call("h = lambda a=g(): a", true)]
    #[case::loop_holding_a_call("for i in xs:\n        g(i)", true)]
    #[case::loop_without_a_call("for i in xs:\n        y = i", false)]
    #[case::match_capture_rebinding_a_name_the_value_reads(
        "match m:\n        case n:\n            pass",
        true
    )]
    #[case::nested_def_body_call("def h():\n        g()", false)]
    #[case::nested_def_decorator_call("@g()\n    def h():\n        pass", true)]
    #[case::nested_def_default_call("def h(a=g()):\n        pass", true)]
    #[case::rebinding_a_name_the_value_reads("n = 1", true)]
    #[case::subscript_delete("del n[0]", true)]
    #[case::with_entry("with m:\n        pass", true)]
    #[case::yield_expression("yield", true)]
    #[case::yield_from("yield from xs", true)]
    fn between_reads_each_statement_shape(#[case] between: &str, #[case] runs: bool) {
        let src = format!("async def f(n, m, xs):\n    x = n.size\n    {between}\n    return x\n");
        assert_eq!(effect_between(&src), runs, "{between}");
    }

    #[test]
    fn between_reads_the_header_of_the_block_holding_the_read() {
        assert!(effect_between(
            "def f(n):\n    x = n.size\n    if g():\n        return x\n"
        ));
    }
}
