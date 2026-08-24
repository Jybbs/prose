//! Classifies import statements into the canonical group order
//! `__future__` → bare → external `from` → local-package, finds the runs
//! of adjacent imports the ordering rules act on, builds the composite
//! sort key ordering a run within and across those groups, counts the
//! canonical blank lines dividing two imports, and shapes the deletions
//! that drop the aliases a rule has left unread. First-party detection
//! reads the package-name list from `[tool.prose.imports]`.

use ruff_python_ast::{Stmt, StmtImportFrom};

mod grouping;
mod pruning;
mod runs;

pub(crate) use grouping::{ModuleKey, import_group, import_sort_key, module_key};
pub(crate) use pruning::{Dropping, fold_landing, prune_import_statements, stands_alone};
pub(crate) use runs::{
    defers_annotations, future_annotations_alias, import_blank_lines, import_runs,
    sectioned_import_runs,
};

use runs::lines_under_blank_run;

const FUTURE_ANNOTATIONS: &str = "annotations";
const FUTURE_MODULE: &str = "__future__";

/// Display width of the `import ` keyword and its trailing space, the
/// distance from an aligned `import` column to the first member.
pub(crate) const IMPORT_KEYWORD_WIDTH: usize = "import ".len();

/// True for an absolute `from __future__ import …` statement.
pub(crate) fn is_future(node: &StmtImportFrom) -> bool {
    node.level == 0 && node.module.as_deref() == Some(FUTURE_MODULE)
}

/// True for an `import` or `from`-import statement.
pub(crate) fn is_import(stmt: &Stmt) -> bool {
    stmt.is_import_stmt() || stmt.is_import_from_stmt()
}
