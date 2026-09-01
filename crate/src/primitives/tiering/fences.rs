//! The fence a class raises where its base list runs a hook no static
//! read of the module follows, so a metaclass or a `__class_getitem__`
//! reached through a compiled module bounds the run rather than joining
//! its reference graph.

use itertools::Itertools;
use ruff_python_ast::{Expr, Stmt, helpers::any_over_expr};
use rustc_hash::FxHashSet;

use super::refs::root_name;

/// The slot of every statement in `body` that fences the run, being a
/// class whose base list runs a hook this module cannot read. Nothing
/// written above such a class may sort below it, since the hook reaches
/// module bindings the base list never names and every one of them was
/// bound when it ran. A definition written below was unbound then, so it
/// stays free to sort.
pub(crate) fn fenced_slots(body: &[Stmt], defined: &FxHashSet<&str>) -> Vec<usize> {
    body.iter()
        .positions(|stmt| runs_opaque_code(stmt, defined))
        .collect()
}

/// True where a class's base list runs a call or a subscript this module
/// cannot read, its chain rooting outside `defined` or in something
/// other than a name. A metaclass and a `__class_getitem__` hook both
/// run at class creation, and a C extension holding one calls back into
/// this module, so no static read follows where it reaches.
fn runs_opaque_code(stmt: &Stmt, defined: &FxHashSet<&str>) -> bool {
    let Stmt::ClassDef(class) = stmt else {
        return false;
    };
    class.arguments.as_deref().is_some_and(|arguments| {
        arguments.iter_source_order().any(|argument| {
            any_over_expr(argument.value(), |expr| runs_a_foreign_hook(expr, defined))
        })
    })
}

/// True where `expr` calls or subscripts something this module does not
/// define, which runs code no static read of the module reaches.
fn runs_a_foreign_hook(expr: &Expr, defined: &FxHashSet<&str>) -> bool {
    let invoked = match expr {
        Expr::Call(call) => &call.func,
        Expr::Subscript(subscript) => &subscript.value,
        _ => return false,
    };
    root_name(invoked).is_none_or(|name| !defined.contains(name))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::{primitives::tiering::definition_names, testing::parse};

    #[rstest]
    #[case::imported_subscripted_base("from vendor import Generic\n\n\nclass W(Generic[str]):\n    pass\n", vec![1])]
    #[case::imported_called_base("from vendor import make\n\n\nclass W(make()):\n    pass\n", vec![1])]
    #[case::local_subscripted_base("class G:\n    pass\n\n\nclass W(G[str]):\n    pass\n", vec![])]
    #[case::plain_imported_base("from vendor import Base\n\n\nclass W(Base):\n    pass\n", vec![])]
    #[case::opaque_metaclass("from vendor import meta\n\n\nclass W(metaclass=meta()):\n    pass\n", vec![1])]
    #[case::no_class("import os\n\nVALUE = 1\n", vec![])]
    fn fenced_slots_names_only_a_hook_this_module_cannot_read(
        #[case] src: &str,
        #[case] expected: Vec<usize>,
    ) {
        let source = parse(src);
        let body = &source.ast().body;
        assert_eq!(fenced_slots(body, &definition_names(body)), expected);
    }
}
