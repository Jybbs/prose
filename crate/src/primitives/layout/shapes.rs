//! Reads the layout shape a construct already carries.

use super::*;

/// True for the collapse-only forms, a subscript whose `[index]` joins
/// onto one line whatever the index shape and the four comprehensions,
/// each joining when it fits and never expanding the way a literal does.
pub(crate) fn is_collapse_only(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::DictComp(_)
            | Expr::Generator(_)
            | Expr::ListComp(_)
            | Expr::SetComp(_)
            | Expr::Subscript(_)
    )
}

/// True for the bracketed expressions the visitor measures for a
/// single-line collapse: the four collection literals plus the
/// collapse-only forms, a subscript and the four comprehensions.
pub(crate) fn is_collapsible(expr: &Expr) -> bool {
    is_layoutable(expr) || is_collapse_only(expr)
}

/// True when `slice`, a bracketed construct's source text, already
/// carries the flush column shape the expand path emits, its opening
/// bracket ending its line and its closing bracket opening its own.
/// Every other break is a fracture.
pub(crate) fn is_column_shaped(slice: &str) -> bool {
    flush_bracket_open(slice).is_some_and(|body| {
        body.rsplit_once(['\n', '\r'])
            .is_some_and(|(_, close)| close.trim_start().len() == 1)
    })
}

/// True when `range` carries a break a join could close, spanning
/// lines without already holding the flush column shape.
pub(crate) fn is_fractured(source: &Source, range: TextRange) -> bool {
    source.contains_line_break(range) && !is_column_shaped(source.slice(range))
}

/// True for the four collection-literal `Expr` variants the layout
/// rules lay out, `Dict`, `List`, `Set`, and `Tuple`.
pub(crate) fn is_layoutable(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Dict(_) | Expr::List(_) | Expr::Set(_) | Expr::Tuple(_)
    )
}

/// True for a `Dict`, `List`, `Set`, or parenthesized `Tuple` node
/// carrying more than one entry. A bare tuple carries no bracket pair
/// to hang broken lines on.
pub(crate) fn is_multi_entry(node: AnyNodeRef) -> bool {
    match node {
        AnyNodeRef::ExprDict(dict) => dict.len() > 1,
        AnyNodeRef::ExprList(list) => list.len() > 1,
        AnyNodeRef::ExprSet(set) => set.len() > 1,
        AnyNodeRef::ExprTuple(tuple) => tuple.parenthesized && tuple.len() > 1,
        _ => false,
    }
}
