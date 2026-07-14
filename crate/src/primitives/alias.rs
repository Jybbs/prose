//! Classifies whether a value names an object that already exists rather
//! than constructing a new one, the split between a bare PEP 484 type
//! alias and a module constant.

use ruff_python_ast::{
    Expr, ExprBinOp, Operator,
    helpers::{is_dotted_name, map_subscript},
};

/// True when `value` names an existing object. A subscript qualifies on a
/// dotted base whatever its slice holds, leaving `Literal["read"]` an
/// alias and `load()[0]` a constant. A `|` union holds only when both
/// sides do, leaving `int | float` an alias and `1 | 2` a constant.
pub(crate) fn value_is_alias(value: &Expr) -> bool {
    match value {
        Expr::BinOp(ExprBinOp {
            left,
            op: Operator::BitOr,
            right,
            ..
        }) => is_union_arm(left) && is_union_arm(right),
        _ => is_dotted_name(map_subscript(value)),
    }
}

/// True when `arm` is a type a PEP 604 union may carry, an alias value
/// or the `None` of an optional. A bare `None` outside a union binds a
/// sentinel rather than naming a type.
fn is_union_arm(arm: &Expr) -> bool {
    arm.is_none_literal_expr() || value_is_alias(arm)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{first_value, parse};

    #[rstest]
    #[case("Turtle", true)]
    #[case("TarFile.open", true)]
    #[case("list[float]", true)]
    #[case("Union[int, float]", true)]
    #[case("Literal[\"read\", \"write\"]", true)]
    #[case("int | float", true)]
    #[case("int | None", true)]
    #[case("str | None | bytes", true)]
    #[case("dict[str, int] | list[str]", true)]
    #[case("42", false)]
    #[case("None", false)]
    #[case("\"MyClass\"", false)]
    #[case("f\"{prefix}-suffix\"", false)]
    #[case("[1, 2]", false)]
    #[case("{\"a\": 1}", false)]
    #[case("(int, str)", false)]
    #[case("1 | 2", false)]
    #[case("int | 2", false)]
    #[case("BASE * 2", false)]
    #[case("make()", false)]
    #[case("lambda row: row.id", false)]
    #[case("get_registry().default", false)]
    #[case("load()[0]", false)]
    fn value_is_alias_splits_named_objects_from_constructed_data(
        #[case] value_src: &str,
        #[case] expected: bool,
    ) {
        let source = parse(&format!("X = {value_src}\n"));
        assert_eq!(
            value_is_alias(first_value(&source)),
            expected,
            "{value_src}"
        );
    }
}
