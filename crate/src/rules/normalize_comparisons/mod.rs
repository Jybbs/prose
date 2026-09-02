//! Normalizes a two-operand comparison so it states its check outright.
//! A `==` or `!=` test against `None` becomes `is` or `is not`, a test
//! whose constant side leads flips so the variable leads, and a leading
//! `not` folds into the `in` or `is` it negates. A test against `True`
//! or `False` is flagged rather than rewritten, because dropping the
//! literal changes the test for a non-boolean operand. A chained
//! comparison is left as written, as is one inside an f-string or
//! t-string replacement field.

use ruff_diagnostics::Edit;
use ruff_python_ast::{AnyNodeRef, CmpOp, ExprCompare};

use ruff_text_size::Ranged;

use crate::{
    config::Config,
    diagnostics::Diagnostic,
    primitives::walk::{Descent, filter_map_over_exprs, filter_map_over_parented_exprs},
    rule::{Rule, RuleId},
    source::Source,
};

mod constancy;
mod lint;
mod plan;
mod render;

use constancy::constancy;
use lint::boolean_lint;
use plan::{Plan, Test};
use render::{edits, identity_op, negating_parent, reflected};

#[derive(Debug)]
pub(crate) struct NormalizeComparisons {
    identity: bool,
    negation: bool,
    operand_order: bool,
}

impl NormalizeComparisons {
    pub(crate) const MESSAGE: &'static str = "normalize a comparison to state its check directly";

    pub(crate) const PRESERVES_BINDINGS: bool = true;

    pub(crate) fn from_config(config: &Config) -> Self {
        let facets = &config.rules.normalize_comparisons;
        Self {
            identity: facets.rewrite_identity,
            negation: facets.rewrite_negation,
            operand_order: facets.rewrite_operand_order,
        }
    }

    /// The form `test` settles on across the enabled facets, or `None`
    /// where every facet leaves it as authored. The operand swap
    /// resolves first so the identity rewrite reads the settled order,
    /// and the `not` fold resolves last so it reads the settled
    /// operator.
    fn plan(&self, source: &Source, test: Test<'_>, negated: bool) -> Option<Plan> {
        let Test {
            mut left,
            mut op,
            mut right,
            ..
        } = test;
        let mut flip = false;
        if self.operand_order
            && let Some(reflected) = reflected(op)
            && constancy(left) > constancy(right)
            && source
                .comment_ranges()
                .comments_in_range(test.compare.range())
                .is_empty()
        {
            op = reflected;
            std::mem::swap(&mut left, &mut right);
            flip = true;
        }
        if self.identity
            && let Some(identity) = identity_op(op, left, right)
        {
            op = identity;
        }
        let mut drop_not = false;
        if self.negation && negated && matches!(op, CmpOp::In | CmpOp::Is) {
            op = op.negate();
            drop_not = true;
        }
        (flip || drop_not || op != test.op).then_some(Plan { drop_not, flip, op })
    }

    /// The edit group `compare` earns, or `None` for a chained
    /// comparison and for one every facet leaves as authored.
    fn rewrite(
        &self,
        source: &Source,
        compare: &ExprCompare,
        parent: AnyNodeRef,
    ) -> Option<Vec<Edit>> {
        let test = Test::of(compare)?;
        let negated = negating_parent(parent);
        let plan = self.plan(source, test, negated.is_some())?;
        edits(source, test, &plan, negated)
    }
}

impl Rule for NormalizeComparisons {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        filter_map_over_parented_exprs(source.ast(), Descent::Over, |expr, parent| {
            expr.as_compare_expr()
                .and_then(|compare| self.rewrite(source, compare, parent))
        })
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }

    fn lint(&self, source: &Source) -> Vec<Diagnostic> {
        if !self.identity {
            return Vec::new();
        }
        filter_map_over_exprs(&source.ast().body, Descent::Over, |expr| {
            boolean_lint(Test::of(expr.as_compare_expr()?)?)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{first_expr, parse};

    /// The two-operand test the lone comparison statement of `source`
    /// states.
    pub(super) fn test_of(source: &Source) -> Option<Test<'_>> {
        Test::of(
            first_expr(source)
                .as_compare_expr()
                .expect("first statement is a comparison"),
        )
    }

    #[test]
    fn identity_facet_off_withdraws_the_boolean_lint() {
        let rule = NormalizeComparisons {
            identity: false,
            negation: true,
            operand_order: true,
        };
        assert!(rule.lint(&parse("flag == True\n")).is_empty());
    }

    #[test]
    fn test_of_declines_a_chained_comparison() {
        let source = parse("0 < n < 10\n");
        assert!(test_of(&source).is_none());
    }
}
