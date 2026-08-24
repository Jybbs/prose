//! Shared primitives used across rule implementations.

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

/// Inserts `item` into `vec` at the slot keeping it ascending by
/// `key`, before any element whose key compares equal.
pub(crate) fn insert_sorted_by_key<T, K: Ord>(vec: &mut Vec<T>, item: T, key: impl Fn(&T) -> K) {
    let slot = vec.partition_point(|existing| key(existing) < key(&item));
    vec.insert(slot, item);
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

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

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
