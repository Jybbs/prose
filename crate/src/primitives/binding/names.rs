//! Pure name helpers over import, assignment, and reference AST nodes:
//! target extraction, the `SCREAMING_CASE` casing predicate, and the
//! `TYPE_CHECKING` guard, independent of the binding table.

use ruff_python_ast::{Alias, Expr, ExprName, Identifier, Stmt, StmtAnnAssign, StmtAssign, StmtIf};

/// The bare-`Name` target name of an `Stmt::AnnAssign`. `None` when the
/// target is an attribute or subscript (`self.x: int`, `d[k]: int`).
pub(crate) fn annotated_name_target(ann: &StmtAnnAssign) -> Option<&str> {
    Some(annotated_name_target_expr(ann)?.id.as_str())
}

/// The bare-`Name` target node of an `Stmt::AnnAssign`, carrying its
/// range. `None` when the target is an attribute or subscript.
fn annotated_name_target_expr(ann: &StmtAnnAssign) -> Option<&ExprName> {
    ann.target.as_name_expr()
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

/// Returns `true` when `id` begins with an ASCII uppercase letter and
/// every remaining character is an ASCII uppercase letter, digit, or
/// underscore. A leading underscore fails the first test, so dunder and
/// private names never qualify. `ruff_python_stdlib::str::is_cased_uppercase`
/// reads `_HIDDEN` as uppercase, so it cannot stand in where a leading
/// underscore must disqualify the name.
pub(crate) fn is_screaming_case(id: &str) -> bool {
    let mut chars = id.chars();
    chars.next().is_some_and(|c| c.is_ascii_uppercase())
        && chars.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// Returns `true` when `stmt.test` matches the bare `TYPE_CHECKING`
/// name or any `<...>.TYPE_CHECKING` attribute access.
fn is_type_checking_block(stmt: &StmtIf) -> bool {
    tail_identifier(stmt.test.as_ref()) == Some("TYPE_CHECKING")
}

/// The single bare-`Name` target of an `Stmt::Assign` or
/// `Stmt::AnnAssign`, paired with its value and, for an annotated
/// assignment, its annotation. The value is `None` for a bare annotation
/// (`X: int`). `None` for any other statement or a non-single-name target.
pub(crate) fn single_name_assignment(
    stmt: &Stmt,
) -> Option<(&ExprName, Option<&Expr>, Option<&Expr>)> {
    match stmt {
        Stmt::Assign(a) => Some((single_name_target_expr(a)?, Some(a.value.as_ref()), None)),
        Stmt::AnnAssign(a) => Some((
            annotated_name_target_expr(a)?,
            a.value.as_deref(),
            Some(a.annotation.as_ref()),
        )),
        _ => None,
    }
}

/// The single bare-`Name` target name of an `Stmt::Assign`. `None` for
/// a multi-target, destructuring, attribute, or subscript assignment.
pub(crate) fn single_name_target(assign: &StmtAssign) -> Option<&str> {
    Some(single_name_target_expr(assign)?.id.as_str())
}

/// The single bare-`Name` target node of an `Stmt::Assign`, carrying
/// its range. `None` for a multi-target, destructuring, attribute, or
/// subscript assignment.
fn single_name_target_expr(assign: &StmtAssign) -> Option<&ExprName> {
    match assign.targets.as_slice() {
        [Expr::Name(name)] => Some(name),
        _ => None,
    }
}

/// True for a statement the module-constant scan does not descend into:
/// a `def` or `class` scope, or an `if TYPE_CHECKING:` block.
pub(crate) fn skips_module_scan(stmt: &Stmt) -> bool {
    matches!(stmt, Stmt::FunctionDef(_) | Stmt::ClassDef(_))
        || matches!(stmt, Stmt::If(s) if is_type_checking_block(s))
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
    use rstest::rstest;

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

    #[rstest]
    fn is_screaming_case_accepts_canonical_constants(
        #[values("PI", "MAX_RETRIES", "X1", "LOG_LEVEL_2")] id: &str,
    ) {
        assert!(is_screaming_case(id));
    }

    #[rstest]
    fn is_screaming_case_rejects_mixed_and_lowercase_names(
        #[values("", "pi", "Pi", "pI", "_HIDDEN", "1ABC", "MAX_retries")] id: &str,
    ) {
        assert!(!is_screaming_case(id));
    }

    #[rstest]
    #[case("if TYPE_CHECKING:\n    x = 1\n", true)]
    #[case("if typing.TYPE_CHECKING:\n    x = 1\n", true)]
    #[case("if DEBUG:\n    x = 1\n", false)]
    fn is_type_checking_block_matches_bare_and_qualified(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        let source = parse(src);
        let if_stmt = source.ast().body[0].as_if_stmt().expect("an if statement");
        assert_eq!(is_type_checking_block(if_stmt), expected);
    }

    #[test]
    fn single_name_assignment_extracts_target_value_and_annotation() {
        let source = parse("X = 1\ny: int = 2\nz: int\nself.x = 1\na, b = 1, 2\n");
        let shapes: Vec<Option<(&str, bool, bool)>> = source
            .ast()
            .body
            .iter()
            .map(|stmt| {
                single_name_assignment(stmt).map(|(target, value, annotation)| {
                    (target.id.as_str(), value.is_some(), annotation.is_some())
                })
            })
            .collect();
        assert_eq!(
            shapes,
            vec![
                Some(("X", true, false)),
                Some(("y", true, true)),
                Some(("z", false, true)),
                None,
                None,
            ],
        );
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
