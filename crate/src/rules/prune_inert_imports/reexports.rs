//! The explicit re-export surface of a module, read from its
//! module-scope `__all__` writes, from the PEP 484 `x as x` alias form,
//! and from a `noqa` comment trailing an import statement.

use ruff_python_ast::{Alias, Expr, Stmt};
use ruff_text_size::Ranged;
use rustc_hash::FxHashSet;

use super::inventory::is_self_alias;
use crate::{
    primitives::{
        binding::{sequence_elts, single_name_assignment},
        comments::trailing_comment,
        scope::sub_bodies,
        walk::any_over_stmts,
    },
    source::Source,
};

const DUNDER_ALL: &str = "__all__";

/// The code `flake8` and its successors report an unread import under.
const F401: &str = "F401";

/// The marker a suppression comment opens with, matched case-insensitively.
const NOQA: &str = "noqa";

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

/// True where a `noqa` comment trails `stmt`, either bare or naming
/// `F401`, which marks every unread name the statement binds deliberate.
pub(super) fn noqa_holds_imports(source: &Source, stmt: &Stmt) -> bool {
    trailing_comment(source, stmt.start()).is_some_and(|range| {
        noqa_codes(source.slice(range)).is_some_and(|codes| {
            codes.is_empty() || codes.iter().any(|code| code.eq_ignore_ascii_case(F401))
        })
    })
}

/// The codes a `noqa` comment names, empty for the bare form covering
/// every code, `None` where the comment carries no `noqa` at all.
fn noqa_codes(comment: &str) -> Option<Vec<&str>> {
    let lowered = comment.to_ascii_lowercase();
    let opened = lowered.find(NOQA)? + NOQA.len();
    let Some(listed) = comment[opened..].trim_start().strip_prefix(':') else {
        return Some(Vec::new());
    };
    Some(
        listed
            .split(',')
            .map(str::trim)
            .take_while(|code| code.chars().all(char::is_alphanumeric) && !code.is_empty())
            .collect(),
    )
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

    #[rstest]
    #[case::bare("  # noqa", true)]
    #[case::listed("  # noqa: F401", true)]
    #[case::unspaced("  # noqa:F401", true)]
    #[case::lowercase("  # noqa: f401", true)]
    #[case::among_others("  # noqa: E501, F401", true)]
    #[case::uppercase_marker("  # NOQA: F401", true)]
    #[case::trailing_prose("  # noqa: F401 kept for re-export", true)]
    #[case::other_code("  # noqa: E501", false)]
    #[case::plain_comment("  # kept for re-export", false)]
    #[case::no_comment("", false)]
    fn a_noqa_comment_holds_every_name_its_import_binds(
        #[case] comment: &str,
        #[case] holds: bool,
    ) {
        let source = parse(&format!("from _sre import MAXREPEAT, MAXGROUPS{comment}\n"));
        let stmt = &source.ast().body[0];
        assert_eq!(noqa_holds_imports(&source, stmt), holds);
    }

    #[test]
    fn a_noqa_on_one_import_leaves_its_neighbour_alone() {
        let source = parse("import os  # noqa: F401\nimport sys\n");
        let body = &source.ast().body;
        assert!(noqa_holds_imports(&source, &body[0]));
        assert!(!noqa_holds_imports(&source, &body[1]));
    }
}
