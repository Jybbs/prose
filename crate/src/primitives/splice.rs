//! Reparse-and-compare oracles a rule runs before committing a rewrite
//! it cannot otherwise validate. `splice_parses` reports whether the
//! candidate parses at all and `splice_preserves_tree` whether it
//! reparses to the same statement tree, with `reparse_window` narrowing
//! the slice either one reads to the module-body statement covering the
//! rewrite.

use ruff_python_ast::{Stmt, comparable::ComparableStmt};
use ruff_python_parser::parse_module;
use ruff_text_size::{Ranged, TextRange};

use crate::source::Source;

/// The narrowest window a splice over `range` reparses within, the
/// module-body statement covering it or the whole module where none
/// does.
pub(super) fn reparse_window(source: &Source, range: TextRange) -> TextRange {
    covering_statement(&source.ast().body, range)
        .map_or_else(|| source.module_range(), Ranged::range)
}

/// Reports whether splicing `replacement` into `outer` at `inner`
/// yields source that `parse` accepts, the round-trip a rule runs
/// before committing a rewrite it cannot otherwise validate.
pub(crate) fn splice_parses<T, E>(
    source: &Source,
    outer: TextRange,
    inner: TextRange,
    replacement: &str,
    parse: impl Fn(&str) -> Result<T, E>,
) -> bool {
    splice_reparse(source, outer, inner, replacement, parse).is_ok()
}

/// Reports whether splicing `replacement` over `range` reparses to the
/// same statement tree, the round-trip a rule runs before committing a
/// rewrite it means to leave semantics-free. The window is the
/// module-body statement covering `range`, widening to the whole module
/// where no single statement does.
pub(crate) fn splice_preserves_tree(source: &Source, range: TextRange, replacement: &str) -> bool {
    let body = &source.ast().body;
    let covering = covering_statement(body, range);
    let window = covering.map_or_else(|| source.module_range(), Ranged::range);
    let before = covering.map_or(body.as_slice(), std::slice::from_ref);
    let Ok(reparsed) = splice_reparse(source, window, range, replacement, parse_module) else {
        return false;
    };
    before.iter().map(ComparableStmt::from).eq(reparsed
        .syntax()
        .body
        .iter()
        .map(ComparableStmt::from))
}

/// The module-body statement whose own range covers `range`, `None`
/// where no single statement does.
fn covering_statement(body: &[Stmt], range: TextRange) -> Option<&Stmt> {
    let after = body.partition_point(|stmt| stmt.start() <= range.start());
    body[..after]
        .last()
        .filter(|stmt| range.end() <= stmt.end())
}

/// Splices `replacement` into `outer` at `inner` and returns the parsed
/// result, the shared body under [`splice_parses`] and
/// [`splice_preserves_tree`].
fn splice_reparse<T, E>(
    source: &Source,
    outer: TextRange,
    inner: TextRange,
    replacement: &str,
    parse: impl Fn(&str) -> Result<T, E>,
) -> Result<T, E> {
    let candidate = format!(
        "{}{replacement}{}",
        source.slice(TextRange::new(outer.start(), inner.start())),
        source.slice(TextRange::new(inner.end(), outer.end())),
    );
    parse(&candidate)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{parse, range};

    /// A module whose second statement wraps a grouping parenthesis
    /// pair inside its body, so the pair's covering statement is the
    /// `def` rather than the module.
    const NESTED_PAREN: &str = "x = 1\ndef f():\n    return (1)\n";

    /// A module whose grouping parenthesis pair sits inside its first
    /// statement, the boundary `covering_statement`'s partition point
    /// resolves at index zero.
    const LEADING_PAREN: &str = "def f():\n    return (1)\nx = 1\n";

    /// The `(1)` span inside `text`, its opener the last in the module
    /// since the `def` header carries the first.
    fn paren_pair(text: &str) -> TextRange {
        let open = u32::try_from(text.rfind('(').expect("a paren")).expect("fits");
        range(open, open + 3)
    }

    #[test]
    fn covering_statement_answers_none_for_a_range_spanning_two_statements() {
        let source = parse("x = 1\ny = 2\n");

        assert!(covering_statement(&source.ast().body, range(0, 11)).is_none());
    }

    #[rstest]
    #[case(NESTED_PAREN, 1)]
    #[case(LEADING_PAREN, 0)]
    fn covering_statement_finds_the_statement_holding_a_nested_range(
        #[case] text: &str,
        #[case] index: usize,
    ) {
        let source = parse(text);

        let stmt =
            covering_statement(&source.ast().body, paren_pair(text)).expect("the def covers it");

        assert_eq!(stmt.range(), source.ast().body[index].range());
    }

    #[test]
    fn reparse_window_narrows_to_the_statement_covering_the_range() {
        let source = parse(NESTED_PAREN);

        assert_eq!(
            reparse_window(&source, paren_pair(NESTED_PAREN)),
            source.ast().body[1].range(),
        );
    }

    #[test]
    fn reparse_window_widens_to_the_module_for_an_uncovered_range() {
        let source = parse("x = 1\ny = 2\n");

        assert_eq!(
            reparse_window(&source, source.module_range()),
            source.module_range(),
        );
    }

    #[rstest]
    #[case::same_tree("1", true)]
    #[case::changed_tree("2", false)]
    fn splice_preserves_tree_reads_the_statement_covering_the_range(
        #[case] replacement: &str,
        #[case] preserved: bool,
    ) {
        let source = parse(NESTED_PAREN);

        assert_eq!(
            splice_preserves_tree(&source, paren_pair(NESTED_PAREN), replacement),
            preserved,
        );
    }

    #[rstest]
    #[case::same_tree("x = 1\ny = 2\n", true)]
    #[case::changed_tree("x = 1\ny = 3\n", false)]
    fn splice_preserves_tree_widens_to_the_module_for_an_uncovered_range(
        #[case] replacement: &str,
        #[case] preserved: bool,
    ) {
        let source = parse("x = 1\ny = 2\n");

        assert_eq!(
            splice_preserves_tree(&source, source.module_range(), replacement),
            preserved,
        );
    }
}
