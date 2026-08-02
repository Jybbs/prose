//! Normalizes a two-operand comparison so it states its check outright.
//! A `==` or `!=` test against `None` becomes `is` or `is not`, a test
//! whose constant side leads flips so the variable leads, and a leading
//! `not` folds into the `in` or `is` it negates. A test against `True`
//! or `False` is flagged rather than rewritten, because dropping the
//! literal changes the test for a non-boolean operand. A chained
//! comparison is left as written, as is one inside an f-string or
//! t-string replacement field.

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    AnyNodeRef, CmpOp, Expr, ExprCompare, ExprUnaryOp, UnaryOp, helpers::is_constant_non_singleton,
};
use ruff_python_stdlib::str::is_cased_uppercase;
use ruff_text_size::{Ranged, TextRange};

use crate::{
    config::Config,
    diagnostics::Diagnostic,
    primitives::{
        comparison::opening_token_kind,
        walk::{Interpolations, filter_map_over_exprs, filter_map_over_parented_exprs},
    },
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct NormalizeComparisons {
    identity: bool,
    negation: bool,
    operand_order: bool,
}

impl NormalizeComparisons {
    pub(crate) const MESSAGE: &'static str = "normalize a comparison to state its check directly";

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
            && !source.intersects_comment(test.compare)
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
        filter_map_over_parented_exprs(source.ast(), Interpolations::Opaque, |expr, parent| {
            self.rewrite(source, expr.as_compare_expr()?, parent)
        })
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }

    fn lint(&self, source: &Source) -> Vec<Diagnostic> {
        if !self.identity {
            return Vec::new();
        }
        filter_map_over_exprs(source.ast(), |expr| {
            boolean_lint(Test::of(expr.as_compare_expr()?)?)
        })
    }
}

/// How constant an operand reads, the ladder deciding which side of a
/// test leads. The variants rank by declaration order, so a literal
/// outranks a `SCREAMING_CASE` name and both outrank everything else.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Constancy {
    Unlikely,
    Probably,
    Definitely,
}

/// The form a test settles on, holding the operator it reads with
/// alongside whether its operands trade places and whether an enclosing
/// `not` folds away.
struct Plan {
    drop_not: bool,
    flip: bool,
    op: CmpOp,
}

/// The two-operand comparison this rule reads, its operands paired with
/// the operator between them and the node they were read from.
#[derive(Clone, Copy)]
struct Test<'a> {
    compare: &'a ExprCompare,
    left: &'a Expr,
    op: CmpOp,
    right: &'a Expr,
}

impl<'a> Test<'a> {
    /// The test `compare` states, or `None` for a chained comparison.
    fn of(compare: &'a ExprCompare) -> Option<Self> {
        let ([op], [right]) = (&*compare.ops, &*compare.comparators) else {
            return None;
        };
        Some(Self {
            compare,
            left: &compare.left,
            op: *op,
            right,
        })
    }
}

/// The lint a test comparing an operand against `True` or `False`
/// draws, or `None` for every other test.
fn boolean_lint(test: Test<'_>) -> Option<Diagnostic> {
    if !matches!(test.op, CmpOp::Eq | CmpOp::NotEq) {
        return None;
    }
    let literal = test
        .left
        .as_boolean_literal_expr()
        .or_else(|| test.right.as_boolean_literal_expr())?;
    let value = if literal.value { "True" } else { "False" };
    Some(Diagnostic::lint(
        NormalizeComparisons::SLUG,
        literal.range(),
        format!(
            "Comparison against the `{value}` literal. Test the operand directly, or compare with `is` to check identity"
        ),
    ))
}

/// Scores `expr` on the constancy ladder. A literal is `Definitely`, a
/// `SCREAMING_CASE` name or attribute is `Probably`, a collection or
/// arithmetic expression takes the weakest score among its parts, and
/// everything else is `Unlikely`.
fn constancy(expr: &Expr) -> Constancy {
    match expr {
        _ if expr.is_literal_expr() => Constancy::Definitely,
        Expr::Attribute(attribute) => named_constancy(&attribute.attr),
        Expr::BinOp(bin_op) => constancy(&bin_op.left).min(constancy(&bin_op.right)),
        Expr::Dict(dict) => weakest(dict.iter_values().chain(dict.iter_keys().flatten())),
        Expr::List(list) => weakest(list),
        Expr::Name(name) => named_constancy(&name.id),
        Expr::Tuple(tuple) => weakest(tuple),
        Expr::UnaryOp(unary)
            if matches!(unary.op, UnaryOp::Invert | UnaryOp::UAdd | UnaryOp::USub) =>
        {
            constancy(&unary.operand)
        }
        _ => Constancy::Unlikely,
    }
}

/// The edits `plan` calls for, the `not` deletion first, then the
/// operand swap, then the operator-token replacement. `None` where a
/// rewritten operator resolves to no token, which declines the whole
/// group rather than swapping operands around an operator that stayed
/// as authored.
fn edits(
    source: &Source,
    test: Test<'_>,
    plan: &Plan,
    negated: Option<&ExprUnaryOp>,
) -> Option<Vec<Edit>> {
    let lead = source.paren_aware_range(test.left.into(), test.compare.into());
    let trail = source.paren_aware_range(test.right.into(), test.compare.into());
    let operator = if plan.op == test.op {
        None
    } else {
        Some(operator_range(
            source,
            TextRange::new(lead.end(), trail.start()),
            test.op,
        )?)
    };
    let mut edits = Vec::new();
    if let Some(unary) = negated.filter(|_| plan.drop_not) {
        let operand = source.paren_aware_range(test.compare.into(), unary.into());
        edits.push(Edit::range_deletion(TextRange::new(
            unary.start(),
            operand.start(),
        )));
    }
    if plan.flip {
        edits.push(Edit::range_replacement(
            source.slice(trail).to_owned(),
            lead,
        ));
        edits.push(Edit::range_replacement(
            source.slice(lead).to_owned(),
            trail,
        ));
    }
    edits.extend(operator.map(|range| Edit::range_replacement(plan.op.as_str().to_owned(), range)));
    Some(edits)
}

/// The identity operator a test settles on when either side is a `None`
/// literal, or `None` where neither is or the other side is a constant
/// `is` would not match.
fn identity_op(op: CmpOp, left: &Expr, right: &Expr) -> Option<CmpOp> {
    let identity = match op {
        CmpOp::Eq => CmpOp::Is,
        CmpOp::NotEq => CmpOp::IsNot,
        _ => return None,
    };
    let other = if left.is_none_literal_expr() {
        right
    } else if right.is_none_literal_expr() {
        left
    } else {
        return None;
    };
    (!is_constant_non_singleton(other)).then_some(identity)
}

/// The constancy an identifier carries, `Probably` for a
/// `SCREAMING_CASE` name and `Unlikely` otherwise.
fn named_constancy(identifier: &str) -> Constancy {
    if is_cased_uppercase(identifier) {
        Constancy::Probably
    } else {
        Constancy::Unlikely
    }
}

/// The `not` expression `parent` names, or `None` for every other node.
fn negating_parent(parent: AnyNodeRef<'_>) -> Option<&ExprUnaryOp> {
    match parent {
        AnyNodeRef::ExprUnaryOp(unary) if unary.op.is_not() => Some(unary),
        _ => None,
    }
}

/// The range of the lone lexer token spelling `op` inside `gap`.
fn operator_range(source: &Source, gap: TextRange, op: CmpOp) -> Option<TextRange> {
    let kind = opening_token_kind(op);
    source
        .tokens()
        .in_range(gap)
        .iter()
        .find(|token| token.kind() == kind)
        .map(Ranged::range)
}

/// The operator a flipped test reads with, or `None` for the operators
/// an operand swap does not preserve.
const fn reflected(op: CmpOp) -> Option<CmpOp> {
    match op {
        CmpOp::Eq => Some(CmpOp::Eq),
        CmpOp::Gt => Some(CmpOp::Lt),
        CmpOp::GtE => Some(CmpOp::LtE),
        CmpOp::Lt => Some(CmpOp::Gt),
        CmpOp::LtE => Some(CmpOp::GtE),
        CmpOp::NotEq => Some(CmpOp::NotEq),
        CmpOp::In | CmpOp::Is | CmpOp::IsNot | CmpOp::NotIn => None,
    }
}

/// The weakest constancy among `exprs`, `Definitely` for an empty
/// collection.
fn weakest<'a>(exprs: impl IntoIterator<Item = &'a Expr>) -> Constancy {
    exprs
        .into_iter()
        .map(constancy)
        .min()
        .unwrap_or(Constancy::Definitely)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::{
        diagnostics::Severity,
        testing::{first_expr, parse},
    };

    fn constancy_of(src: &str) -> Constancy {
        let source = parse(src);
        constancy(first_expr(&source))
    }

    /// The two-operand test the lone comparison statement of `source`
    /// states.
    fn test_of(source: &Source) -> Option<Test<'_>> {
        Test::of(
            first_expr(source)
                .as_compare_expr()
                .expect("first statement is a comparison"),
        )
    }

    #[rstest]
    #[case("[0, 1]", Constancy::Definitely)]
    #[case("[]", Constancy::Definitely)]
    #[case("[a, 1]", Constancy::Unlikely)]
    #[case("(1, 2)", Constancy::Definitely)]
    #[case("(1, LIMIT)", Constancy::Probably)]
    #[case("{'k': 1}", Constancy::Definitely)]
    #[case("{'k': v}", Constancy::Unlikely)]
    #[case("{LIMIT: 1}", Constancy::Probably)]
    #[case("{**base, 'k': 1}", Constancy::Unlikely)]
    #[case("{**BASE, 'k': 1}", Constancy::Probably)]
    fn constancy_takes_a_collection_at_its_weakest_part(
        #[case] src: &str,
        #[case] expected: Constancy,
    ) {
        assert_eq!(constancy_of(src), expected);
    }

    #[rstest]
    #[case("LIMIT", Constancy::Probably)]
    #[case("limit", Constancy::Unlikely)]
    #[case("cfg.LIMIT", Constancy::Probably)]
    #[case("cfg.limit", Constancy::Unlikely)]
    fn constancy_reads_a_name_by_its_casing(#[case] src: &str, #[case] expected: Constancy) {
        assert_eq!(constancy_of(src), expected);
    }

    #[rstest]
    #[case("42", Constancy::Definitely)]
    #[case("-42", Constancy::Definitely)]
    #[case("+42", Constancy::Definitely)]
    #[case("~MASK", Constancy::Probably)]
    #[case("LIMIT + 1", Constancy::Probably)]
    #[case("limit + 1", Constancy::Unlikely)]
    #[case("f()", Constancy::Unlikely)]
    #[case("not flag", Constancy::Unlikely)]
    fn constancy_scores_a_literal_above_a_computed_operand(
        #[case] src: &str,
        #[case] expected: Constancy,
    ) {
        assert_eq!(constancy_of(src), expected);
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

    #[rstest]
    #[case("value == None\n", Some(CmpOp::Is))]
    #[case("value != None\n", Some(CmpOp::IsNot))]
    #[case("None == value\n", Some(CmpOp::Is))]
    #[case("None == 5\n", None)]
    #[case("value == 5\n", None)]
    #[case("value < None\n", None)]
    fn identity_op_maps_an_equality_against_none(
        #[case] src: &str,
        #[case] expected: Option<CmpOp>,
    ) {
        let source = parse(src);
        let test = test_of(&source).expect("a two-operand comparison");
        assert_eq!(identity_op(test.op, test.left, test.right), expected);
    }

    #[test]
    fn lint_pins_the_literal_range_severity_and_message() {
        let source = parse("flag == True\n");
        let diagnostics = NormalizeComparisons::from_config(&Config::default()).lint(&source);
        let only = diagnostics.first().expect("one boolean lint");

        assert_eq!(only.severity, Severity::Lint);
        assert!(only.fix.is_none());
        assert_eq!(source.slice(only.range), "True");
        assert!(only.message.contains("`True`"));
    }

    #[test]
    fn negating_parent_declines_a_non_not_unary() {
        let source = parse("-x\n");
        let unary = first_expr(&source)
            .as_unary_op_expr()
            .expect("first statement is a unary op");

        assert!(negating_parent(unary.into()).is_none());
    }

    #[test]
    fn operator_range_declines_a_gap_holding_no_operator_token() {
        let source = parse("value == None\n");
        assert!(operator_range(&source, TextRange::default(), CmpOp::Eq).is_none());
    }

    #[rstest]
    #[case(CmpOp::Eq, Some(CmpOp::Eq))]
    #[case(CmpOp::Gt, Some(CmpOp::Lt))]
    #[case(CmpOp::GtE, Some(CmpOp::LtE))]
    #[case(CmpOp::In, None)]
    #[case(CmpOp::Is, None)]
    #[case(CmpOp::IsNot, None)]
    #[case(CmpOp::Lt, Some(CmpOp::Gt))]
    #[case(CmpOp::LtE, Some(CmpOp::GtE))]
    #[case(CmpOp::NotEq, Some(CmpOp::NotEq))]
    #[case(CmpOp::NotIn, None)]
    fn reflected_covers_every_variant(#[case] op: CmpOp, #[case] expected: Option<CmpOp>) {
        assert_eq!(reflected(op), expected);
    }

    #[test]
    fn test_of_declines_a_chained_comparison() {
        let source = parse("0 < n < 10\n");
        assert!(test_of(&source).is_none());
    }
}
