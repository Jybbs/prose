//! When a module's annotations run, the timing that decides which
//! annotations count among the names a definition reads as it binds.

use ruff_python_ast::{PythonVersion, Stmt, StmtClassDef};

use crate::primitives::imports::defers_annotations;

/// When the annotations of a module run, which decides whether an
/// annotation's names count among the names a definition reads as it
/// binds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Annotations {
    /// Each annotation runs as the statement holding it runs.
    Eager,
    /// Each annotation runs on first access under PEP 649 and PEP 749,
    /// which a class reads at creation only where its header runs code.
    Lazy,
    /// `from __future__ import annotations` stores each annotation as a
    /// string.
    Stringified,
}

impl Annotations {
    /// The timing of the module `body` under `target`, the future import
    /// taking precedence over a target from Python 3.14 on.
    pub(crate) fn of(body: &[Stmt], target: Option<PythonVersion>) -> Self {
        if defers_annotations(body) {
            Self::Stringified
        } else if target.is_some_and(PythonVersion::defers_annotations) {
            Self::Lazy
        } else {
            Self::Eager
        }
    }

    /// True when the annotations of `class`'s body run while the class
    /// is created. A lazy annotation runs there only where the header
    /// carries a decorator, a base, or a keyword, through which a
    /// decorator, a metaclass, or an `__init_subclass__` hook can read
    /// `__annotations__`.
    pub(super) fn read_by(self, class: &StmtClassDef) -> bool {
        match self {
            Self::Eager => true,
            Self::Lazy => {
                !class.decorator_list.is_empty()
                    || !class.bases().is_empty()
                    || !class.keywords().is_empty()
            }
            Self::Stringified => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::parse;

    #[rstest]
    #[case::future_import(
        "from __future__ import annotations\n",
        Some(PythonVersion::PY314),
        Annotations::Stringified
    )]
    #[case::target_from_3_14("x = 1\n", Some(PythonVersion::PY314), Annotations::Lazy)]
    #[case::target_below_3_14("x = 1\n", Some(PythonVersion::PY313), Annotations::Eager)]
    #[case::no_target("x = 1\n", None, Annotations::Eager)]
    fn annotations_of_reads_the_future_import_ahead_of_the_target(
        #[case] src: &str,
        #[case] target: Option<PythonVersion>,
        #[case] expected: Annotations,
    ) {
        let source = parse(src);
        assert_eq!(Annotations::of(&source.ast().body, target), expected);
    }
}
