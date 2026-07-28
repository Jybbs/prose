//! Classifies whether evaluating a value runs code when it binds. An
//! inert value only reads names and builds the result, whereas an
//! effectful one carries a call, a comprehension, an `await`, or a
//! notebook escape command somewhere in its tree.

use ruff_python_ast::{
    Expr,
    visitor::{Visitor, walk_expr},
};

use crate::primitives::tiering::walk_lambda_defaults;

/// Walks the evaluation-time surface of a value, pruning each lambda
/// body, and flips `effectful` on a call, comprehension, `await`, or
/// notebook escape command. `ruff_python_ast::helpers::contains_effect`
/// also classifies a subscript, an operator expression, and a walrus as
/// effectful and walks lambda bodies, so it over-pins against this
/// narrower split.
struct EffectVisitor {
    effectful: bool,
}

impl<'src> Visitor<'src> for EffectVisitor {
    fn visit_expr(&mut self, expr: &'src Expr) {
        match expr {
            Expr::Await(_)
            | Expr::Call(_)
            | Expr::DictComp(_)
            | Expr::Generator(_)
            | Expr::IpyEscapeCommand(_)
            | Expr::ListComp(_)
            | Expr::SetComp(_) => self.effectful = true,
            Expr::Lambda(lambda) => walk_lambda_defaults(self, lambda),
            _ => walk_expr(self, expr),
        }
    }
}

/// True when evaluating `value` runs code beyond reading names, meaning
/// it carries a call, a comprehension, an `await`, or a notebook escape
/// command outside a lambda body.
pub(crate) fn value_is_effectful(value: &Expr) -> bool {
    let mut visitor = EffectVisitor { effectful: false };
    visitor.visit_expr(value);
    visitor.effectful
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{first_value, notebook, parse};

    #[test]
    fn value_is_effectful_reads_a_notebook_escape_command() {
        let source = notebook(&["SHELL = !ls\n"]);
        assert!(value_is_effectful(first_value(&source)));
    }

    #[rstest]
    #[case("call()", true)]
    #[case("await fetch()", true)]
    #[case("[make(), 1]", true)]
    #[case("[n for n in seq]", true)]
    #[case("{n for n in seq}", true)]
    #[case("{k: v for k in seq}", true)]
    #[case("(n for n in seq)", true)]
    #[case("lambda k=make(): k", true)]
    #[case("42", false)]
    #[case("value", false)]
    #[case("obj.attr", false)]
    #[case("table[key]", false)]
    #[case("BASE * 2", false)]
    #[case("a if cond else b", false)]
    #[case("[a, b, c]", false)]
    #[case("(n := 5)", false)]
    #[case("lambda row: row.compute()", false)]
    #[case("lambda: compute()", false)]
    fn value_is_effectful_splits_effectful_from_inert(
        #[case] value_src: &str,
        #[case] expected: bool,
    ) {
        let source = parse(&format!("X = {value_src}\n"));
        assert_eq!(
            value_is_effectful(first_value(&source)),
            expected,
            "{value_src}"
        );
    }
}
