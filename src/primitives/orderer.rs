//! Reorders sibling AST nodes by a `classify` closure. Items
//! returning `None` pin in their source slot, and items returning
//! `Some(key)` redistribute across the remaining slots in `key`
//! order. Each item's extent comes from its `Ranged` impl, and
//! interstitial text between adjacent items stays in source
//! position.

use std::borrow::Cow;
use std::ops::Range;

use ruff_python_trivia::CommentRanges;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::source::Source;

/// Splices each rendered child at its sorted position. `gap_override`
/// returning `Some(text)` for new-order slot `i` substitutes that
/// text for the source gap between slot `i` and slot `i + 1`. A
/// `None` return copies the source gap verbatim. `blocks` must be
/// non-empty and in source order, with `rendered` and `order` the
/// same length as `blocks`.
pub(crate) fn assemble_blocks<'src>(
    source: &'src Source,
    blocks: &[TextRange],
    rendered: &[Cow<'src, str>],
    order: &[usize],
    mut gap_override: impl FnMut(usize) -> Option<&'src str>,
) -> String {
    let mut out = String::with_capacity(blocks_span(blocks).len().to_usize());
    for (i, (&idx, block)) in order.iter().zip(blocks).enumerate() {
        out.push_str(&rendered[idx]);
        if let Some(next) = blocks.get(i + 1) {
            let gap = gap_override(i)
                .unwrap_or_else(|| source.slice(TextRange::new(block.end(), next.start())));
            out.push_str(gap);
        }
    }
    out
}

/// Returns the source-level extent of `items[i]`, made of the
/// item's own range plus any comment-only lines directly above it
/// (no intervening blank line) and the rest of its last line.
/// Bounded by the previous item's end (or `outer.start()` for the
/// first item) and the next item's start (or `outer.end()` for the
/// last).
pub(crate) fn block_range<T: Ranged>(
    source: &Source,
    items: &[T],
    i: usize,
    outer: TextRange,
) -> TextRange {
    let item = items[i].range();
    let lower = items[..i].last().map_or(outer.start(), |t| t.end());
    let upper = items.get(i + 1).map_or(outer.end(), |t| t.start());
    TextRange::new(
        leading_attached_start(source, item.start(), lower),
        source.text().line_end(item.end()).min(upper),
    )
}

/// Total source extent covered by `blocks`. Requires non-empty input.
pub(crate) fn blocks_span(blocks: &[TextRange]) -> TextRange {
    blocks[0].cover(*blocks.last().expect("non-empty blocks"))
}

/// Convenience wrapper for `permute_in_place` over the full `items`
/// span. Shared by every caller that sorts the entire slice rather
/// than a sub-run.
pub(crate) fn permute_full<'a, T, K>(
    order: &mut [usize],
    items: &'a [T],
    classify: impl FnMut(&'a T) -> Option<K>,
) -> bool
where
    K: Ord,
{
    permute_in_place(order, items, 0..items.len(), classify)
}

/// Permutes the slots of `order` within `range` in place by sorting
/// items classified as `Some(K)`. Items returning `None` pin in their
/// current slot. Stable across equal keys. Returns `true` when the
/// permutation actually rewrote any slot.
pub(crate) fn permute_in_place<'a, T, K>(
    order: &mut [usize],
    items: &'a [T],
    range: Range<usize>,
    mut classify: impl FnMut(&'a T) -> Option<K>,
) -> bool
where
    K: Ord,
{
    let (slots, mut keyed): (Vec<usize>, Vec<(K, usize)>) = range
        .filter_map(|slot| {
            let src = order[slot];
            classify(&items[src]).map(|k| (slot, (k, src)))
        })
        .unzip();
    if keyed.is_sorted_by_key(|x| &x.0) {
        return false;
    }
    keyed.sort_by(|a, b| a.0.cmp(&b.0));
    for (slot, (_, src)) in slots.into_iter().zip(keyed) {
        order[slot] = src;
    }
    true
}

/// Recursive sibling rewriter. Each item gets its block source slice
/// passed to `render_block`, which returns either `Cow::Borrowed` (no
/// internal change) or `Cow::Owned` (subtree rewrote itself, e.g.
/// nested sort folded in). When the items don't need reordering at
/// this scope *and* every rendered child is borrowed, the function
/// returns `Cow::Borrowed(source.slice(blocks_span))` with no
/// allocation. Any other case returns `Cow::Owned(rendered)` covering
/// the same span, with each block's content placed by the sorted
/// order and the gaps between blocks copied verbatim from source.
pub(crate) fn reorder_text<'src, 'a, T, S, F>(
    source: &'src Source,
    items: &'a [T],
    classify: impl FnMut(&'a T) -> Option<S>,
    mut render_block: F,
) -> Cow<'src, str>
where
    T: Ranged,
    S: Ord,
    F: FnMut(usize, &'src str) -> Cow<'src, str>,
{
    if items.is_empty() {
        return Cow::Borrowed("");
    }
    let (blocks, rendered): (Vec<TextRange>, Vec<Cow<'src, str>>) = items
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let block = t.range();
            (block, render_block(i, source.slice(block)))
        })
        .unzip();
    let mut order: Vec<usize> = (0..items.len()).collect();
    let permuted = permute_full(&mut order, items, classify);
    if !permuted && rendered.iter().all(|c| matches!(c, Cow::Borrowed(_))) {
        return Cow::Borrowed(source.slice(blocks_span(&blocks)));
    }
    Cow::Owned(assemble_blocks(source, &blocks, &rendered, &order, |_| {
        None
    }))
}

/// Walks backward through own-line comments preceding `item_start`,
/// stopping at the first comment that is inline (not own-line) or
/// separated from the running attachment point by a blank line.
fn leading_attached_start(source: &Source, item_start: TextSize, lower: TextSize) -> TextSize {
    let text = source.text();
    let mut current = text.line_start(item_start);
    if lower > current {
        return item_start;
    }
    for comment in source
        .comment_ranges()
        .comments_in_range(TextRange::new(lower, current))
        .iter()
        .rev()
    {
        if !CommentRanges::is_own_line(comment.start(), text)
            || text.full_line_end(comment.start()) != current
        {
            break;
        }
        current = text.line_start(comment.start());
    }
    current
}

#[cfg(test)]
mod tests {
    use assert_matches::assert_matches;
    use indoc::indoc;
    use pretty_assertions::assert_eq;
    use ruff_text_size::TextLen;

    use super::*;
    use crate::test_support::parse;

    fn body_range(source: &Source) -> TextRange {
        TextRange::up_to(source.text().text_len())
    }

    fn borrow<'a>(_: usize, slice: &'a str) -> Cow<'a, str> {
        Cow::Borrowed(slice)
    }

    #[test]
    fn assemble_blocks_mixes_overridden_and_source_gaps() {
        let source = parse("def a(): pass\ndef b(): pass\ndef c(): pass\n");
        let blocks: Vec<TextRange> = source.ast().body.iter().map(|s| s.range()).collect();
        let rendered: Vec<Cow<str>> = blocks
            .iter()
            .map(|&b| Cow::Borrowed(source.slice(b)))
            .collect();
        let order = vec![0, 1, 2];
        let assembled = assemble_blocks(&source, &blocks, &rendered, &order, |i| {
            (i == 0).then_some(" ; ")
        });
        assert_eq!(assembled, "def a(): pass ; def b(): pass\ndef c(): pass");
    }

    #[test]
    fn assemble_blocks_substitutes_gap_when_override_returns_some() {
        let source = parse("def a(): pass\ndef b(): pass\n");
        let blocks: Vec<TextRange> = source.ast().body.iter().map(|s| s.range()).collect();
        let rendered: Vec<Cow<str>> = blocks
            .iter()
            .map(|&b| Cow::Borrowed(source.slice(b)))
            .collect();
        let order = vec![0, 1];
        let assembled = assemble_blocks(&source, &blocks, &rendered, &order, |_| Some(" ; "));
        assert_eq!(assembled, "def a(): pass ; def b(): pass");
    }

    #[test]
    fn block_range_excludes_detached_comment_above_blank_line() {
        let source = parse(indoc! {"
            # detached

            def a(): pass
        "});
        let block = block_range(&source, &source.ast().body, 0, body_range(&source));
        assert_eq!(source.slice(block), "def a(): pass");
    }

    #[test]
    fn block_range_extends_back_through_attached_comments() {
        let source = parse(indoc! {"
            # one
            # two
            def a(): pass
        "});
        let block = block_range(&source, &source.ast().body, 0, body_range(&source));
        assert_eq!(source.slice(block), "# one\n# two\ndef a(): pass");
    }

    #[test]
    fn block_range_extends_forward_through_inline_trailing_comment() {
        let source = parse("def a(): pass  # trailing\n");
        let block = block_range(&source, &source.ast().body, 0, body_range(&source));
        assert_eq!(source.slice(block), "def a(): pass  # trailing");
    }

    #[test]
    fn block_range_extends_to_end_of_final_line_for_multi_line_item() {
        let source = parse(indoc! {"
            def a(
                x,
                y,
            ): pass  # trailing
        "});
        let block = block_range(&source, &source.ast().body, 0, body_range(&source));
        assert_eq!(
            source.slice(block),
            "def a(\n    x,\n    y,\n): pass  # trailing"
        );
    }

    #[test]
    fn block_range_last_item_uses_outer_end_as_upper_bound() {
        let source = parse("def a(): pass\ndef b(): pass  # trailing\n");
        let body = &source.ast().body;
        let block = block_range(&source, body, body.len() - 1, body_range(&source));
        assert_eq!(source.slice(block), "def b(): pass  # trailing");
    }

    #[test]
    fn block_range_lower_bound_blocks_back_extension_into_prior_item() {
        let source = parse("def a(): pass\ndef b(): pass\n");
        let block = block_range(&source, &source.ast().body, 1, body_range(&source));
        assert_eq!(source.slice(block), "def b(): pass");
    }

    #[test]
    fn permute_in_place_leaves_slots_outside_range_untouched() {
        let mut order = vec![0, 1, 2, 3];
        let items = ["d", "c", "b", "a"];
        let permuted = permute_in_place(&mut order, &items, 1..3, |s: &&str| Some(*s));
        assert!(permuted);
        assert_eq!(order, vec![0, 2, 1, 3]);
    }

    #[test]
    fn permute_in_place_pins_unclassified() {
        let mut order = vec![0, 1, 2];
        let items = ["b", "x", "a"];
        let permuted = permute_in_place(&mut order, &items, 0..3, |s: &&str| {
            (*s != "x").then_some(*s)
        });
        assert!(permuted);
        assert_eq!(order, vec![2, 1, 0]);
    }

    #[test]
    fn permute_in_place_preserves_relative_order_of_equal_keys() {
        let mut order = vec![0, 1, 2, 3];
        let items = [(2, 'a'), (1, 'b'), (1, 'c'), (1, 'd')];
        let permuted = permute_in_place(&mut order, &items, 0..4, |t: &(u8, char)| Some(t.0));
        assert!(permuted);
        assert_eq!(order, vec![1, 2, 3, 0]);
    }

    #[test]
    fn permute_in_place_returns_false_when_already_sorted() {
        let mut order = vec![0, 1, 2];
        let items = ["a", "b", "c"];
        let permuted = permute_in_place(&mut order, &items, 0..3, |s: &&str| Some(*s));
        assert!(!permuted);
        assert_eq!(order, vec![0, 1, 2]);
    }

    #[test]
    fn permute_in_place_returns_false_when_fewer_than_two_classified() {
        let mut order = vec![0, 1, 2];
        let items = ["x", "x", "a"];
        let permuted = permute_in_place(&mut order, &items, 0..3, |s: &&str| {
            (*s != "x").then_some(*s)
        });
        assert!(!permuted);
        assert_eq!(order, vec![0, 1, 2]);
    }

    #[test]
    fn reorder_text_inline_swaps_two_items() {
        let source = parse("def f(b, a): pass\n");
        let func = source.ast().body[0].as_function_def_stmt().expect("def");
        let params = &func.parameters;
        let cow = reorder_text(
            &source,
            &params.args,
            |p| Some(p.parameter.name.as_str()),
            borrow,
        );
        assert_matches!(cow, Cow::Owned(_));
        assert_eq!(&*cow, "a, b");
    }

    #[test]
    fn reorder_text_pins_non_classified() {
        let source = parse(indoc! {"
            def b(): pass
            CONST = 1
            def a(): pass
        "});
        let body = &source.ast().body;
        let cow = reorder_text(
            &source,
            body,
            |stmt| stmt.as_function_def_stmt().map(|f| f.name.as_str()),
            borrow,
        );
        assert_eq!(&*cow, "def a(): pass\nCONST = 1\ndef b(): pass");
    }

    #[test]
    fn reorder_text_returns_borrowed_when_already_sorted_and_no_render_change() {
        let source = parse("def a(): pass\ndef b(): pass\n");
        let cow = reorder_text(
            &source,
            &source.ast().body,
            |stmt| stmt.as_function_def_stmt().map(|f| f.name.as_str()),
            borrow,
        );
        assert_matches!(cow, Cow::Borrowed(_));
    }

    #[test]
    fn reorder_text_returns_empty_borrowed_for_empty_items() {
        let source = parse("");
        let body = &source.ast().body;
        let cow = reorder_text(
            &source,
            body.as_slice(),
            |stmt: &ruff_python_ast::Stmt| stmt.as_function_def_stmt().map(|f| f.name.as_str()),
            borrow,
        );
        assert_matches!(cow, Cow::Borrowed(""));
    }

    #[test]
    fn reorder_text_returns_owned_when_render_block_owns_even_without_sort() {
        let source = parse("def a(): pass\ndef b(): pass\n");
        let cow = reorder_text(
            &source,
            &source.ast().body,
            |stmt| stmt.as_function_def_stmt().map(|f| f.name.as_str()),
            |i, slice| {
                if i == 0 {
                    Cow::Owned(slice.replace("def a", "def A"))
                } else {
                    Cow::Borrowed(slice)
                }
            },
        );
        assert_matches!(cow, Cow::Owned(_));
        assert!((*cow).contains("def A"));
    }

    #[test]
    fn reorder_text_returns_owned_when_sort_and_render_owned_combine() {
        let source = parse("def b(): pass\ndef a(): pass\n");
        let cow = reorder_text(
            &source,
            &source.ast().body,
            |stmt| stmt.as_function_def_stmt().map(|f| f.name.as_str()),
            |_, slice| Cow::Owned(slice.replace("def ", "DEF ")),
        );
        assert_matches!(cow, Cow::Owned(_));
        assert_eq!(&*cow, "DEF a(): pass\nDEF b(): pass");
    }
}
