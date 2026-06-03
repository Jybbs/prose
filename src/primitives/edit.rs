//! Edit-shaping primitives shared across rules. `apply_edits` splices
//! a sorted edit list into a source string, the pipeline runner's
//! transform between rules. `apply_inline_edits` folds a list of
//! edits into a source range, returning `Cow::Borrowed` when no
//! edit applies. Both decline overlapping edits, `apply_edits` with
//! `None` and `apply_inline_edits` with `Cow::Borrowed`. `narrow_edit`
//! trims a candidate replacement to its minimal divergent range
//! against the source.

use std::borrow::Cow;

use ruff_diagnostics::Edit;
use ruff_text_size::{Ranged, TextLen, TextRange, TextSize};

use crate::source::Source;

/// Splices `edits` into `text` and returns the resulting string, or
/// `None` when the sorted edits overlap.
///
/// Sorts edits by start-then-end (via `Edit`'s `Ord` impl) and weaves
/// them in one forward pass, linear in the source length regardless of
/// how many edits apply. Declines with `None` rather than slicing an
/// inverted range, leaving the caller to keep the source unchanged.
pub(crate) fn apply_edits(text: &str, mut edits: Vec<Edit>) -> Option<String> {
    edits.sort_unstable();
    weave(text, TextRange::up_to(text.text_len()), &edits)
}

/// Folds any leaf edits whose range falls inside `range` into the
/// source slice for that range. Returns `Cow::Borrowed` when no leaf
/// edit applies or the in-range edits overlap. `edits` must be sorted
/// by `range().start()`, an invariant that `collect_leaf_edits` upholds
/// via the AST visitor's source-order pre-order walk.
pub(crate) fn apply_inline_edits<'src>(
    source: &'src Source,
    range: TextRange,
    edits: &[Edit],
) -> Cow<'src, str> {
    let lo = edits.partition_point(|e| e.start() < range.start());
    let hi = lo + edits[lo..].partition_point(|e| e.start() <= range.end());
    let mut inside = edits[lo..hi]
        .iter()
        .filter(|e| e.end() <= range.end())
        .peekable();
    if inside.peek().is_none() {
        return Cow::Borrowed(source.slice(range));
    }
    match weave(source.text(), range, inside) {
        Some(out) => Cow::Owned(out),
        None => Cow::Borrowed(source.slice(range)),
    }
}

/// Trims a candidate replacement to its minimal spanning range by
/// stripping the longest common codepoint prefix and suffix shared
/// with `source_slice`. Returns `None` when `text` already equals
/// `source_slice` (no edit needed). Walks codepoint-by-codepoint so
/// the trim never lands inside a multibyte UTF-8 sequence.
pub(crate) fn narrow_edit(
    mut text: String,
    span: TextRange,
    source_slice: &str,
) -> Option<(TextRange, String)> {
    if text == source_slice {
        return None;
    }
    let prefix_len: TextSize = text
        .chars()
        .zip(source_slice.chars())
        .take_while(|(a, b)| a == b)
        .map(|(c, _)| c.text_len())
        .sum();
    let prefix_bytes = prefix_len.to_usize();
    let text_tail = &text[prefix_bytes..];
    let source_tail = &source_slice[prefix_bytes..];
    let suffix_len: TextSize = text_tail
        .chars()
        .rev()
        .zip(source_tail.chars().rev())
        .take_while(|(a, b)| a == b)
        .map(|(c, _)| c.text_len())
        .sum();
    let suffix_bytes = suffix_len.to_usize();
    text.truncate(text.len() - suffix_bytes);
    text.drain(..prefix_bytes);
    Some((span.add_start(prefix_len).sub_end(suffix_len), text))
}

/// Narrows `text` against the source slice covered by `span` and
/// shapes the result as either a deletion or replacement Edit.
/// Returns `None` when the text already matches the source slice.
pub(crate) fn narrowed_replacement(source: &Source, span: TextRange, text: String) -> Option<Edit> {
    let (narrowed_span, narrowed_text) = narrow_edit(text, span, source.slice(span))?;
    Some(if narrowed_text.is_empty() {
        Edit::range_deletion(narrowed_span)
    } else {
        Edit::range_replacement(narrowed_text, narrowed_span)
    })
}

/// Wraps each edit in its own single-edit fix group, the shape a rule
/// whose edits are mutually independent returns from `apply`.
pub(crate) fn singleton_groups(edits: impl IntoIterator<Item = Edit>) -> Vec<Vec<Edit>> {
    edits.into_iter().map(|edit| vec![edit]).collect()
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
    let candidate = format!(
        "{}{replacement}{}",
        source.slice(TextRange::new(outer.start(), inner.start())),
        source.slice(TextRange::new(inner.end(), outer.end())),
    );
    parse(&candidate).is_ok()
}

/// Weaves `edits` into the `span` slice of `text` and returns the
/// woven string, or `None` when two edits overlap. `edits` must be
/// sorted by start and lie within `span`, the overlap being an edit
/// whose start precedes the running cursor.
fn weave<'a>(
    text: &str,
    span: TextRange,
    edits: impl IntoIterator<Item = &'a Edit>,
) -> Option<String> {
    let mut out = String::with_capacity(span.len().to_usize());
    let mut cursor = span.start();
    for edit in edits {
        if edit.start() < cursor {
            return None;
        }
        out.push_str(&text[TextRange::new(cursor, edit.start())]);
        out.push_str(edit.content().unwrap_or_default());
        cursor = edit.end();
    }
    out.push_str(&text[TextRange::new(cursor, span.end())]);
    Some(out)
}

#[cfg(test)]
mod tests {
    use assert_matches::assert_matches;

    use super::*;
    use crate::test_support::{parse, range};

    #[test]
    fn apply_edits_declines_overlapping_edits() {
        let out = apply_edits(
            "abcdef",
            vec![
                Edit::range_replacement("X".to_owned(), range(0, 3)),
                Edit::range_replacement("Y".to_owned(), range(2, 4)),
            ],
        );

        assert_matches!(out, None);
    }

    #[test]
    fn apply_edits_handles_insertions_and_deletions() {
        let out = apply_edits(
            "abcd",
            vec![
                Edit::insertion("<".to_owned(), 0u32.into()),
                Edit::range_deletion(range(2, 3)),
            ],
        );

        assert_eq!(out, Some("<abd".to_owned()));
    }

    #[test]
    fn apply_edits_handles_multiple_non_overlapping_edits() {
        let out = apply_edits(
            "abcdef",
            vec![
                Edit::range_replacement("X".to_owned(), range(0, 1)),
                Edit::range_replacement("Y".to_owned(), range(4, 5)),
            ],
        );

        assert_eq!(out, Some("XbcdYf".to_owned()));
    }

    #[test]
    fn apply_edits_keeps_adjacent_edits() {
        let out = apply_edits(
            "abcdef",
            vec![
                Edit::range_replacement("X".to_owned(), range(0, 2)),
                Edit::range_replacement("Y".to_owned(), range(2, 4)),
            ],
        );

        assert_eq!(out, Some("XYef".to_owned()));
    }

    #[test]
    fn apply_edits_sorts_unsorted_input() {
        let out = apply_edits(
            "abcdef",
            vec![
                Edit::range_replacement("Y".to_owned(), range(4, 5)),
                Edit::range_replacement("X".to_owned(), range(0, 1)),
            ],
        );

        assert_eq!(out, Some("XbcdYf".to_owned()));
    }

    #[test]
    fn apply_inline_edits_declines_overlapping_edits() {
        let source = parse("abcdef\n");
        let result = apply_inline_edits(
            &source,
            range(0, 6),
            &[
                Edit::range_replacement("X".to_owned(), range(0, 3)),
                Edit::range_replacement("Y".to_owned(), range(2, 4)),
            ],
        );

        assert_matches!(result, Cow::Borrowed("abcdef"));
    }

    #[test]
    fn apply_inline_edits_keeps_adjacent_edits() {
        let source = parse("abcdef\n");
        let result = apply_inline_edits(
            &source,
            range(0, 6),
            &[
                Edit::range_replacement("X".to_owned(), range(0, 2)),
                Edit::range_replacement("Y".to_owned(), range(2, 4)),
            ],
        );

        assert_matches!(result, Cow::Owned(text) if text == "XYef");
    }

    #[test]
    fn narrow_edit_handles_multibyte_codepoint_at_divergence() {
        let span = range(0, 7);
        let (r, text) = narrow_edit("α = 1\n".to_owned(), span, "β = 1\n").expect("differs");
        assert_eq!(r.start().to_u32(), 0);
        assert_eq!(r.end().to_u32(), 2);
        assert_eq!(text, "α");
    }

    #[test]
    fn narrow_edit_handles_pure_deletion() {
        let span = range(0, 3);
        let (r, text) = narrow_edit("ab".to_owned(), span, "abc").expect("differs");
        assert_eq!(r.start().to_u32(), 2);
        assert_eq!(r.end().to_u32(), 3);
        assert_eq!(text, "");
    }

    #[test]
    fn narrow_edit_handles_pure_insertion() {
        let span = range(0, 3);
        let (r, text) = narrow_edit("abxc".to_owned(), span, "abc").expect("differs");
        assert_eq!(r.start().to_u32(), 2);
        assert_eq!(r.end().to_u32(), 2);
        assert_eq!(text, "x");
    }

    #[test]
    fn narrow_edit_returns_none_when_text_equals_source() {
        assert!(narrow_edit("hello".to_owned(), range(0, 5), "hello").is_none());
    }

    #[test]
    fn narrow_edit_returns_whole_input_when_no_common_prefix_or_suffix() {
        let span = range(0, 3);
        let (r, text) = narrow_edit("abc".to_owned(), span, "xyz").expect("differs");
        assert_eq!(r.start().to_u32(), 0);
        assert_eq!(r.end().to_u32(), 3);
        assert_eq!(text, "abc");
    }

    #[test]
    fn narrow_edit_trims_common_prefix() {
        let span = range(0, 3);
        let (r, text) = narrow_edit("abc".to_owned(), span, "abd").expect("differs");
        assert_eq!(r.start().to_u32(), 2);
        assert_eq!(r.end().to_u32(), 3);
        assert_eq!(text, "c");
    }

    #[test]
    fn narrow_edit_trims_common_prefix_and_suffix() {
        let span = range(0, 7);
        let (r, text) = narrow_edit("ab1cdef".to_owned(), span, "ab2cdef").expect("differs");
        assert_eq!(r.start().to_u32(), 2);
        assert_eq!(r.end().to_u32(), 3);
        assert_eq!(text, "1");
    }

    #[test]
    fn narrow_edit_trims_common_suffix() {
        let span = range(0, 3);
        let (r, text) = narrow_edit("abc".to_owned(), span, "xbc").expect("differs");
        assert_eq!(r.start().to_u32(), 0);
        assert_eq!(r.end().to_u32(), 1);
        assert_eq!(text, "a");
    }
}
