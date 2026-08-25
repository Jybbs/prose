//! The literal kinds a parameter's call sites carry, folded into the
//! one scalar type a suggestion names.

use std::collections::BTreeSet;

use ruff_python_ast::{Expr, LiteralExpressionRef, Number, UnaryOp};

/// The signals a parameter draws toward an inferred annotation, folded
/// into the one scalar type they agree on plus an optional `| None` arm.
#[derive(Default)]
pub(super) struct SignalSet {
    has_none: bool,
    opaque: bool,
    types: BTreeSet<&'static str>,
}

impl SignalSet {
    /// Folds the signal `expr` contributes, peeling a unary `+`/`-` over
    /// a number so `-1` reads as `int`. A non-literal lands `opaque`.
    pub(super) fn add(&mut self, expr: &Expr) {
        let inner = match expr {
            Expr::UnaryOp(unary) if matches!(unary.op, UnaryOp::UAdd | UnaryOp::USub) => {
                unary.operand.as_ref()
            }
            _ => expr,
        };
        match inner.as_literal_expr() {
            Some(LiteralExpressionRef::NumberLiteral(number)) => {
                self.types.insert(match &number.value {
                    Number::Int(_) => "int",
                    Number::Float(_) => "float",
                    Number::Complex { .. } => "complex",
                });
            }
            Some(LiteralExpressionRef::StringLiteral(_)) => {
                self.types.insert("str");
            }
            Some(LiteralExpressionRef::BytesLiteral(_)) => {
                self.types.insert("bytes");
            }
            Some(LiteralExpressionRef::BooleanLiteral(_)) => {
                self.types.insert("bool");
            }
            Some(LiteralExpressionRef::NoneLiteral(_)) => self.has_none = true,
            _ => self.opaque = true,
        }
    }

    /// The suggested annotation, or `None` when a non-literal disqualified
    /// the set, the typed signals conflict, or none is typed.
    pub(super) fn suggestion(&self) -> Option<String> {
        if self.opaque {
            return None;
        }
        let mut types = self.types.iter().copied();
        let only = types.next()?;
        if types.next().is_some() {
            return None;
        }
        Some(if self.has_none {
            format!("{only} | None")
        } else {
            only.to_string()
        })
    }
}
