//! The names a module lists in its module-scope `__all__`, read from
//! every statement that writes or binds it.

use std::slice;

use ruff_python_ast::{
    Expr, Stmt,
    helpers::{any_over_body, any_over_expr},
};
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
    /// A write no static read settles, a method call on `__all__` anywhere
    /// in the module, a write inside a compound statement, or a statement
    /// other than an assignment binding the name.
    Unreadable,
}

impl<'a> DunderAll<'a> {
    /// Reads every statement in `body` that writes or binds `__all__` into
    /// one surface.
    pub(crate) fn of(body: &'a [Stmt]) -> Self {
        if any_over_body(body, mutates_dunder_all) {
            return Self::Unreadable;
        }
        let mut listed: Option<FxHashSet<&str>> = None;
        for stmt in body {
            match Write::of(stmt) {
                Some(Write::Names(items)) => listed.get_or_insert_default().extend(items),
                Some(Write::Unreadable) => return Self::Unreadable,
                None if binds_dunder_all(stmt) || nested_write(stmt) => return Self::Unreadable,
                None => {}
            }
        }
        listed.map_or(Self::Undeclared, Self::Listed)
    }

    pub(crate) fn declares_a_surface(&self) -> bool {
        !matches!(self, Self::Undeclared)
    }

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
    /// leaving it alone.
    fn of(stmt: &'a Stmt) -> Option<Self> {
        let (targets, value) = match stmt {
            Stmt::AnnAssign(node) => (slice::from_ref(&*node.target), node.value.as_deref()?),
            Stmt::Assign(node) => (node.targets.as_slice(), &*node.value),
            Stmt::AugAssign(node) => (slice::from_ref(&*node.target), &*node.value),
            Stmt::Global(node) if node.names.iter().any(|name| name == DUNDER_ALL) => {
                return Some(Self::Unreadable);
            }
            Stmt::Import(_) | Stmt::ImportFrom(_) => {
                return module_bound_names(stmt)
                    .contains(&DUNDER_ALL)
                    .then_some(Self::Unreadable);
            }
            _ => return None,
        };
        if targets.iter().any(names_dunder_all) {
            Some(string_items(value).map_or(Self::Unreadable, Self::Names))
        } else {
            targets
                .iter()
                .any(|target| any_over_expr(target, writes_dunder_all))
                .then_some(Self::Unreadable)
        }
    }
}

/// True when `stmt` binds `__all__`, skipping a bare annotation, which
/// binds nothing at run time.
fn binds_dunder_all(stmt: &Stmt) -> bool {
    stmt.as_ann_assign_stmt()
        .is_none_or(|node| node.value.is_some())
        && module_bound_names(stmt).contains(&DUNDER_ALL)
}

fn mutates_dunder_all(expr: &Expr) -> bool {
    expr.as_call_expr()
        .and_then(|call| call.func.as_attribute_expr())
        .is_some_and(|attribute| names_dunder_all(&attribute.value))
}

fn names_dunder_all(expr: &Expr) -> bool {
    expr.as_name_expr()
        .is_some_and(|name| name.id == DUNDER_ALL)
}

fn nested_write(stmt: &Stmt) -> bool {
    sub_bodies(stmt)
        .into_iter()
        .any(|(body, _)| any_over_stmts(body, |nested| Write::of(nested).is_some()))
}

fn string_items(value: &Expr) -> Option<Vec<&str>> {
    sequence_elts(value)?
        .iter()
        .map(|elt| Some(elt.as_string_literal_expr()?.value.to_str()))
        .collect()
}

fn writes_dunder_all(expr: &Expr) -> bool {
    match expr {
        Expr::Name(name) => name.ctx.is_store() && name.id == DUNDER_ALL,
        Expr::Subscript(subscript) => {
            subscript.ctx.is_store() && names_dunder_all(&subscript.value)
        }
        _ => false,
    }
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
    #[case::for_target("for __all__ in [[\"other\"]]:\n    pass\n")]
    #[case::with_target("with open(path) as __all__:\n    pass\n")]
    #[case::walrus("(__all__ := [\"other\"])\n")]
    #[case::deleted("__all__ = [\"a\"]\ndel __all__\n")]
    #[case::definition("def __all__():\n    pass\n")]
    #[case::function_for_target(
        "def setup():\n    global __all__\n    for __all__ in [[\"other\"]]:\n        pass\n"
    )]
    #[case::function_import("def setup():\n    from pkg import __all__\n")]
    #[case::annotated_walrus("names: list[str] = (__all__ := [\"other\"])\n")]
    #[case::augmented_subscript("__all__ = [\"a\"]\n__all__[0] += \"b\"\n")]
    #[case::annotated_subscript("__all__ = [\"a\"]\n__all__[0]: str = \"other\"\n")]
    #[case::comprehension_append("__all__ = []\n[__all__.append(name) for name in (\"other\",)]\n")]
    #[case::assigned_append("__all__ = []\n_ = __all__.append(\"other\")\n")]
    #[case::lambda_append("__all__ = []\nregister = lambda name: __all__.append(name)\n")]
    #[case::conditional_append("__all__ = []\nif flag:\n    __all__.append(\"other\")\n")]
    #[case::function_append("__all__ = []\ndef register():\n    __all__.append(\"other\")\n")]
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
    #[case::read_in_a_target("__all__ = [\"loads\"]\nlookup[__all__[0]] = 1\n", true)]
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
