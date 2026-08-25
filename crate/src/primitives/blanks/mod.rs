//! Module-scope blank-line policy, the gap text it renders to, and the
//! walk back over a blank run. [`module_blank_lines`] declares the
//! canonical blank count for a module-scope `(prev, curr)` pair,
//! [`blank_gap`] turns a line ending and a count into the separator an
//! assembled body seats between two blocks, and
//! [`whitespace_start_before`] reaches back over the run preceding an
//! offset, stopping at the wall opening a notebook cell.

use ruff_python_ast::{CmpOp, Expr, Stmt};
use ruff_source_file::LineEnding;
use ruff_text_size::TextSize;

use crate::{primitives::imports::import_blank_lines, source::Source};

/// The gap seating `blanks` blank lines between two assembled blocks,
/// one newline closing the first block plus one per blank line, each in
/// `ending`'s sequence.
pub(crate) fn blank_gap(ending: LineEnding, blanks: u32) -> &'static str {
    let run = newline_run(ending);
    run.get(..(blanks as usize + 1) * ending.len())
        .unwrap_or_else(|| {
            unreachable!(
                "invariant: module blank policy returns at most {} blanks",
                run.len() / ending.len() - 1,
            )
        })
}

/// The canonical blank-line count for the module-scope pair `(prev,
/// curr)`. `None` means no case applies and the pair keeps the gap the
/// source holds. A statement following an `if __name__ == "__main__":`
/// block carries 1. A grouped import pair carries 1 across distinct
/// canonical groups and reports no opinion within a group, while an
/// ungrouped pair reads as one flat block and never divides. A
/// top-level `FunctionDef` or `ClassDef` carries 2 on each side,
/// whatever statement kind neighbors it, and any other statement
/// following an import carries 1.
pub(crate) fn module_blank_lines(
    prev: &Stmt,
    curr: &Stmt,
    first_party: &[String],
    grouped: bool,
) -> Option<u32> {
    if is_main_guard(prev) {
        return Some(1);
    }
    if let Some(blanks) = import_blank_lines(prev, curr, first_party, grouped) {
        return (blanks != 0).then_some(blanks);
    }
    match (prev, curr) {
        (_, Stmt::FunctionDef(_) | Stmt::ClassDef(_))
        | (Stmt::FunctionDef(_) | Stmt::ClassDef(_), _) => Some(2),
        (Stmt::Import(_) | Stmt::ImportFrom(_), _) => Some(1),
        _ => None,
    }
}

/// Returns the start of the contiguous ASCII-whitespace run immediately
/// preceding `offset`, held at the start of the notebook cell
/// containing `offset` so the run never reaches into the cell above.
pub(crate) fn whitespace_start_before(source: &Source, offset: TextSize) -> TextSize {
    let text = source.text();
    let trimmed = text[..offset.to_usize()].trim_end_matches(|c: char| c.is_ascii_whitespace());
    TextSize::of(trimmed).max(source.cell_start(offset).unwrap_or_default())
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

/// The run [`blank_gap`] slices, `ending` repeated one time more than
/// the widest count [`module_blank_lines`] returns.
fn newline_run(ending: LineEnding) -> &'static str {
    match ending {
        LineEnding::Cr => "\r\r\r",
        LineEnding::CrLf => "\r\n\r\n\r\n",
        LineEnding::Lf => "\n\n\n",
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use ruff_text_size::Ranged;

    use super::*;
    use crate::testing::{notebook, parse};

    const MAIN_GUARD: &str = "if __name__ == \"__main__\":\n    main()\n";

    /// The canonical count for the first two statements of `src`.
    fn blanks_of(src: &str, first_party: &[&str]) -> Option<u32> {
        let list: Vec<String> = first_party.iter().map(|&s| s.to_owned()).collect();
        let source = parse(src);
        let body = &source.ast().body;
        module_blank_lines(&body[0], &body[1], &list, true)
    }

    #[rstest]
    #[should_panic(expected = "invariant: module blank policy returns at most 2 blanks")]
    fn blank_gap_refuses_a_count_beyond_the_policy(
        #[values(LineEnding::Cr, LineEnding::CrLf, LineEnding::Lf)] ending: LineEnding,
    ) {
        blank_gap(ending, 3);
    }

    #[rstest]
    #[case(LineEnding::Lf, 0, "\n")]
    #[case(LineEnding::Lf, 1, "\n\n")]
    #[case(LineEnding::Lf, 2, "\n\n\n")]
    #[case(LineEnding::CrLf, 0, "\r\n")]
    #[case(LineEnding::CrLf, 1, "\r\n\r\n")]
    #[case(LineEnding::CrLf, 2, "\r\n\r\n\r\n")]
    #[case(LineEnding::Cr, 0, "\r")]
    #[case(LineEnding::Cr, 1, "\r\r")]
    #[case(LineEnding::Cr, 2, "\r\r\r")]
    fn blank_gap_seats_one_newline_per_blank_plus_one(
        #[case] ending: LineEnding,
        #[case] blanks: u32,
        #[case] gap: &str,
    ) {
        assert_eq!(blank_gap(ending, blanks), gap);
    }

    #[test]
    fn is_main_guard_accepts_canonical_form() {
        let source = parse(MAIN_GUARD);
        assert!(is_main_guard(&source.ast().body[0]));
    }

    #[rstest]
    #[case("x = 1\n")]
    #[case("if x:\n    pass\n")]
    #[case("if __name__ != \"__main__\":\n    pass\n")]
    #[case("if __name__ == \"main\":\n    pass\n")]
    #[case("if other == \"__main__\":\n    pass\n")]
    #[case("if __name__ == __main__:\n    pass\n")]
    #[case("if __name__ == \"__main__\" and x:\n    pass\n")]
    fn is_main_guard_rejects_every_other_shape(#[case] src: &str) {
        let source = parse(src);
        assert!(!is_main_guard(&source.ast().body[0]));
    }

    #[rstest]
    #[case("from os import path\nfrom . import x\n", &[], Some(1))]
    #[case("from os import path\nfrom myapp import x\n", &["myapp"], Some(1))]
    #[case("import os\nimport myapp\n", &["myapp"], Some(1))]
    #[case("import myapp\nfrom myapp import x\n", &["myapp"], None)]
    #[case("from myapp import a\nfrom myapp.db import b\n", &["myapp"], None)]
    fn module_blank_lines_divides_distinct_import_groups(
        #[case] src: &str,
        #[case] first_party: &[&str],
        #[case] expected: Option<u32>,
    ) {
        assert_eq!(blanks_of(src, first_party), expected);
    }

    #[rstest]
    #[case("import os\nimport sys\n")]
    #[case("from os import path\nfrom sys import argv\n")]
    #[case("x = 1\ny = 2\n")]
    fn module_blank_lines_holds_no_opinion_on_a_flat_run(#[case] src: &str) {
        assert_eq!(blanks_of(src, &[]), None);
    }

    #[rstest]
    #[case("x = 1\nclass C: pass\n")]
    #[case("x = 1\ndef f(): pass\n")]
    #[case("import os\ndef f(): pass\n")]
    #[case("from sys import path\nclass C: pass\n")]
    #[case("class C: pass\nPORT = 8080\n")]
    #[case("class C: pass\nPORT: int = 8080\n")]
    #[case("class C: pass\nlaunch()\n")]
    #[case("class C: pass\nfor x in y:\n    pass\n")]
    #[case("def f(): pass\nPORT = 8080\n")]
    #[case("def f(): pass\nprint(1)\n")]
    #[case("def f(): pass\nif ready:\n    go()\n")]
    #[case("def f(): pass\nimport os\n")]
    #[case("def f(): pass\nfrom os import path\n")]
    #[case("async def f(): pass\nprint(1)\n")]
    fn module_blank_lines_pairs_a_definition_to_two(#[case] src: &str) {
        assert_eq!(blanks_of(src, &[]), Some(2));
    }

    #[test]
    fn module_blank_lines_pairs_a_statement_after_a_main_guard_to_one() {
        assert_eq!(blanks_of(&format!("{MAIN_GUARD}xs = 1\n"), &[]), Some(1));
    }

    #[rstest]
    #[case("import os\nfrom sys import argv\n")]
    #[case("from sys import argv\nimport os\n")]
    #[case("import os\nPORT = 8080\n")]
    #[case("from sys import path\nPORT: int = 8080\n")]
    #[case("import os\nlaunch()\n")]
    #[case("from sys import path\nif ready:\n    go()\n")]
    #[case("from __future__ import annotations\nPORT = 8080\n")]
    fn module_blank_lines_pairs_an_import_boundary_to_one(#[case] src: &str) {
        assert_eq!(blanks_of(src, &[]), Some(1));
    }

    #[test]
    fn whitespace_start_before_holds_at_a_notebook_cell_wall() {
        let source = notebook(&["import os", "\n\nvalue = 1\n"]);
        let start = source.ast().body[1].start();

        assert_eq!(
            whitespace_start_before(&source, start),
            source.cell_start(start).expect("the cell holding it"),
            "the run stops at the wall rather than reaching the cell above",
        );
    }

    #[rstest]
    #[case::crlf("a\r\n\r\nb", 5, 1)]
    #[case::leading_whitespace("   \n\n\nx", 6, 0)]
    #[case::stops_at_non_whitespace("ab\n\ncd", 4, 2)]
    fn whitespace_start_before_walks_back_over_the_run(
        #[case] text: &str,
        #[case] offset: u32,
        #[case] expected: u32,
    ) {
        assert_eq!(
            whitespace_start_before(&parse(text), TextSize::new(offset)),
            TextSize::new(expected),
        );
    }
}
