//! Reparse-and-compare oracles a rule runs before committing a rewrite
//! it cannot otherwise validate. `splice_parses` reports whether the
//! candidate parses at all and `splice_preserves_tree` whether it
//! reparses to the same statement tree, with `reparse_window` narrowing
//! the slice either one reads to the innermost statement covering the
//! rewrite.

use ruff_python_ast::{Stmt, comparable::ComparableStmt};
use ruff_python_parser::parse_module;
use ruff_text_size::{Ranged, TextRange};

use crate::{
    primitives::{decorator::is_decorated, scope::sub_bodies, slots::item_holding},
    source::Source,
};

/// The narrowest window a splice over `range` reparses within, the
/// innermost statement covering it or the whole module where none
/// does.
pub(super) fn reparse_window(source: &Source, range: TextRange) -> TextRange {
    window_of(source, covering_statement(&source.ast().body, range))
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
/// innermost statement covering `range`, widening to the whole module
/// where no single statement does.
pub(crate) fn splice_preserves_tree(source: &Source, range: TextRange, replacement: &str) -> bool {
    let body = &source.ast().body;
    let covering = covering_statement(body, range);
    let window = window_of(source, covering);
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

/// The innermost statement whose own range covers `range` and whose
/// slice reparses on its own, descending through the sub-bodies each
/// covering statement opens. `None` where no module-body statement
/// covers it.
fn covering_statement(body: &[Stmt], range: TextRange) -> Option<&Stmt> {
    let mut covering = statement_covering(body, range)?;
    let mut window = covering;
    while let Some(inner) = sub_bodies(covering)
        .into_iter()
        .find_map(|(nested, _)| statement_covering(nested, range))
    {
        covering = inner;
        if slices_cleanly(inner) {
            window = inner;
        }
    }
    Some(window)
}

/// True where a slice taken from `stmt`'s own start reparses on its
/// own. That slice drops the indent the first line carries, which every
/// deeper line survives, whereas a line back at the column the first
/// one just left no longer lines up. A second clause keyword sits at
/// that column, as `elif`, `else`, `except` and `finally` each do, and
/// so does the `def` or `class` line under a decorator.
fn slices_cleanly(stmt: &Stmt) -> bool {
    !is_decorated(stmt)
        && sub_bodies(stmt)
            .iter()
            .filter(|(body, _)| !body.is_empty())
            .count()
            <= 1
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

/// The statement of `body` whose own range covers `range`, `None`
/// where no single statement does.
fn statement_covering(body: &[Stmt], range: TextRange) -> Option<&Stmt> {
    item_holding(body, range.start()).filter(|stmt| range.end() <= stmt.end())
}

/// The window `covering` reparses within, its own range or the whole
/// module where no statement covers the splice.
fn window_of(source: &Source, covering: Option<&Stmt>) -> TextRange {
    covering.map_or_else(|| source.module_range(), Ranged::range)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{parse, range};

    /// A module whose second statement wraps a grouping parenthesis
    /// pair inside its body, so the descent runs one level past the
    /// module body.
    const NESTED_PAREN: &str = "x = 1\ndef f():\n    return (1)\n";

    /// A module whose grouping parenthesis pair sits inside its first
    /// statement, the boundary `statement_covering`'s partition point
    /// resolves at index zero.
    const LEADING_PAREN: &str = "def f():\n    return (1)\nx = 1\n";

    /// A module whose grouping parenthesis pair sits two suites deep,
    /// so the descent runs past the `def` and then past the `if`.
    const TWICE_NESTED_PAREN: &str = "def f():\n    if x:\n        return (1)\n";

    /// A class whose decorated method carries a grouping parenthesis
    /// pair in its signature, putting the pair inside a statement whose
    /// `def` line sits at the column its decorator holds.
    const DECORATED_DEF_PAREN: &str = "class C:\n    @d\n    def m(x=(1)):\n        pass\n";

    /// A module whose `if` carries an `else` clause and a grouping
    /// parenthesis pair in its test, putting the pair inside a
    /// statement whose `else` sits at the column the `if` holds.
    const ELSE_CLAUSE_PAREN: &str =
        "def f():\n    if (x):\n        pass\n    else:\n        pass\n";

    /// The `(1)` span inside `text`, its opener the last in the module
    /// since the `def` header carries the first.
    fn paren_pair(text: &str) -> TextRange {
        let open = u32::try_from(text.rfind('(').expect("a paren")).expect("fits");
        range(open, open + 3)
    }

    /// The `return` statement holding the `(1)` pair, reached by
    /// following the first sub-body of each statement in turn.
    fn innermost_return(source: &Source, index: usize) -> TextRange {
        let mut stmt = &source.ast().body[index];
        while let Some(&(nested, _)) = sub_bodies(stmt).first() {
            stmt = &nested[0];
        }
        stmt.range()
    }

    #[test]
    fn covering_statement_answers_none_for_a_range_spanning_two_statements() {
        let source = parse("x = 1\ny = 2\n");

        assert!(covering_statement(&source.ast().body, range(0, 11)).is_none());
    }

    #[rstest]
    #[case(NESTED_PAREN, 1)]
    #[case(LEADING_PAREN, 0)]
    #[case(TWICE_NESTED_PAREN, 0)]
    fn covering_statement_descends_to_the_innermost_statement_holding_a_nested_range(
        #[case] text: &str,
        #[case] index: usize,
    ) {
        let source = parse(text);

        let stmt =
            covering_statement(&source.ast().body, paren_pair(text)).expect("the return covers it");

        assert_eq!(stmt.range(), innermost_return(&source, index));
    }

    #[rstest]
    #[case::decorated_definition(DECORATED_DEF_PAREN)]
    #[case::second_clause(ELSE_CLAUSE_PAREN)]
    fn covering_statement_declines_a_statement_whose_slice_would_not_reparse(#[case] text: &str) {
        let source = parse(text);

        let stmt = covering_statement(&source.ast().body, paren_pair(text))
            .expect("the module statement covers it");

        assert_eq!(stmt.range(), source.ast().body[0].range());
    }

    #[test]
    fn reparse_window_narrows_to_the_innermost_statement_covering_the_range() {
        let source = parse(NESTED_PAREN);

        assert_eq!(
            reparse_window(&source, paren_pair(NESTED_PAREN)),
            innermost_return(&source, 1),
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
