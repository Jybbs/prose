//! Shared walker for PEP 257 docstring statements, the first
//! body-statement of the module, each class, and each function that
//! holds a string literal as its first expression statement.
//! [`walk_docstrings`] drives a caller's function across every such
//! literal in source order, paired with the definition that owns it,
//! and skips an implicitly concatenated docstring expression where
//! [`docstring_slots`] reports the slot whatever its part count. The
//! `body`, `entries`, `grammar`, and `scan` submodules carry the
//! text-level helpers for walking a docstring body directly.

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    ExprStringLiteral, Stmt, StringLiteral,
    statement_visitor::{StatementVisitor, walk_stmt},
};
use ruff_text_size::{Ranged, TextRange};

use crate::{
    primitives::{scope::scoped_body, walk::filter_map_over_stmts},
    source::Source,
};

mod body;
mod entries;
mod grammar;
mod scan;

pub(crate) use body::{DocstringBody, docstring_body, indent_prefix, triple_quoted_body};
pub(crate) use entries::{entry_carrying_sections, entry_runs};
pub(crate) use grammar::{section_heading, sibling_entry_head, typed_entry_head};
pub(crate) use scan::{LineScan, LineScanner, ScannedLine, opens_structure};

/// The walker driving one function across every docstring in a body,
/// each paired with the class or function definition whose body opens
/// on it.
struct Walker<'a, F> {
    f: &'a mut F,
}

impl<'src, F: FnMut(Option<&'src Stmt>, &'src StringLiteral)> StatementVisitor<'src>
    for Walker<'_, F>
{
    fn visit_stmt(&mut self, stmt: &'src Stmt) {
        if let Some((body, _)) = scoped_body(stmt)
            && let Some(lit) = body_docstring(body)
        {
            (self.f)(Some(stmt), lit);
        }
        walk_stmt(self, stmt);
    }
}

/// Returns `body`'s PEP 257 docstring literal, its first statement
/// when that is a single-part string expression.
pub(crate) fn body_docstring(body: &[Stmt]) -> Option<&StringLiteral> {
    leading_string(body).and_then(ExprStringLiteral::as_single_part_string)
}

/// The range of the leading string expression in `body` and in every
/// class and function body nested inside it, the slot a docstring
/// occupies whatever its part count. An implicitly concatenated
/// docstring lands here where [`body_docstring`] skips it.
pub(crate) fn docstring_slots(body: &[Stmt]) -> Vec<TextRange> {
    leading_string(body)
        .map(Ranged::range)
        .into_iter()
        .chain(filter_map_over_stmts(body, |stmt| {
            Some(leading_string(scoped_body(stmt)?.0)?.range())
        }))
        .collect()
}

/// Every class and function definition in `source` whose body opens on
/// a docstring, paired with that docstring literal in source order. The
/// module docstring carries no definition and is absent.
pub(crate) fn documented_definitions(source: &Source) -> Vec<(&Stmt, &StringLiteral)> {
    let mut found = Vec::new();
    walk_docstrings(source, |owner, lit| {
        found.extend(owner.map(|definition| (definition, lit)));
    });
    found
}

/// Walks every docstring in `source` and gathers the edits `f` produces
/// against each into one fix group per docstring. The closure receives
/// `source`, the docstring literal, and that docstring's edit buffer. A
/// docstring whose buffer stays empty contributes no group.
pub(crate) fn rewrite_docstrings<F>(source: &Source, mut f: F) -> Vec<Vec<Edit>>
where
    F: FnMut(&Source, &StringLiteral, &mut Vec<Edit>),
{
    let mut groups = Vec::new();
    walk_docstrings(source, |_, lit| {
        let mut edits = Vec::new();
        f(source, lit, &mut edits);
        if !edits.is_empty() {
            groups.push(edits);
        }
    });
    groups
}

/// Drives `f` across every docstring in `source` in source order,
/// paired with the class or function definition whose body opens on it
/// and `None` for the module docstring.
pub(crate) fn walk_docstrings<'src>(
    source: &'src Source,
    mut f: impl FnMut(Option<&'src Stmt>, &'src StringLiteral),
) {
    let body = &source.ast().body;
    if let Some(lit) = body_docstring(body) {
        f(None, lit);
    }
    Walker { f: &mut f }.visit_body(body);
}

/// `body`'s leading string expression, the slot a docstring occupies
/// whether or not that expression is a single part.
fn leading_string(body: &[Stmt]) -> Option<&ExprStringLiteral> {
    body.first()
        .and_then(Stmt::as_expr_stmt)
        .and_then(|e| e.value.as_string_literal_expr())
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use ruff_text_size::Ranged;

    use super::*;
    use crate::testing::parse;

    /// Every docstring `source` carries in source order: its value, its
    /// indent prefix, and its triple-quoted body text where one exists.
    #[derive(Default)]
    struct Probe {
        bodies: Vec<String>,
        indents: Vec<String>,
        values: Vec<String>,
    }

    impl Probe {
        fn run(source: &Source) -> Vec<String> {
            probe_with_source(source).values
        }
    }

    fn probe_with_source(source: &Source) -> Probe {
        let mut probe = Probe::default();
        walk_docstrings(source, |_, lit| {
            probe.values.push(lit.value.to_string());
            probe.indents.push(indent_prefix(source, lit).to_owned());
            probe
                .bodies
                .extend(triple_quoted_body(source, lit).map(|b| b.text.to_owned()));
        });
        probe
    }

    #[test]
    fn collects_class_function_and_method_docstrings_in_source_order() {
        let s = parse(
            "\"\"\"M\"\"\"\nclass C:\n    \"\"\"C\"\"\"\n    def m(self):\n        \"\"\"m\"\"\"\n        pass\n",
        );
        assert_eq!(Probe::run(&s), ["M", "C", "m"]);
    }

    #[test]
    fn collects_nested_function_docstrings() {
        let s = parse(
            "def outer():\n    \"\"\"o\"\"\"\n    def inner():\n        \"\"\"i\"\"\"\n        pass\n",
        );
        assert_eq!(Probe::run(&s), ["o", "i"]);
    }

    #[rstest]
    #[case("x = 1\n", 0)]
    #[case("\"\"\"M\"\"\"\n", 1)]
    #[case("\"a\" \"b\"\n", 1)]
    #[case("b\"a\" b\"b\"\n", 0)]
    #[case("def f():\n    helper()\n    return 1\n", 0)]
    #[case(
        "\"\"\"M\"\"\"\nclass C:\n    \"\"\"C\"\"\"\n    def m(self):\n        \"\"\"m\"\"\"\n        pass\n",
        3
    )]
    fn docstring_slots_collects_every_leading_string_expression(
        #[case] src: &str,
        #[case] expected: usize,
    ) {
        assert_eq!(docstring_slots(&parse(src).ast().body).len(), expected);
    }

    #[test]
    fn documented_definitions_pairs_each_definition_with_its_own_docstring() {
        let s = parse(
            "\"\"\"M\"\"\"\nclass C:\n    \"\"\"C\"\"\"\n    def m(self):\n        \"\"\"m\"\"\"\n        pass\n\ndef bare():\n    pass\n",
        );
        let paired: Vec<(bool, String)> = documented_definitions(&s)
            .into_iter()
            .map(|(definition, lit)| (definition.is_class_def_stmt(), lit.value.to_string()))
            .collect();
        assert_eq!(paired, [(true, "C".to_owned()), (false, "m".to_owned())]);
    }

    #[test]
    fn indent_prefix_preserves_source_indent_characters() {
        let s = parse("class C:\n\t\"\"\"doc\"\"\"\n\tpass\n");
        let probe = probe_with_source(&s);
        assert_eq!(probe.indents, ["\t"]);
    }

    #[test]
    fn returns_empty_for_module_with_no_docstrings() {
        let s = parse("x = 1\ndef f():\n    return 1\n");
        assert!(Probe::run(&s).is_empty());
    }

    #[test]
    fn rewrite_docstrings_drops_a_docstring_that_produces_no_edit() {
        let s = parse("\"\"\"M\"\"\"\ndef f():\n    \"\"\"f\"\"\"\n    pass\n");
        let groups = rewrite_docstrings(&s, |_, lit, edits| {
            if lit.value.to_string() == "f" {
                edits.push(Edit::range_deletion(lit.range()));
            }
        });
        assert_eq!(groups.len(), 1);
    }

    #[test]
    fn rewrite_docstrings_groups_edits_per_docstring_in_source_order() {
        let s = parse("\"\"\"M\"\"\"\ndef f():\n    \"\"\"f\"\"\"\n    pass\n");
        let groups = rewrite_docstrings(&s, |_, lit, edits| {
            edits.push(Edit::range_deletion(lit.range()));
        });
        assert_eq!(groups.len(), 2);
        assert!(groups.iter().all(|group| group.len() == 1));
        assert!(groups.windows(2).all(|w| w[0][0].start() < w[1][0].start()));
    }

    #[test]
    fn skips_implicitly_concatenated_docstring_expressions() {
        let s = parse("\"\"\"a\"\"\" \"\"\"b\"\"\"\n");
        assert!(Probe::run(&s).is_empty());
    }

    #[test]
    fn skips_string_expression_that_is_not_first_statement() {
        let s = parse("x = 1\n\"not a docstring\"\n");
        assert!(Probe::run(&s).is_empty());
    }

    #[test]
    fn triple_quoted_body_extracts_inner_body_text() {
        let s = parse("'''hello'''\n");
        let probe = probe_with_source(&s);
        assert_eq!(probe.bodies, ["hello"]);
    }

    #[test]
    fn triple_quoted_body_rejects_inline_with_def() {
        let s = parse("def f(): \"\"\"doc\"\"\"\n");
        let probe = probe_with_source(&s);
        assert!(probe.bodies.is_empty());
    }

    #[test]
    fn triple_quoted_body_rejects_non_triple_quoted_literal() {
        let s = parse("\"hello\"\n");
        let probe = probe_with_source(&s);
        assert!(probe.bodies.is_empty());
    }
}
