//! Shared primitives used across rule implementations.

use ruff_python_ast::Stmt;
use ruff_text_size::{Ranged, TextSize};
use rustc_hash::FxHashMap;

pub(crate) mod alias;
pub(crate) mod aligner;
pub(crate) mod binding;
pub(crate) mod blanks;
pub(crate) mod call_keywords;
pub(crate) mod colon_targets;
pub(crate) mod comments;
pub(crate) mod comparison;
pub(crate) mod constructor;
pub(crate) mod decorator;
pub(crate) mod docstring;
pub(crate) mod edit;
pub(crate) mod effect;
pub(crate) mod equal_targets;
pub(crate) mod fracture;
pub(crate) mod imports;
pub(crate) mod inline;
pub(crate) mod layout;
pub(crate) mod one_row;
pub(crate) mod orderer;
pub(crate) mod padding;
pub(crate) mod params;
pub(crate) mod quoting;
pub(crate) mod range;
pub(crate) mod reseat;
pub(crate) mod reserve;
pub(crate) mod scope;
pub(crate) mod sections;
pub(crate) mod slots;
pub(crate) mod splice;
pub(crate) mod tiering;
pub(crate) mod tokens;
pub(crate) mod travel;
pub(crate) mod walk;

/// PEP 8 indent step in spaces, the depth one nested level adds.
pub(crate) const INDENT_STEP: usize = 4;

/// `pairs` grouped by key, each key holding its values in arrival
/// order.
pub(crate) fn group_map<K: Eq + std::hash::Hash, V>(
    pairs: impl IntoIterator<Item = (K, V)>,
) -> FxHashMap<K, Vec<V>> {
    pairs
        .into_iter()
        .fold(FxHashMap::default(), |mut map, (key, value)| {
            map.entry(key).or_default().push(value);
            map
        })
}

/// The slot of the `body` statement whose start is at or before
/// `offset`, `None` ahead of the first statement.
pub(crate) fn slot_holding(body: &[Stmt], offset: TextSize) -> Option<usize> {
    body.partition_point(|stmt| stmt.start() <= offset)
        .checked_sub(1)
}

/// Byte offset of the first `:` in `s` that sits at paren-and-bracket
/// depth zero, reading the walrus `:=` as one operator rather than as a
/// colon. `None` when every colon is nested or `s` carries none.
pub(crate) fn unbracketed_colon(s: &str) -> Option<usize> {
    let mut depth = 0usize;
    let bytes = s.as_bytes();
    for (cursor, byte) in bytes.iter().enumerate() {
        match byte {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth = depth.saturating_sub(1),
            b':' if depth == 0 && bytes.get(cursor + 1) != Some(&b'=') => return Some(cursor),
            _ => {}
        }
    }
    None
}

/// The slot at which `item` keeps `items` ascending by `key`, ahead of
/// any element whose key compares equal.
fn sorted_slot<T, K: Ord>(items: &[T], item: &T, key: impl Fn(&T) -> K) -> usize {
    items.partition_point(|existing| key(existing) < key(item))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::parse;

    #[test]
    fn group_map_keeps_each_keys_values_in_arrival_order() {
        let grouped = group_map([(1, "a"), (2, "b"), (1, "c")]);
        assert_eq!(grouped[&1], vec!["a", "c"]);
        assert_eq!(grouped[&2], vec!["b"]);
    }

    #[rstest]
    #[case::ahead_of_the_first_statement(0, None)]
    #[case::statement_start(7, Some(0))]
    #[case::inside_a_statement(10, Some(0))]
    #[case::between_statements(13, Some(0))]
    #[case::past_the_last_statement(20, Some(1))]
    fn slot_holding_reads_the_statement_starting_at_or_before_the_offset(
        #[case] offset: u32,
        #[case] expected: Option<usize>,
    ) {
        let source = parse("# lead\nx = 1\n\ny = 2\n");
        assert_eq!(
            slot_holding(&source.ast().body, TextSize::new(offset)),
            expected
        );
    }

    #[rstest]
    #[case("n := 5", None)]
    #[case("lambda: 1", Some(6))]
    #[case("k := v, other: type", Some(13))]
    fn unbracketed_colon_reads_a_walrus_as_one_operator(
        #[case] text: &str,
        #[case] expected: Option<usize>,
    ) {
        assert_eq!(unbracketed_colon(text), expected, "{text}");
    }

    #[test]
    fn unbracketed_colon_returns_none_when_colon_nested_or_absent() {
        assert!(unbracketed_colon("name (only: parens)").is_none());
        assert!(unbracketed_colon("List[str, int]").is_none());
        assert!(unbracketed_colon("no colon here").is_none());
    }

    #[test]
    fn unbracketed_colon_skips_balanced_parens_and_brackets() {
        assert_eq!(unbracketed_colon("markup (str): desc"), Some(12));
        assert_eq!(
            unbracketed_colon("x (Dict[str, int]): mapping"),
            Some("x (Dict[str, int])".len()),
        );
    }
}
