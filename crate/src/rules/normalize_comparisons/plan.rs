//! Reads a two-operand comparison into the rewrite it earns, the form
//! it settles on beside the operands and operator it was read from.

use ruff_python_ast::{CmpOp, Expr, ExprCompare};

/// The form a test settles on, holding the operator it reads with
/// alongside whether its operands trade places and whether an enclosing
/// `not` folds away.
pub(super) struct Plan {
    pub(super) drop_not: bool,
    pub(super) flip: bool,
    pub(super) op: CmpOp,
}

/// The two-operand comparison this rule reads, its operands paired with
/// the operator between them and the node they were read from.
#[derive(Clone, Copy)]
pub(super) struct Test<'a> {
    pub(super) compare: &'a ExprCompare,
    pub(super) left: &'a Expr,
    pub(super) op: CmpOp,
    pub(super) right: &'a Expr,
}

impl<'a> Test<'a> {
    /// The test `compare` states, or `None` for a chained comparison.
    pub(super) fn of(compare: &'a ExprCompare) -> Option<Self> {
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
