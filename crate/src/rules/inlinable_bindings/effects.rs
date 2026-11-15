//! The points of a function body that run code, delete a name, or
//! store one, in the order Python evaluates them, which an inline
//! moving a value past them would reorder.

use ruff_python_ast::{
    Expr, ExprAttribute, ExprContext, ExprSubscript, Stmt, WithItem,
    visitor::{Visitor, walk_expr, walk_with_item},
};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::primitives::{
    effect::runs_code, range::blocks_span, scope::sub_bodies, tiering::walk_lambda_defaults,
    walk::walk_stmt,
};

/// Every point and statement span in one function body, with the arms
/// of each `if`, `match`, and conditional expression, of which one
/// runs. Points sit in evaluation order, leaving out each nested
/// function and lambda body, which runs nothing where it is written.
#[derive(Default)]
pub(super) struct Effects<'src> {
    arms: Vec<Vec<TextRange>>,
    points: Vec<Point<'src>>,
    statements: Vec<TextRange>,
}

impl<'src> Effects<'src> {
    pub(super) fn read(body: &'src [Stmt]) -> Self {
        let mut effects = Self::default();
        effects.visit_body(body);
        effects
    }

    /// Returns `true` when `at` sits in an arm that closes before `read`
    /// inside the `if`, `match`, or conditional expression that also
    /// holds `read`, so the path reaching `read` skips that arm, where
    /// `value` sits outside every arm of the set.
    fn exclusive(&self, value: TextRange, read: TextSize, at: TextRange) -> bool {
        self.arms.iter().any(|arms| {
            arms.iter().all(|arm| !arm.contains_range(value))
                && arms.last().is_some_and(|last| read < last.end())
                && arms
                    .iter()
                    .any(|arm| arm.contains_range(at) && arm.end() <= read)
        })
    }

    /// Returns `true` when `within` loads the name `id`.
    fn loads(&self, within: TextRange, id: &str) -> bool {
        self.points.iter().any(|point| match point {
            Point::Name(at, name, ExprContext::Load) => within.contains(*at) && *name == id,
            _ => false,
        })
    }

    /// The evaluation order of the name at `at`, `None` where no walked
    /// name starts there.
    fn position(&self, at: TextSize) -> Option<usize> {
        self.points
            .iter()
            .position(|point| matches!(point, Point::Name(start, ..) if *start == at))
    }

    /// Returns `true` when a point running after `value` and before
    /// `read`, on the path reaching `read`, runs code, deletes a name
    /// `value` reads, or holds one of `rebinds`, the writes of the
    /// names `value` reads. A read inside a nested function or lambda
    /// body counts the points ending before the statement holding it.
    pub(super) fn between(
        &self,
        value: TextRange,
        read: TextSize,
        mut rebinds: impl Iterator<Item = TextSize>,
    ) -> bool {
        let reached = self.position(read);
        let settled = self
            .points
            .iter()
            .rposition(|point| matches!(point, Point::Name(at, ..) if value.contains(*at)));
        let statement = self
            .statements
            .iter()
            .filter(|statement| statement.contains(read))
            .map(|statement| statement.start())
            .max()
            .unwrap_or(read);
        let ahead = |order: Option<usize>, at: TextRange| {
            !at.contains(read)
                && match order.zip(settled) {
                    Some((order, settled)) => order > settled && !value.contains_range(at),
                    None => at.start() >= value.end(),
                }
                && !self.exclusive(value, read, at)
                && match order.zip(reached) {
                    Some((order, reached)) => order < reached,
                    None => at.end() <= statement,
                }
        };
        rebinds.any(|write| ahead(self.position(write), TextRange::empty(write)))
            || self
                .points
                .iter()
                .enumerate()
                .any(|(order, point)| match point {
                    Point::Deletes(at, id) => ahead(Some(order), *at) && self.loads(value, id),
                    Point::Name(..) => false,
                    Point::Runs(at) => ahead(Some(order), *at),
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
                self.points.push(Point::Runs(expr.range()));
            }
            Expr::If(node) => self.arms.push(vec![node.body.range(), node.orelse.range()]),
            Expr::Lambda(lambda) => {
                walk_lambda_defaults(self, lambda);
                return;
            }
            Expr::Name(name) if name.ctx.is_del() => self
                .points
                .push(Point::Deletes(name.range, name.id.as_str())),
            Expr::Name(name) => {
                self.points
                    .push(Point::Name(name.start(), name.id.as_str(), name.ctx));
            }
            _ if runs_code(expr) => self.points.push(Point::Runs(expr.range())),
            _ => {}
        }
        walk_expr(self, expr);
    }

    fn visit_stmt(&mut self, stmt: &'src Stmt) {
        self.statements.push(stmt.range());
        match stmt {
            Stmt::AugAssign(node) if !node.target.is_name_expr() => {
                self.visit_expr(&node.target);
                self.visit_expr(&node.value);
                return;
            }
            Stmt::For(node) if node.is_async => self.points.push(Point::Runs(node.iter.range())),
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
            Stmt::If(_) | Stmt::Match(_) => self.arms.push(
                sub_bodies(stmt)
                    .into_iter()
                    .map(|(body, _)| blocks_span(body))
                    .collect(),
            ),
            Stmt::Import(_) | Stmt::ImportFrom(_) => self.points.push(Point::Runs(stmt.range())),
            _ => {}
        }
        walk_stmt(self, stmt);
    }

    fn visit_with_item(&mut self, item: &'src WithItem) {
        self.points.push(Point::Runs(item.range()));
        walk_with_item(self, item);
    }
}

/// One point of a function body as it runs, a name it reads or stores,
/// a name it deletes, or code it runs.
enum Point<'src> {
    Deletes(TextRange, &'src str),
    Name(TextSize, &'src str, ExprContext),
    Runs(TextRange),
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{first_def, parse};

    fn effect_between(src: &str) -> bool {
        let source = parse(src);
        let body = &first_def(&source).body;
        let value = TextRange::at(
            TextSize::try_from(src.find("n.size").expect("each case writes `n.size`"))
                .expect("the offset fits"),
            TextSize::of("n.size"),
        );
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
    fn between_counts_a_sibling_arm_once_the_write_sits_in_an_arm() {
        assert!(effect_between(
            "def f(n, xs):\n    for i in xs:\n        if i:\n            x = n.size\n        elif i == 1:\n            g()\n        else:\n            return x\n"
        ));
    }

    #[test]
    fn between_reads_a_store_target_ahead_of_the_value() {
        assert!(effect_between(
            "def f(n, d):\n    d[g()] = x = n.size\n    return x\n"
        ));
    }

    #[test]
    fn between_reads_the_header_of_the_block_holding_the_read() {
        assert!(effect_between(
            "def f(n):\n    x = n.size\n    if g():\n        return x\n"
        ));
    }

    #[rstest]
    #[case::elif_test_ahead_of_the_arm(
        "if c:\n        pass\n    elif g():\n        return x",
        true
    )]
    #[case::elif_test_holding_the_read("if c:\n        g()\n    elif x:\n        pass", false)]
    #[case::else_beside_a_call("if c:\n        g()\n    else:\n        return x", false)]
    #[case::except_after_a_try_body("try:\n        g()\n    except E:\n        return x", true)]
    #[case::for_else_after_the_loop("for i in xs:\n        g()\n    else:\n        return x", true)]
    #[case::match_case_beside_a_call(
        "match m:\n        case 1:\n            g()\n        case _:\n            return x",
        false
    )]
    #[case::match_guard_ahead_of_the_case(
        "match m:\n        case 1 if g():\n            pass\n        case _:\n            return x",
        true
    )]
    #[case::nested_else_beside_a_call(
        "if c:\n        pass\n    else:\n        if d:\n            g()\n        else:\n            return x",
        false
    )]
    #[case::rebinding_in_a_sibling_arm("if c:\n        n = 1\n    else:\n        return x", false)]
    fn between_skips_the_arms_beside_the_read(#[case] arms: &str, #[case] runs: bool) {
        let src = format!("def f(n, m, xs):\n    x = n.size\n    {arms}\n");
        assert_eq!(effect_between(&src), runs, "{arms}");
    }

    #[rstest]
    #[case::augmented_rebinding_after_the_read("n += x", false)]
    #[case::augmented_store_after_the_read("d[x] += g()", false)]
    #[case::branch_beside_the_read("return g() if c else x", false)]
    #[case::call_ahead_of_the_read("return g() + x", true)]
    #[case::call_after_the_read("return x + g()", false)]
    #[case::call_argument_ahead_of_the_read("return h(g(), x)", true)]
    #[case::call_holding_the_read("return g(x)", false)]
    #[case::comprehension_holding_its_first_iterable("return [g(i) for i in x]", false)]
    #[case::condition_ahead_of_the_read("return x if g() else 0", true)]
    #[case::conditional_ahead_of_the_read("return (g() if c else 0) + x", true)]
    #[case::short_circuit_ahead_of_the_read("return g() or x", true)]
    #[case::short_circuit_after_the_read("return x or g()", false)]
    #[case::store_value_ahead_of_its_target("d[x] = g()", true)]
    #[case::walrus_rebinding_ahead_of_the_read("return (n := 1) + x", true)]
    #[case::with_item_after_the_read("with open(x), g():\n        pass", false)]
    #[case::with_item_ahead_of_the_read("with g(), open(x):\n        pass", true)]
    fn between_reads_the_read_statement_in_evaluation_order(
        #[case] read: &str,
        #[case] runs: bool,
    ) {
        let src = format!("def f(n, c, d):\n    x = n.size\n    {read}\n");
        assert_eq!(effect_between(&src), runs, "{read}");
    }
}
