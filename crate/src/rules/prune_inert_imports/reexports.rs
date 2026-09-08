//! The explicit re-export surface of a module, read from its
//! module-scope `__all__` writes, from an import binding `__all__`
//! itself, from the PEP 484 `x as x` alias form, and from an import
//! whose source module reads as private.

use ruff_python_ast::{Alias, Expr, Stmt, StmtAssign, helpers::is_dunder};
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
    surface: Surface,
}

impl<'a> Reexports<'a> {
    /// Reads every module-scope `__all__` write, along with every import
    /// binding the name, into one surface. A write naming anything other
    /// than string literals, a write below module scope, and an import
    /// binding `__all__` each leave the surface `Unreadable` and every
    /// name held, whereas a module writing it nowhere leaves the surface
    /// `Undeclared`.
    pub(super) fn of(body: &'a [Stmt]) -> Self {
        let mut names = FxHashSet::default();
        let mut surface = Surface::Undeclared;
        for stmt in body {
            match dunder_all_write(stmt) {
                None => {}
                Some(DunderAll::Names(items)) => {
                    names.extend(items);
                    surface = Surface::Listed;
                }
                Some(DunderAll::Unreadable) => return Self::unreadable(),
            }
            if nested_dunder_all_write(stmt) {
                return Self::unreadable();
            }
        }
        Self { names, surface }
    }

    /// A surface no static read settles, holding every name.
    fn unreadable() -> Self {
        Self {
            names: FxHashSet::default(),
            surface: Surface::Unreadable,
        }
    }

    /// True when the module writes `__all__` anywhere or binds the
    /// name from another module, whatever the write lists.
    pub(super) fn declares_a_surface(&self) -> bool {
        !matches!(self.surface, Surface::Undeclared)
    }

    /// True when `alias`, binding `bound`, marks an explicit re-export.
    pub(super) fn holds(&self, alias: &Alias, bound: &str) -> bool {
        matches!(self.surface, Surface::Unreadable)
            || is_self_alias(alias)
            || self.names.contains(bound)
    }
}

/// What one statement contributes to `__all__`.
enum DunderAll<'a> {
    Names(Vec<&'a str>),
    Unreadable,
}

/// The state a module's `__all__` writes leave its re-export surface
/// in.
enum Surface {
    /// Every write reads as a list or tuple of string literals.
    Listed,
    /// The module writes `__all__` nowhere.
    Undeclared,
    /// A write no static read settles, a write below module scope, or
    /// an import binding the name.
    Unreadable,
}

/// True where `node` takes a name out of a module whose last segment
/// leads with `_`, a dunder module such as `__future__` excepted.
pub(super) fn reexports_a_private_member(node: &ImportNode<'_>) -> bool {
    node.module()
        .map(|source| source.rsplit_once('.').map_or(source, |(_, last)| last))
        .is_some_and(|module| module.starts_with('_') && !is_dunder(module))
}

/// What `stmt` writes to `__all__`, `None` for a statement leaving it
/// alone. An import binding the name reads as unreadable, because the
/// list it binds is written in another module.
fn dunder_all_write(stmt: &Stmt) -> Option<DunderAll<'_>> {
    let value = match stmt {
        Stmt::Assign(node) if writes_into_dunder_all(node) => {
            return Some(DunderAll::Unreadable);
        }
        Stmt::AugAssign(node) if names_dunder_all(&node.target) => node.value.as_ref(),
        Stmt::Expr(node) if mutates_dunder_all(&node.value) => return Some(DunderAll::Unreadable),
        Stmt::Import(_) | Stmt::ImportFrom(_) => {
            return imports_dunder_all(stmt).then_some(DunderAll::Unreadable);
        }
        _ => {
            let (_, value) = single_name_assignment(stmt)
                .filter(|(target, _)| target.id.as_str() == DUNDER_ALL)?;
            value?
        }
    };
    Some(string_items(value).map_or(DunderAll::Unreadable, DunderAll::Names))
}

/// True when `stmt` binds `__all__` out of another module.
fn imports_dunder_all(stmt: &Stmt) -> bool {
    ImportNode::of(stmt).is_some_and(|node| {
        node.names()
            .iter()
            .any(|alias| node.bound(alias) == DUNDER_ALL)
    })
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

/// True for an assignment writing through a subscript of `__all__`,
/// covering the `__all__[:] = …` and `__all__[0] = …` forms.
fn writes_into_dunder_all(node: &StmtAssign) -> bool {
    node.targets.iter().any(|target| {
        target
            .as_subscript_expr()
            .is_some_and(|subscript| names_dunder_all(&subscript.value))
    })
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::parse;

    /// Returns the `holds` verdict `Reexports::of` reaches over the
    /// first import of `src`, which binds `bound`.
    fn holds_first_import(src: &str, bound: &str) -> bool {
        let source = parse(src);
        let body = &source.ast().body;
        let alias = &body[0].as_import_from_stmt().expect("a from import").names[0];
        Reexports::of(body).holds(alias, bound)
    }

    #[rstest]
    #[case::bare("import __all__\n", true)]
    #[case::member("from pkg import __all__\n", true)]
    #[case::aliased("from pkg import names as __all__\n", true)]
    #[case::nested("if flag:\n    from pkg import __all__\n", true)]
    #[case::other_member("from pkg import names\n", false)]
    fn an_import_binding_dunder_all_leaves_the_surface_unreadable(
        #[case] src: &str,
        #[case] holds: bool,
    ) {
        let source = format!("from json import dumps\n{src}__all__ = [\"other\"]\n");

        assert_eq!(holds_first_import(&source, "dumps"), holds);
    }

    #[rstest]
    #[case::conditional("if flag:\n    __all__ = [\"other\"]\n")]
    #[case::function_scope("def setup():\n    global __all__\n    __all__ = [\"other\"]\n")]
    #[case::class_scope("class C:\n    __all__ = [\"other\"]\n")]
    #[case::loop_body("for _ in xs:\n    __all__ = [\"other\"]\n")]
    #[case::try_handler("try:\n    pass\nexcept E:\n    __all__ = [\"other\"]\n")]
    #[case::call("__all__ = build()\n")]
    #[case::name("__all__ = EXPORTS\n")]
    #[case::non_literal_item("__all__ = [name]\n")]
    #[case::append("__all__ = []\n__all__.append(\"other\")\n")]
    #[case::extend("__all__ = []\n__all__.extend(other)\n")]
    #[case::slice_assignment("__all__ = []\n__all__[:] = [\"other\"]\n")]
    #[case::subscript_assignment("__all__ = [\"a\"]\n__all__[0] = \"other\"\n")]
    fn an_unreadable_surface_holds_every_name(#[case] src: &str) {
        assert!(holds_first_import(
            &format!("from json import dumps\n{src}"),
            "dumps",
        ));
    }

    #[rstest]
    #[case::listed("__all__ = [\"loads\"]\n", true)]
    #[case::empty_list("__all__ = []\n", true)]
    #[case::augmented_only("__all__ += [\"loads\"]\n", true)]
    #[case::unreadable("__all__ = build()\n", true)]
    #[case::nested("if flag:\n    __all__ = [\"loads\"]\n", true)]
    #[case::bare_annotation("__all__: list[str]\n", false)]
    #[case::other_name("__slots__ = [\"loads\"]\n", false)]
    #[case::no_write("value = 1\n", false)]
    fn declares_a_surface_reads_whether_the_module_writes_dunder_all(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        let source = parse(src);
        assert_eq!(
            Reexports::of(&source.ast().body).declares_a_surface(),
            expected,
        );
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
        assert_eq!(
            holds_first_import(&format!("from json import loads\n{src}"), "loads"),
            holds,
        );
    }

    #[rstest]
    #[case::private_module("from _ssl import OPENSSL_VERSION\n", true)]
    #[case::private_submodule("from pkg._impl import thing\n", true)]
    #[case::relative_private("from ._impl import thing\n", true)]
    #[case::parent_relative_private("from .._impl import thing\n", true)]
    #[case::public_module("from pkg.impl import thing\n", false)]
    #[case::public_leaf_of_private("from _pkg.sub import thing\n", false)]
    #[case::dunder_module("from __future__ import annotations\n", false)]
    #[case::dots_only("from . import thing\n", false)]
    #[case::bare_import("import _socket\n", false)]
    fn reexports_a_private_member_reads_the_module_the_names_come_from(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        let source = parse(src);
        let node = ImportNode::of(&source.ast().body[0]).expect("an import statement");
        assert_eq!(reexports_a_private_member(&node), expected);
    }
}
