//! Canonical blank-line counts per scope. Dispatches a `(prev,
//! curr)` pair to the class-, function-, or module-scope policy.

use ruff_python_ast::{CmpOp, Expr, Stmt, helpers::is_docstring_stmt};

use crate::primitives::{imports::import_group, scope::BodyScope};

/// Returns the canonical blank-line count for the pair `(prev, curr)`
/// at `scope`. `None` means no case applies and the pair is skipped,
/// leaving any in-gap whitespace and comments untouched. For `Class`
/// and `Function` scopes, the pair includes the scope-entry transition
/// wherein `prev` is the enclosing `ClassDef` or `FunctionDef` itself
/// and `curr` is the first body member.
pub(super) fn canonical_blanks(
    prev: &Stmt,
    curr: &Stmt,
    scope: BodyScope,
    first_party: &[String],
) -> Option<u32> {
    match scope {
        BodyScope::Class => class_scope_blanks(prev, curr),
        BodyScope::Function => function_scope_blanks(prev, curr),
        BodyScope::Module => module_scope_blanks(prev, curr, first_party),
    }
}

/// Class-scope pair dispatch. The class header pairs with its first
/// body member, with 0 blank lines before a docstring and 1 otherwise.
/// Class-field → method and method-after-method pairs carry 1. Any
/// docstring-predecessor pair carries 1.
fn class_scope_blanks(prev: &Stmt, curr: &Stmt) -> Option<u32> {
    match (prev, curr) {
        (Stmt::ClassDef(_), _) => Some(u32::from(!is_docstring_stmt(curr))),
        (Stmt::FunctionDef(_) | Stmt::AnnAssign(_) | Stmt::Assign(_), Stmt::FunctionDef(_)) => {
            Some(1)
        }
        _ if is_docstring_stmt(prev) => Some(1),
        _ => None,
    }
}

/// Function-scope pair dispatch. The function header carries 1 blank
/// line before its first body statement when that statement is a
/// compound-body opener.
fn function_scope_blanks(prev: &Stmt, curr: &Stmt) -> Option<u32> {
    match (prev, curr) {
        (
            Stmt::FunctionDef(_),
            Stmt::For(_)
            | Stmt::If(_)
            | Stmt::Match(_)
            | Stmt::Try(_)
            | Stmt::While(_)
            | Stmt::With(_),
        ) => Some(1),
        _ => None,
    }
}

/// True when `stmt` is `if __name__ == "__main__":`.
fn is_main_guard(stmt: &Stmt) -> bool {
    let Some(if_stmt) = stmt.as_if_stmt() else {
        return false;
    };
    let Some(cmp) = if_stmt.test.as_compare_expr() else {
        return false;
    };
    let ([CmpOp::Eq], Some(left), Some(right)) = (
        cmp.ops.as_ref(),
        cmp.left.as_name_expr(),
        cmp.comparators
            .first()
            .and_then(Expr::as_string_literal_expr),
    ) else {
        return false;
    };
    left.id == "__name__" && right.value.to_str() == "__main__"
}

/// Module-scope pair dispatch. A statement following an
/// `if __name__ == "__main__":` block carries 1 blank line. A pair of
/// import statements lands 1 blank line when they fall in different
/// canonical groups (bare, external `from`, local-package) and none
/// when they share a group. A top-level `FunctionDef` or `ClassDef`
/// carries 2 blank lines before it. An `Assign` or `AnnAssign`
/// following a top-level `FunctionDef` or `ClassDef` carries 2.
fn module_scope_blanks(prev: &Stmt, curr: &Stmt, first_party: &[String]) -> Option<u32> {
    if is_main_guard(prev) {
        return Some(1);
    }
    if let Some((prev_group, curr_group)) =
        import_group(prev, first_party).zip(import_group(curr, first_party))
    {
        return (prev_group != curr_group).then_some(1);
    }
    match (prev, curr) {
        (_, Stmt::FunctionDef(_) | Stmt::ClassDef(_)) => Some(2),
        (Stmt::FunctionDef(_) | Stmt::ClassDef(_), Stmt::Assign(_) | Stmt::AnnAssign(_)) => Some(2),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{first_class, first_def, parse};

    fn main_guard_src() -> &'static str {
        "if __name__ == \"__main__\":\n    main()\n"
    }

    #[test]
    fn canonical_blanks_class_docstring_predecessor_returns_one() {
        let s = parse("class C:\n    '''doc'''\n    def m1(self): pass\n");
        let class = first_class(&s);
        assert_eq!(
            canonical_blanks(&class.body[0], &class.body[1], BodyScope::Class, &[]),
            Some(1),
        );
    }

    #[rstest]
    fn canonical_blanks_class_field_to_method_returns_one(
        #[values(
            "class C:\n    x: int = 1\n    def m(self): pass\n",
            "class C:\n    x = 1\n    def m(self): pass\n"
        )]
        src: &str,
    ) {
        let s = parse(src);
        let class = first_class(&s);
        assert_eq!(
            canonical_blanks(&class.body[0], &class.body[1], BodyScope::Class, &[]),
            Some(1),
        );
    }

    #[test]
    fn canonical_blanks_class_header_to_docstring_returns_zero() {
        let s = parse("class C:\n    '''doc'''\n    pass\n");
        let class = first_class(&s);
        assert_eq!(
            canonical_blanks(&s.ast().body[0], &class.body[0], BodyScope::Class, &[]),
            Some(0),
        );
    }

    #[rstest]
    fn canonical_blanks_class_header_to_first_member_returns_one(
        #[values(
            "class C:\n    def m(self): pass\n",
            "class C:\n    @decorator\n    def m(self): pass\n",
            "class C:\n    x: int = 1\n",
            "class C:\n    x = 1\n",
            "class C:\n    class Inner:\n        pass\n"
        )]
        src: &str,
    ) {
        let s = parse(src);
        let class = first_class(&s);
        assert_eq!(
            canonical_blanks(&s.ast().body[0], &class.body[0], BodyScope::Class, &[]),
            Some(1),
        );
    }

    #[rstest]
    fn canonical_blanks_function_header_to_compound_body_returns_one(
        #[values(
            "def f():\n    for x in y:\n        pass\n",
            "def f():\n    if x:\n        pass\n",
            "def f():\n    match x:\n        case _: pass\n",
            "def f():\n    try:\n        pass\n    except Exception:\n        pass\n",
            "def f():\n    while x:\n        pass\n",
            "def f():\n    with x:\n        pass\n",
            "async def f():\n    async for x in y:\n        pass\n",
            "async def f():\n    async with x:\n        pass\n"
        )]
        src: &str,
    ) {
        let s = parse(src);
        let func = first_def(&s);
        assert_eq!(
            canonical_blanks(&s.ast().body[0], &func.body[0], BodyScope::Function, &[]),
            Some(1),
        );
    }

    #[rstest]
    fn canonical_blanks_function_header_to_simple_stmt_returns_none(
        #[values(
            "def f():\n    x = 1\n",
            "def f():\n    return None\n",
            "def f():\n    '''doc'''\n",
            "def f():\n    def inner(): pass\n"
        )]
        src: &str,
    ) {
        let s = parse(src);
        let func = first_def(&s);
        assert_eq!(
            canonical_blanks(&s.ast().body[0], &func.body[0], BodyScope::Function, &[]),
            None,
        );
    }

    #[test]
    fn canonical_blanks_in_class_body_pairs_method_after_method_to_one() {
        let s = parse("class C:\n    def m1(self): pass\n    def m2(self): pass\n");
        let class = first_class(&s);
        assert_eq!(
            canonical_blanks(&class.body[0], &class.body[1], BodyScope::Class, &[]),
            Some(1),
        );
    }

    #[test]
    fn canonical_blanks_in_function_body_returns_none() {
        let s = parse("def f():\n    x = 1\n    y = 2\n");
        let func = first_def(&s);
        assert_eq!(
            canonical_blanks(&func.body[0], &func.body[1], BodyScope::Function, &[]),
            None,
        );
    }

    #[test]
    fn canonical_blanks_module_after_main_guard_returns_one() {
        let s = parse(&format!("{}xs = 1\n", main_guard_src()));
        let body = &s.ast().body;
        assert_eq!(
            canonical_blanks(&body[0], &body[1], BodyScope::Module, &[]),
            Some(1)
        );
    }

    #[rstest]
    fn canonical_blanks_module_assignment_after_def_or_class_returns_two(
        #[values(
            "class C: pass\nPORT = 8080\n",
            "class C: pass\nPORT: int = 8080\n",
            "def f(): pass\nPORT = 8080\n",
            "def f(): pass\nPORT: int = 8080\n"
        )]
        src: &str,
    ) {
        let s = parse(src);
        let body = &s.ast().body;
        assert_eq!(
            canonical_blanks(&body[0], &body[1], BodyScope::Module, &[]),
            Some(2),
        );
    }

    #[test]
    fn canonical_blanks_module_class_after_module_stmt_returns_two() {
        let s = parse("x = 1\nclass C: pass\n");
        let body = &s.ast().body;
        assert_eq!(
            canonical_blanks(&body[0], &body[1], BodyScope::Module, &[]),
            Some(2)
        );
    }

    #[test]
    fn canonical_blanks_module_def_after_module_stmt_returns_two() {
        let s = parse("x = 1\ndef f(): pass\n");
        let body = &s.ast().body;
        assert_eq!(
            canonical_blanks(&body[0], &body[1], BodyScope::Module, &[]),
            Some(2)
        );
    }

    #[rstest]
    #[case("from os import path\nfrom . import x\n", &[], Some(1))]
    #[case("from os import path\nfrom myapp import x\n", &["myapp"], Some(1))]
    #[case("import os\nimport myapp\n", &["myapp"], Some(1))]
    #[case("import myapp\nfrom myapp import x\n", &["myapp"], None)]
    #[case("from myapp import a\nfrom myapp.db import b\n", &["myapp"], None)]
    fn canonical_blanks_module_import_group_boundary_separates_distinct_groups(
        #[case] src: &str,
        #[case] first_party: &[&str],
        #[case] expected: Option<u32>,
    ) {
        let list: Vec<String> = first_party.iter().map(|&s| s.to_owned()).collect();
        let s = parse(src);
        let body = &s.ast().body;
        assert_eq!(
            canonical_blanks(&body[0], &body[1], BodyScope::Module, &list),
            expected,
        );
    }

    #[rstest]
    fn canonical_blanks_module_import_kind_boundary_returns_one(
        #[values(
            "import os\nfrom sys import argv\n",
            "from sys import argv\nimport os\n"
        )]
        src: &str,
    ) {
        let s = parse(src);
        let body = &s.ast().body;
        assert_eq!(
            canonical_blanks(&body[0], &body[1], BodyScope::Module, &[]),
            Some(1),
        );
    }

    #[rstest]
    fn canonical_blanks_module_same_kind_import_run_returns_none(
        #[values(
            "import os\nimport sys\n",
            "from os import path\nfrom sys import argv\n"
        )]
        src: &str,
    ) {
        let s = parse(src);
        let body = &s.ast().body;
        assert_eq!(
            canonical_blanks(&body[0], &body[1], BodyScope::Module, &[]),
            None
        );
    }

    #[test]
    fn canonical_blanks_unrelated_pair_returns_none() {
        let s = parse("x = 1\ny = 2\n");
        let body = &s.ast().body;
        assert_eq!(
            canonical_blanks(&body[0], &body[1], BodyScope::Module, &[]),
            None
        );
    }

    #[test]
    fn is_main_guard_accepts_canonical_form() {
        let s = parse(main_guard_src());
        assert!(is_main_guard(&s.ast().body[0]));
    }

    #[test]
    fn is_main_guard_rejects_non_if_statements() {
        let s = parse("x = 1\n");
        assert!(!is_main_guard(&s.ast().body[0]));
    }

    #[rstest]
    fn is_main_guard_rejects_other_if_conditions(
        #[values(
            "if x:\n    pass\n",
            "if __name__ != \"__main__\":\n    pass\n",
            "if __name__ == \"main\":\n    pass\n",
            "if other == \"__main__\":\n    pass\n",
            "if __name__ == __main__:\n    pass\n",
            "if __name__ == \"__main__\" and x:\n    pass\n"
        )]
        src: &str,
    ) {
        let s = parse(src);
        assert!(!is_main_guard(&s.ast().body[0]));
    }
}
