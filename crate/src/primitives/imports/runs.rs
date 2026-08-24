//! The runs adjacent imports form, their section boundaries, and the
//! blank lines between them.

use std::ops::Range;

use ruff_python_ast::{Stmt, StmtImportFrom};
use ruff_source_file::LineRanges;
use ruff_text_size::TextRange;

use crate::{
    primitives::{
        blanks::whitespace_start_before,
        sections::Sections,
        slots::{runs_where, slot_runs},
    },
    source::Source,
};

use super::*;

/// True when the module carries `from __future__ import annotations`,
/// deferring every annotation's evaluation per PEP 563.
pub(crate) fn defers_annotations(body: &[Stmt]) -> bool {
    body.iter()
        .filter_map(Stmt::as_import_from_stmt)
        .any(|node| future_annotations_alias(node).is_some())
}

/// Returns the position of the `annotations` alias in a
/// `from __future__ import …` statement, or `None` for any other
/// import.
pub(crate) fn future_annotations_alias(node: &StmtImportFrom) -> Option<usize> {
    if !is_future(node) {
        return None;
    }
    node.names
        .iter()
        .position(|alias| alias.name.id == FUTURE_ANNOTATIONS)
}

/// Canonical blank-line count between two adjacent import statements,
/// the one decider the import collapse, the banded import arm, and
/// `space-statements` share. `Some(1)` divides distinct groups while
/// `grouped`, `Some(0)` seats every other import pair tight, and `None`
/// pins any pair that is not two imports. Ungrouped, the imports read as
/// one flat block, so no pair carries a divider.
pub(crate) fn import_blank_lines(
    a: &Stmt,
    b: &Stmt,
    first_party: &[String],
    grouped: bool,
) -> Option<u32> {
    let a_group = import_group(a, first_party)?;
    let b_group = import_group(b, first_party)?;
    Some(u32::from(grouped && a_group != b_group))
}

/// The runs of adjacent import statements in `body`, a lone import a
/// run of its own.
pub(crate) fn import_runs(body: &[Stmt]) -> Vec<Vec<usize>> {
    slot_runs(body, |a, b| is_import(a) && is_import(b))
        .filter(|run| is_import(&body[run.start]))
        .map(Iterator::collect)
        .collect()
}

/// Slot ranges of every import run across a sectioned body, each run
/// offset to absolute slot indices so it never spans a section divider.
/// The unit `group-imports` partitions and `alphabetize-siblings`
/// sorts, one run at a time within each section.
pub(crate) fn sectioned_import_runs(sections: &Sections, body: &[Stmt]) -> Vec<Range<usize>> {
    sections
        .ranges()
        .iter()
        .flat_map(|section| {
            runs_where(&body[section.clone()], is_import)
                .into_iter()
                .map(move |run| section.start + run.start..section.start + run.end)
        })
        .collect()
}

/// The full lines `stmt` sits on together with the blank run directly
/// above them, held within the statement's notebook cell.
pub(super) fn lines_under_blank_run(source: &Source, stmt: TextRange) -> TextRange {
    let lines = source.full_lines_within_cell(stmt);
    let above = whitespace_start_before(source, lines.start());
    TextRange::new(
        source.text().full_line_end(above).min(lines.start()),
        lines.end(),
    )
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::{primitives::orderer::member_blocks, testing::parse};

    #[rstest]
    #[case("from __future__ import annotations\n", true)]
    #[case("from __future__ import annotations, division\n", true)]
    #[case("from __future__ import division\n", false)]
    #[case("from other import annotations\n", false)]
    #[case("import __future__\n", false)]
    #[case("x = 1\n", false)]
    fn defers_annotations_detects_the_future_import(#[case] src: &str, #[case] expected: bool) {
        let source = parse(src);
        assert_eq!(defers_annotations(&source.ast().body), expected);
    }

    #[rstest]
    #[case("import os\nimport sys\n", true, Some(0))]
    #[case("import os\nfrom collections import deque\n", true, Some(1))]
    #[case("import os\nfrom collections import deque\n", false, Some(0))]
    #[case("import os\nimport sys\n", false, Some(0))]
    #[case("from __future__ import annotations\nimport os\n", true, Some(1))]
    #[case("from __future__ import annotations\nimport os\n", false, Some(0))]
    #[case("import os\nx = 1\n", true, None)]
    #[case("x = 1\nimport os\n", true, None)]
    fn import_blank_lines_scores_only_import_pairs(
        #[case] src: &str,
        #[case] grouped: bool,
        #[case] expected: Option<u32>,
    ) {
        let source = parse(src);
        let body = &source.ast().body;
        assert_eq!(
            import_blank_lines(&body[0], &body[1], &[], grouped),
            expected
        );
    }

    #[test]
    fn sectioned_import_runs_offsets_each_section_run_past_the_divider() {
        let source = parse("import os\nimport sys\n# --- Typing ---\nimport abc\nimport io\n");
        let body = &source.ast().body;
        let blocks = member_blocks(&source, body, source.module_range());
        let sections = Sections::of(&source, &blocks);
        assert_eq!(sectioned_import_runs(&sections, body), vec![0..2, 2..4]);
    }
}
