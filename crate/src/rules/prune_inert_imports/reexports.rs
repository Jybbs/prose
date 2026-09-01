//! The explicit re-export surface of a module, read from its
//! module-scope `__all__` writes and from the PEP 484 `x as x` alias
//! form.

use ruff_python_ast::{Alias, Expr, Stmt, helpers::is_dunder};
use rustc_hash::FxHashSet;

use super::inventory::{ImportNode, is_self_alias};
use crate::primitives::{
    binding::{sequence_elts, single_name_assignment},
    scope::sub_bodies,
    walk::any_over_stmts,
};

const DUNDER_ALL: &str = "__all__";

/// The names a module marks for re-export.
pub(super) struct Reexports<'a> {
    names: FxHashSet<&'a str>,
    settled: bool,
}

impl<'a> Reexports<'a> {
    /// Reads every module-scope `__all__` write. `settled` drops to
    /// `false` on a write naming anything other than string literals and
    /// on any write nested below module scope, and an unsettled surface
    /// holds every name.
    pub(super) fn of(body: &'a [Stmt]) -> Self {
        let mut names = FxHashSet::default();
        for stmt in body {
            match dunder_all_write(stmt) {
                None => {}
                Some(DunderAll::Names(items)) => names.extend(items),
                Some(DunderAll::Unreadable) => return Self::unsettled(),
            }
            if nested_dunder_all_write(stmt) {
                return Self::unsettled();
            }
        }
        Self {
            names,
            settled: true,
        }
    }

    /// A surface no static read settles, holding every name.
    fn unsettled() -> Self {
        Self {
            names: FxHashSet::default(),
            settled: false,
        }
    }

    /// True when `alias`, binding `bound`, marks an explicit re-export.
    /// An import binding `__all__` itself sets the whole surface, so it
    /// holds alongside the names a write lists.
    pub(super) fn holds(&self, alias: &Alias, bound: &str) -> bool {
        !self.settled || bound == DUNDER_ALL || is_self_alias(alias) || self.names.contains(bound)
    }
}

/// True where `node` takes a name out of a module its own name marks
/// private, the convention a public module re-exports its
/// implementation through. A dunder module such as `__future__` is not
/// one of those, its names carrying compiler meaning rather than a
/// surface to re-export. Nothing in the importing module needs to
/// read such a name for it to be part of that module's surface, so an
/// unread one reads as re-exported rather than as dead.
pub(super) fn reexports_a_private_member(node: &ImportNode<'_>, alias: &Alias) -> bool {
    if matches!(node, ImportNode::Bare(_)) {
        return false;
    }
    let source = node.source(alias);
    let segments = source.segments();
    segments
        .len()
        .checked_sub(1)
        .and_then(|member| segments[..member].last())
        .is_some_and(|module| module.starts_with('_') && !is_dunder(module))
}

/// What one module-scope statement contributes to `__all__`.
enum DunderAll<'a> {
    Names(Vec<&'a str>),
    Unreadable,
}

/// What `stmt` writes to `__all__`, `None` for a statement leaving it
/// alone.
fn dunder_all_write(stmt: &Stmt) -> Option<DunderAll<'_>> {
    let value = match stmt {
        Stmt::AugAssign(node) if names_dunder_all(&node.target) => node.value.as_ref(),
        Stmt::Expr(node) if mutates_dunder_all(&node.value) => return Some(DunderAll::Unreadable),
        _ => {
            let (target, value) = single_name_assignment(stmt)?;
            if target.id.as_str() != DUNDER_ALL {
                return None;
            }
            value?
        }
    };
    Some(string_items(value).map_or(DunderAll::Unreadable, DunderAll::Names))
}

/// True for a call on an attribute of `__all__`, covering the
/// `__all__.append(…)` and `__all__.extend(…)` forms.
fn mutates_dunder_all(value: &Expr) -> bool {
    value.as_call_expr().is_some_and(|call| {
        call.func
            .as_attribute_expr()
            .is_some_and(|attribute| names_dunder_all(&attribute.value))
    })
}

/// True when `expr` is the bare name `__all__`.
fn names_dunder_all(expr: &Expr) -> bool {
    expr.as_name_expr()
        .is_some_and(|name| name.id.as_str() == DUNDER_ALL)
}

/// True when a statement below `stmt`'s own level writes `__all__`,
/// covering a conditional branch, a loop body, and a `def` or `class`
/// scope alike.
fn nested_dunder_all_write(stmt: &Stmt) -> bool {
    sub_bodies(stmt)
        .into_iter()
        .any(|(body, _)| any_over_stmts(body, |nested| dunder_all_write(nested).is_some()))
}

/// The string-literal items of a list or tuple display. `None` when
/// `value` is another shape or carries an item that is not a string
/// literal.
fn string_items(value: &Expr) -> Option<Vec<&str>> {
    sequence_elts(value)?
        .iter()
        .map(|elt| Some(elt.as_string_literal_expr()?.value.to_str()))
        .collect()
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::parse;

    #[rstest]
    #[case::conditional("if flag:\n    __all__ = [\"other\"]\n")]
    #[case::function_scope("def setup():\n    global __all__\n    __all__ = [\"other\"]\n")]
    #[case::class_scope("class C:\n    __all__ = [\"other\"]\n")]
    #[case::loop_body("for _ in xs:\n    __all__ = [\"other\"]\n")]
    #[case::try_handler("try:\n    pass\nexcept E:\n    __all__ = [\"other\"]\n")]
    fn a_write_below_module_scope_holds_every_name(#[case] src: &str) {
        let source = parse(&format!("from json import dumps\n{src}"));
        let body = &source.ast().body;
        let alias = &body[0].as_import_from_stmt().expect("a from import").names[0];
        assert!(Reexports::of(body).holds(alias, "dumps"));
    }

    #[rstest]
    #[case::call("__all__ = build()\n")]
    #[case::name("__all__ = EXPORTS\n")]
    #[case::non_literal_item("__all__ = [name]\n")]
    #[case::append("__all__ = []\n__all__.append(\"other\")\n")]
    #[case::extend("__all__ = []\n__all__.extend(other)\n")]
    fn an_unreadable_write_holds_every_name(#[case] src: &str) {
        let source = parse(&format!("from json import dumps\n{src}"));
        let body = &source.ast().body;
        let alias = &body[0].as_import_from_stmt().expect("a from import").names[0];
        assert!(Reexports::of(body).holds(alias, "dumps"));
    }

    #[rstest]
    #[case::list("__all__ = [\"loads\"]\n", true)]
    #[case::tuple("__all__ = (\"loads\",)\n", true)]
    #[case::annotated("__all__: list[str] = [\"loads\"]\n", true)]
    #[case::augmented("__all__ = []\n__all__ += [\"loads\"]\n", true)]
    #[case::bare_annotation("__all__: list[str]\n", false)]
    #[case::other_name("__slots__ = [\"loads\"]\n", false)]
    #[case::no_dunder_all("value = 1\n", false)]
    fn of_reads_the_listed_names(#[case] src: &str, #[case] holds: bool) {
        let source = parse(&format!("from json import loads\n{src}"));
        let body = &source.ast().body;
        let alias = &body[0].as_import_from_stmt().expect("a from import").names[0];
        assert_eq!(Reexports::of(body).holds(alias, "loads"), holds);
    }
}
