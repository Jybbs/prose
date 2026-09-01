//! Applies edit groups to source text, weaving overlapping groups and
//! declining any group that conflicts with itself.

use std::borrow::Cow;

use ruff_diagnostics::{Edit, SourceMap};
use ruff_text_size::{Ranged, TextLen, TextRange};

use super::*;
use crate::source::Source;

/// Splices `edits` into `text` and returns the resulting string, the
/// shape a caller reading no offsets takes. Declines with `None` on
/// overlapping edits, as [`apply_edits_mapped`] does.
pub(crate) fn apply_edits(text: &str, mut edits: Vec<Edit>) -> Option<String> {
    edits.sort_unstable();
    weave(text, TextRange::up_to(text.text_len()), &edits, None)
}

/// Splices `edits` into `text` and returns the resulting string beside a
/// [`SourceMap`] of one start-and-end marker per applied edit pairing
/// each original offset with its woven offset, or `None` when the sorted
/// edits overlap.
///
/// Sorts edits by start-then-end (via `Edit`'s `Ord` impl) and weaves
/// them in one forward pass, linear in the source length regardless of
/// how many edits apply. Declines with `None` rather than slicing an
/// inverted range, leaving the caller to keep the source unchanged.
pub(crate) fn apply_edits_mapped(text: &str, mut edits: Vec<Edit>) -> Option<(String, SourceMap)> {
    edits.sort_unstable();
    let mut source_map = SourceMap::default();
    let woven = weave(
        text,
        TextRange::up_to(text.text_len()),
        &edits,
        Some(&mut source_map),
    )?;
    Some((woven, source_map))
}

/// Folds any leaf edits whose range falls inside `range` into the
/// source slice for that range. Returns `Cow::Borrowed` when no leaf
/// edit applies or the in-range edits overlap. `edits` must be sorted
/// by `start()`, an invariant that `collect_leaf_edits` upholds
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
    weave(source.text(), range, inside, None)
        .map_or_else(|| Cow::Borrowed(source.slice(range)), Cow::Owned)
}

/// Splices `bodies` back into `block`, folding any leaf edits into the
/// pre-, inter-, and post-body gaps. `bodies` must be in source order. A
/// caller with no leaf edits passes an empty slice, leaving each gap a
/// borrow of the source.
pub(crate) fn splice_bodies<'src, I>(
    source: &'src Source,
    block: TextRange,
    bodies: I,
    leaf_edits: &[Edit],
) -> Cow<'src, str>
where
    I: IntoIterator<Item = (Cow<'src, str>, TextRange)>,
{
    let mut parts = Vec::new();
    let mut cursor = block.start();
    for (text, span) in bodies {
        parts.push(apply_inline_edits(
            source,
            TextRange::new(cursor, span.start()),
            leaf_edits,
        ));
        parts.push(text);
        cursor = span.end();
    }
    parts.push(apply_inline_edits(
        source,
        TextRange::new(cursor, block.end()),
        leaf_edits,
    ));
    concat_or_borrow(&parts, source, block)
}

/// Returns `Cow::Borrowed` of `source.slice(span)` when every part is
/// still a borrow of source, signalling no descendant rewrite fired.
/// Otherwise concatenates the parts into a single owned string covering
/// the same span.
fn concat_or_borrow<'src>(
    parts: &[Cow<'src, str>],
    source: &'src Source,
    span: TextRange,
) -> Cow<'src, str> {
    if any_owned(parts) {
        Cow::Owned(parts.concat())
    } else {
        Cow::Borrowed(source.slice(span))
    }
}

/// Weaves `edits` into the `span` slice of `text` and returns the
/// woven string, or `None` when two edits overlap. `edits` must be
/// sorted by start and lie within `span`, the overlap being an edit
/// whose start precedes the running cursor. A `Some` `source_map`
/// records a start-and-end marker per edit.
fn weave<'a>(
    text: &str,
    span: TextRange,
    edits: impl IntoIterator<Item = &'a Edit>,
    mut source_map: Option<&mut SourceMap>,
) -> Option<String> {
    let mut out = String::with_capacity(span.len().to_usize());
    let mut cursor = span.start();
    for edit in edits {
        if edit.start() < cursor {
            return None;
        }
        out.push_str(&text[TextRange::new(cursor, edit.start())]);
        if let Some(map) = source_map.as_deref_mut() {
            map.push_start_marker(edit, out.text_len());
        }
        out.push_str(edit.content().unwrap_or_default());
        if let Some(map) = source_map.as_deref_mut() {
            map.push_end_marker(edit, out.text_len());
        }
        cursor = edit.end();
    }
    out.push_str(&text[TextRange::new(cursor, span.end())]);
    Some(out)
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use super::*;
    use crate::testing::{parse, range};

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
    fn apply_edits_mapped_declines_overlapping_edits() {
        let out = apply_edits_mapped(
            "abcdef",
            vec![
                Edit::range_replacement("X".to_owned(), range(0, 3)),
                Edit::range_replacement("Y".to_owned(), range(2, 4)),
            ],
        );

        assert!(out.is_none());
    }

    #[test]
    fn apply_edits_mapped_pairs_each_edit_with_its_woven_offset() {
        let (text, map) = apply_edits_mapped(
            "abcdef",
            vec![Edit::range_replacement("XX".to_owned(), range(1, 2))],
        )
        .expect("woven");

        assert_eq!(text, "aXXcdef");
        let markers = map.markers();
        assert_eq!(markers.len(), 2);
        assert_eq!(markers[0].source(), TextSize::new(1));
        assert_eq!(markers[0].dest(), TextSize::new(1));
        assert_eq!(markers[1].source(), TextSize::new(2));
        assert_eq!(markers[1].dest(), TextSize::new(3));
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
}
