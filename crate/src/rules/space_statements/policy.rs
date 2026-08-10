//! Canonical blank-line counts per scope. Dispatches a `(prev,
//! curr)` pair to the class-, function-, or module-scope policy.

use ruff_python_ast::{Stmt, helpers::is_docstring_stmt};

use crate::primitives::{blanks::module_blank_lines, scope::BodyScope};

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
    grouped: bool,
) -> Option<u32> {
    match scope {
        BodyScope::Class => class_scope_blanks(prev, curr),
        BodyScope::Function => function_scope_blanks(prev, curr),
        BodyScope::Module => module_blank_lines(prev, curr, first_party, grouped),
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

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{first_class, first_def, parse};

    #[test]
    fn canonical_blanks_class_docstring_predecessor_returns_one() {
        let s = parse("class C:\n    '''doc'''\n    def m1(self): pass\n");
        let class = first_class(&s);
        assert_eq!(
            canonical_blanks(&class.body[0], &class.body[1], BodyScope::Class, &[], true),
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
            canonical_blanks(&class.body[0], &class.body[1], BodyScope::Class, &[], true),
            Some(1),
        );
    }

    #[test]
    fn canonical_blanks_class_header_to_docstring_returns_zero() {
        let s = parse("class C:\n    '''doc'''\n    pass\n");
        let class = first_class(&s);
        assert_eq!(
            canonical_blanks(
                &s.ast().body[0],
                &class.body[0],
                BodyScope::Class,
                &[],
                true
            ),
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
            canonical_blanks(
                &s.ast().body[0],
                &class.body[0],
                BodyScope::Class,
                &[],
                true
            ),
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
            canonical_blanks(
                &s.ast().body[0],
                &func.body[0],
                BodyScope::Function,
                &[],
                true
            ),
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
            canonical_blanks(
                &s.ast().body[0],
                &func.body[0],
                BodyScope::Function,
                &[],
                true
            ),
            None,
        );
    }

    #[test]
    fn canonical_blanks_in_class_body_pairs_method_after_method_to_one() {
        let s = parse("class C:\n    def m1(self): pass\n    def m2(self): pass\n");
        let class = first_class(&s);
        assert_eq!(
            canonical_blanks(&class.body[0], &class.body[1], BodyScope::Class, &[], true),
            Some(1),
        );
    }

    #[test]
    fn canonical_blanks_in_function_body_returns_none() {
        let s = parse("def f():\n    x = 1\n    y = 2\n");
        let func = first_def(&s);
        assert_eq!(
            canonical_blanks(&func.body[0], &func.body[1], BodyScope::Function, &[], true),
            None,
        );
    }

    #[rstest]
    #[case("def f(): pass\nPORT = 8080\n", Some(2))]
    #[case("import os\nPORT = 8080\n", Some(1))]
    #[case("x = 1\ny = 2\n", None)]
    fn canonical_blanks_routes_module_scope_to_the_shared_policy(
        #[case] src: &str,
        #[case] expected: Option<u32>,
    ) {
        let s = parse(src);
        let body = &s.ast().body;
        assert_eq!(
            canonical_blanks(&body[0], &body[1], BodyScope::Module, &[], true),
            expected,
        );
    }
}
