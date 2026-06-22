//! Pure name-extraction helpers over import and assignment AST nodes,
//! independent of the binding table.

use ruff_python_ast::{Alias, Expr, Identifier, StmtAnnAssign, StmtAssign};

/// The bare-`Name` target name of an `Stmt::AnnAssign`. `None` when the
/// target is an attribute or subscript (`self.x: int`, `d[k]: int`).
pub(crate) fn annotated_name_target(ann: &StmtAnnAssign) -> Option<&str> {
    Some(ann.target.as_name_expr()?.id.as_str())
}

/// The module-scope name a bare `import a.b` alias binds: its `asname`,
/// or the top-level segment of the dotted path.
pub(crate) fn bare_import_bound_name(alias: &Alias) -> &str {
    alias
        .asname
        .as_ref()
        .map_or_else(|| top_level_module(alias.name.as_str()), Identifier::as_str)
}

/// The name a `from m import x` alias binds: its `asname`, or the
/// imported name itself.
pub(crate) fn from_import_bound_name(alias: &Alias) -> &str {
    alias.asname.as_ref().unwrap_or(&alias.name).as_str()
}

/// The single bare-`Name` target name of an `Stmt::Assign`. `None` for
/// a multi-target, destructuring, attribute, or subscript assignment.
pub(crate) fn single_name_target(assign: &StmtAssign) -> Option<&str> {
    match assign.targets.as_slice() {
        [Expr::Name(name)] => Some(name.id.as_str()),
        _ => None,
    }
}

/// Returns the trailing identifier of a name reference: the bound name
/// of a bare `Name` or the attribute of an `Attribute` access. `None`
/// for any other expression.
pub(crate) fn tail_identifier(expr: &Expr) -> Option<&str> {
    match expr {
        Expr::Attribute(attr) => Some(attr.attr.as_str()),
        Expr::Name(name) => Some(name.id.as_str()),
        _ => None,
    }
}

/// Returns the segment of `dotted` before the first `.`. Matches
/// Python's `import a.b.c` shape, which binds `a` rather than the
/// full dotted path.
pub(crate) fn top_level_module(dotted: &str) -> &str {
    dotted.split_once('.').map_or(dotted, |(head, _)| head)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::parse;

    #[test]
    fn annotated_name_target_keeps_only_name_targets() {
        let source = parse("x: int = 1\nself.x: int = 1\n");
        let targets: Vec<Option<&str>> = source
            .ast()
            .body
            .iter()
            .map(|stmt| annotated_name_target(stmt.as_ann_assign_stmt().expect("ann assign")))
            .collect();
        assert_eq!(targets, vec![Some("x"), None]);
    }

    #[test]
    fn single_name_target_keeps_only_single_name_assignments() {
        let source = parse("X = 1\nself.x = 1\nx, y = 1, 2\n");
        let targets: Vec<Option<&str>> = source
            .ast()
            .body
            .iter()
            .map(|stmt| single_name_target(stmt.as_assign_stmt().expect("assign")))
            .collect();
        assert_eq!(targets, vec![Some("X"), None, None]);
    }

    #[test]
    fn top_level_module_returns_first_segment() {
        assert_eq!(top_level_module("a"), "a");
        assert_eq!(top_level_module("a.b"), "a");
        assert_eq!(top_level_module("a.b.c"), "a");
        assert_eq!(top_level_module(""), "");
    }
}
