//! The names a module lists in its module-scope `__all__`, read from
//! every write to it and from an import binding it.

use ruff_python_ast::{Expr, Stmt, helpers::any_over_expr};
use rustc_hash::FxHashSet;

use crate::primitives::{
    binding::{module_bound_names, sequence_elts},
    scope::sub_bodies,
    walk::any_over_stmts,
};

const DUNDER_ALL: &str = "__all__";

/// The export surface of a module, read from its `__all__` writes.
pub(crate) enum DunderAll<'a> {
    /// Every write reads as a list or tuple of string literals.
    Listed(FxHashSet<&'a str>),
    /// The module writes `__all__` nowhere.
    Undeclared,
    /// A write no static read settles, a write below module scope, or
    /// an import binding the name.
    Unreadable,
}

impl<'a> DunderAll<'a> {
    /// Reads every `__all__` write in `body` and every import binding the
    /// name into one surface, which is `Unreadable` once a write cannot be
    /// listed statically, sits below module scope, or comes through an
    /// import, and `Undeclared` where the module writes none.
    pub(crate) fn of(body: &'a [Stmt]) -> Self {
        let mut listed: Option<FxHashSet<&str>> = None;
        for stmt in body {
            match Write::of(stmt) {
                Some(Write::Names(items)) => listed.get_or_insert_default().extend(items),
                Some(Write::Unreadable) => return Self::Unreadable,
                None if nested_write(stmt) => return Self::Unreadable,
                None => {}
            }
        }
        listed.map_or(Self::Undeclared, Self::Listed)
    }

    /// True when the module writes `__all__` anywhere or binds the
    /// name from another module, whatever the write lists.
    pub(crate) fn declares_a_surface(&self) -> bool {
        !matches!(self, Self::Undeclared)
    }

    /// True when `name` is listed, or when the surface is unreadable and
    /// so holds every name.
    pub(crate) fn exports(&self, name: &str) -> bool {
        match self {
            Self::Listed(names) => names.contains(name),
            Self::Undeclared => false,
            Self::Unreadable => true,
        }
    }
}

/// What one statement contributes to `__all__`.
enum Write<'a> {
    Names(Vec<&'a str>),
    Unreadable,
}

impl<'a> Write<'a> {
    /// Reads what `stmt` writes to `__all__`, `None` for a statement
    /// leaving it alone. A chained assignment reads as a single one does,
    /// whereas a subscript or unpacking holding `__all__`, a mutating
    /// call, and an import binding the name read as `Unreadable`.
    fn of(stmt: &'a Stmt) -> Option<Self> {
        let value = match stmt {
            Stmt::AnnAssign(node) if names_dunder_all(&node.target) => node.value.as_deref()?,
            Stmt::Assign(node) if node.targets.iter().any(names_dunder_all) => node.value.as_ref(),
            Stmt::Assign(node)
                if node
                    .targets
                    .iter()
                    .any(|target| any_over_expr(target, names_dunder_all)) =>
            {
                return Some(Self::Unreadable);
            }
            Stmt::AugAssign(node) if names_dunder_all(&node.target) => node.value.as_ref(),
            Stmt::Expr(node) if mutates_dunder_all(&node.value) => return Some(Self::Unreadable),
            Stmt::Import(_) | Stmt::ImportFrom(_) => {
                return module_bound_names(stmt)
                    .contains(&DUNDER_ALL)
                    .then_some(Self::Unreadable);
            }
            _ => return None,
        };
        Some(string_items(value).map_or(Self::Unreadable, Self::Names))
    }
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
        .is_some_and(|name| name.id == DUNDER_ALL)
}

/// True when a statement below `stmt`'s own level writes `__all__`,
/// covering a conditional branch, a loop body, and a `def` or `class`
/// scope alike.
fn nested_write(stmt: &Stmt) -> bool {
    sub_bodies(stmt)
        .into_iter()
        .any(|(body, _)| any_over_stmts(body, |nested| Write::of(nested).is_some()))
}

/// Returns the string-literal items of a list or tuple display, `None`
/// when `value` is another shape or carries an item that is not a string
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
    #[case::bare("import __all__\n")]
    #[case::member("from pkg import __all__\n")]
    #[case::aliased("from pkg import names as __all__\n")]
    #[case::nested("if flag:\n    from pkg import __all__\n")]
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
    #[case::unpacked("__all__, version = [\"other\"], 1\n")]
    #[case::starred("*__all__, version = [\"other\"], 1\n")]
    fn an_unreadable_surface_exports_every_name(#[case] src: &str) {
        let source = parse(src);
        let surface = DunderAll::of(&source.ast().body);

        assert!(surface.declares_a_surface());
        assert!(surface.exports("anything"));
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
            DunderAll::of(&source.ast().body).declares_a_surface(),
            expected
        );
    }

    #[rstest]
    #[case::list("__all__ = [\"loads\"]\n", true)]
    #[case::tuple("__all__ = (\"loads\",)\n", true)]
    #[case::annotated("__all__: list[str] = [\"loads\"]\n", true)]
    #[case::augmented("__all__ = []\n__all__ += [\"loads\"]\n", true)]
    #[case::augmented_only("__all__ += [\"loads\"]\n", true)]
    #[case::two_writes("__all__ = [\"dumps\"]\n__all__ += [\"loads\"]\n", true)]
    #[case::chained("__all__ = names = [\"loads\"]\n", true)]
    #[case::other_import("from pkg import names\n__all__ = [\"loads\"]\n", true)]
    #[case::read_in_an_expression("__all__ = [\"loads\"]\nprint(__all__)\n", true)]
    #[case::empty_list("__all__ = []\n", false)]
    #[case::other_item("__all__ = [\"dumps\"]\n", false)]
    #[case::bare_annotation("__all__: list[str]\n", false)]
    #[case::other_name("__slots__ = [\"loads\"]\n", false)]
    #[case::no_dunder_all("value = 1\n", false)]
    fn exports_reads_the_listed_names(#[case] src: &str, #[case] expected: bool) {
        let source = parse(src);
        let surface = DunderAll::of(&source.ast().body);

        assert_eq!(surface.exports("loads"), expected);
        assert!(!surface.exports("absent"));
    }
}
