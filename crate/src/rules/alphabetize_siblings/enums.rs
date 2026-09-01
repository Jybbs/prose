//! Enumeration ordering. Reports whether a class numbers its members by
//! the order they are written in, through `auto`, through a `__new__`
//! numbering them as it runs, through an `_order_` the loader checks, or
//! through two members sharing one value.

use ruff_python_ast::{
    Expr, Stmt, StmtClassDef, comparable::ComparableExpr, helpers::any_over_expr,
};
use rustc_hash::FxHashSet;

use crate::primitives::binding::{
    from_import_bound_name, single_name_assignment, type_head_identifier,
};

/// The `enum` member factory whose result counts the members written
/// ahead of it.
const AUTO: &str = "auto";

/// The `enum` module every ordered base and `auto` come out of.
const ENUM: &str = "enum";

/// The class-body name spelling the member order the loader checks the
/// declared members against, raising where the two differ.
const ORDER: &str = "_order_";

/// The bases whose class body numbers its members by declaration
/// order, read by the last segment of a dotted name.
const ORDERED_BASES: &[&str] = &[
    "Enum", "EnumMeta", "EnumType", "Flag", "IntEnum", "IntFlag", "ReprEnum", "StrEnum",
];

/// The enumeration surface one module presents: every name standing for
/// an ordered base, and every name bound to `enum.auto`. A class
/// reaching a base through another class of the same module numbers its
/// members the way a direct subclass does, and `auto` under an alias
/// counts the same way the bare name does.
pub(super) struct Enumerations<'a> {
    autos: FxHashSet<&'a str>,
    bases: FxHashSet<&'a str>,
}

impl<'a> Enumerations<'a> {
    /// Reads `body` for the aliases `enum.auto` binds and for the
    /// classes an ordered base reaches, growing the class set to a fixed
    /// point so a base named through another local class still counts.
    pub(super) fn of(body: &'a [Stmt]) -> Self {
        let mut autos: FxHashSet<&str> = FxHashSet::from_iter([AUTO]);
        let mut bases: FxHashSet<&str> = ORDERED_BASES.iter().copied().collect();
        for node in body.iter().filter_map(Stmt::as_import_from_stmt) {
            if node.module.as_deref() != Some(ENUM) {
                continue;
            }
            autos.extend(
                node.names
                    .iter()
                    .filter(|alias| alias.name.as_str() == AUTO)
                    .map(from_import_bound_name),
            );
        }
        loop {
            let found: Vec<&str> = body
                .iter()
                .filter_map(Stmt::as_class_def_stmt)
                .filter(|class| !bases.contains(class.name.as_str()) && names_a_base(class, &bases))
                .map(|class| class.name.as_str())
                .collect();
            if found.is_empty() {
                break;
            }
            bases.extend(found);
        }
        Self { autos, bases }
    }
}

/// True where `class` is an enumeration whose members take their value
/// from the order they are written in, so a reorder rewrites what each
/// member holds or makes the module raise as it loads. An enumeration
/// spelling every value out and repeating none sorts freely, its
/// members meaning the same wherever they sit.
pub(super) fn class_orders_members(class: &StmtClassDef, enums: &Enumerations) -> bool {
    names_a_base(class, &enums.bases) && numbers_by_position(&class.body, &enums.autos)
}

/// True where `expr` calls one of the names `enum.auto` binds, whose
/// result counts from the members declared ahead of it.
fn calls_auto(expr: &Expr, autos: &FxHashSet<&str>) -> bool {
    matches!(
        expr,
        Expr::Call(call) if type_head_identifier(&call.func).is_some_and(|named| autos.contains(named))
    )
}

/// True where a base or a class keyword of `class` names something in
/// `bases`, each argument read by the head identifier of its dotted or
/// subscripted form.
fn names_a_base(class: &StmtClassDef, bases: &FxHashSet<&str>) -> bool {
    class.arguments.as_deref().is_some_and(|arguments| {
        arguments
            .iter_source_order()
            .filter_map(|argument| type_head_identifier(argument.value()))
            .any(|named| bases.contains(named))
    })
}

/// True where one class-body statement makes a member's value or its
/// identity depend on where it sits, being a `__new__` the enumeration
/// runs per member, an `_order_` the loader checks the declared order
/// against, a value reaching `auto` anywhere inside it, or a value a
/// member above already carries, which makes the one above canonical
/// and this one its alias.
fn numbers_by_position(body: &[Stmt], autos: &FxHashSet<&str>) -> bool {
    let mut seen: FxHashSet<ComparableExpr> = FxHashSet::default();
    for stmt in body {
        if let Stmt::FunctionDef(func) = stmt {
            if func.name.as_str() == "__new__" {
                return true;
            }
            continue;
        }
        let Some((target, value)) = single_name_assignment(stmt) else {
            continue;
        };
        if target.id.as_str() == ORDER {
            return true;
        }
        let Some(value) = value else { continue };
        if any_over_expr(value, |expr| calls_auto(expr, autos)) {
            return true;
        }
        if !seen.insert(ComparableExpr::from(value)) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::parse;

    /// True where the last class of `src` numbers its members by
    /// the order they are written in.
    fn orders(src: &str) -> bool {
        let source = parse(src);
        let body = &source.ast().body;
        let enums = Enumerations::of(body);
        body.iter()
            .filter_map(Stmt::as_class_def_stmt)
            .next_back()
            .is_some_and(|class| class_orders_members(class, &enums))
    }

    #[rstest]
    #[case::auto(
        "from enum import Enum, auto\n\n\nclass K(Enum):\n    Z = auto()\n",
        true
    )]
    #[case::auto_in_a_tuple(
        "from enum import Enum, auto\n\n\nclass K(Enum):\n    Z = auto(), \"z\"\n",
        true
    )]
    #[case::auto_under_an_alias(
        "from enum import Enum\nfrom enum import auto as gen\n\n\nclass K(Enum):\n    Z = gen()\n",
        true
    )]
    #[case::dunder_new(
        "from enum import IntEnum\n\n\nclass K(IntEnum):\n    Z = 1\n\n    def __new__(cls): ...\n",
        true
    )]
    #[case::order_sentinel(
        "from enum import Enum\n\n\nclass K(Enum):\n    _order_ = \"Z Y\"\n    Z = 1\n    Y = 2\n",
        true
    )]
    #[case::repeated_value(
        "from enum import Enum\n\n\nclass K(Enum):\n    Z = 1\n    Y = 1\n",
        true
    )]
    #[case::base_one_hop(
        "from enum import Enum, auto\n\n\nclass B(Enum):\n    pass\n\n\nclass K(B):\n    Z = auto()\n",
        true
    )]
    #[case::metaclass_keyword(
        "from enum import EnumMeta, auto\n\n\nclass K(metaclass=EnumMeta):\n    Z = auto()\n",
        true
    )]
    #[case::every_value_spelt(
        "from enum import Enum\n\n\nclass K(Enum):\n    Z = 1\n    Y = 2\n",
        false
    )]
    #[case::not_an_enumeration("class K:\n    Z = 1\n    Y = 1\n", false)]
    #[case::no_base("class K:\n    pass\n", false)]
    fn class_orders_members_reads_every_positional_shape(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        assert_eq!(orders(src), expected);
    }
}
