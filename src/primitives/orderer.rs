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
    let lower = items[..i].last().map_or(outer.start(), |t| t.range().end());
    let upper = items.get(i + 1).map_or(outer.end(), |t| t.range().start());
    TextRange::new(
        leading_attached_start(source, item.start(), lower),
        source.text().line_end(item.end()).min(upper),
    )
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
    let blocks: Vec<TextRange> = items.iter().map(|t| t.range()).collect();
    let rendered: Vec<Cow<'src, str>> = blocks
        .iter()
        .enumerate()
        .map(|(i, &block)| render_block(i, source.slice(block)))
        .collect();
    let mut order: Vec<usize> = (0..items.len()).collect();
    let permuted = permute_in_place(&mut order, items, 0..items.len(), classify);
    let any_owned = rendered.iter().any(|c| matches!(c, Cow::Owned(_)));
    let span = blocks[0].cover(*blocks.last().expect("non-empty blocks"));
    if !permuted && !any_owned {
        return Cow::Borrowed(source.slice(span));
    }
    Cow::Owned(assemble_blocks(source, &blocks, &rendered, &order))
}

/// Splices each rendered child at its sorted position, copying the
/// source text between adjacent blocks verbatim. `blocks` must be
/// non-empty and in source order; `rendered` and `order` must have
/// the same length as `blocks`.
pub(crate) fn assemble_blocks<'src>(
    source: &'src Source,
    blocks: &[TextRange],
    rendered: &[Cow<'src, str>],
    order: &[usize],
) -> String {
    let span = blocks[0].cover(*blocks.last().expect("non-empty blocks"));
    let mut out = String::with_capacity(span.len().to_usize());
    for (i, &idx) in order.iter().enumerate() {
        out.push_str(&rendered[idx]);
        if let Some(next) = blocks.get(i + 1) {
            out.push_str(source.slice(TextRange::new(blocks[i].end(), next.start())));
        }
    }
    out
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
        .filter_map(|slot| classify(&items[order[slot]]).map(|k| (slot, (k, order[slot]))))
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

/// Walks backward through own-line comments preceding `item_start`,
/// stopping at the first comment that is inline (not own-line) or
/// separated from the running attachment point by a blank line.
fn leading_attached_start(source: &Source, item_start: TextSize, lower: TextSize) -> TextSize {
    let text = source.text();
    let mut current = text.line_start(item_start);
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
    use std::str::FromStr;

    use indoc::indoc;
    use ruff_text_size::TextLen;

    use super::*;

    fn body_range(source: &Source) -> TextRange {
        TextRange::up_to(source.text().text_len())
    }

    #[test]
    fn block_range_excludes_detached_comment_above_blank_line() {
        let src = indoc! {"
            # detached

            def a(): pass
        "};
        let source = Source::from_str(src).expect("parses");
        let block = block_range(&source, &source.ast().body, 0, body_range(&source));
        assert_eq!(source.slice(block), "def a(): pass");
    }

    #[test]
    fn block_range_extends_back_through_attached_comments() {
        let src = indoc! {"
            # one
            # two
            def a(): pass
        "};
        let source = Source::from_str(src).expect("parses");
        let block = block_range(&source, &source.ast().body, 0, body_range(&source));
        assert_eq!(source.slice(block), "# one\n# two\ndef a(): pass");
    }

    #[test]
    fn block_range_extends_forward_through_inline_trailing_comment() {
        let src = "def a(): pass  # trailing\n";
        let source = Source::from_str(src).expect("parses");
        let block = block_range(&source, &source.ast().body, 0, body_range(&source));
        assert_eq!(source.slice(block), "def a(): pass  # trailing");
    }

    #[test]
    fn block_range_lower_bound_blocks_back_extension_into_prior_item() {
        let src = "def a(): pass\ndef b(): pass\n";
        let source = Source::from_str(src).expect("parses");
        let block = block_range(&source, &source.ast().body, 1, body_range(&source));
        assert_eq!(source.slice(block), "def b(): pass");
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
        let src = "def f(b, a): pass\n";
        let source = Source::from_str(src).expect("parses");
        let func = source.ast().body[0].as_function_def_stmt().expect("def");
        let params = &func.parameters;
        let cow = reorder_text(
            &source,
            &params.args,
            |p| Some(p.parameter.name.as_str()),
            |_, slice| Cow::Borrowed(slice),
        );
        assert!(matches!(cow, Cow::Owned(_)));
        assert_eq!(&*cow, "a, b");
    }

    #[test]
    fn reorder_text_pins_non_classified() {
        let src = indoc! {"
            def b(): pass
            CONST = 1
            def a(): pass
        "};
        let source = Source::from_str(src).expect("parses");
        let body = &source.ast().body;
        let cow = reorder_text(
            &source,
            body,
            |stmt| stmt.as_function_def_stmt().map(|f| f.name.as_str()),
            |_, slice| Cow::Borrowed(slice),
        );
        assert_eq!(&*cow, "def a(): pass\nCONST = 1\ndef b(): pass");
    }

    #[test]
    fn reorder_text_returns_borrowed_when_already_sorted_and_no_render_change() {
        let src = "def a(): pass\ndef b(): pass\n";
        let source = Source::from_str(src).expect("parses");
        let cow = reorder_text(
            &source,
            &source.ast().body,
            |stmt| stmt.as_function_def_stmt().map(|f| f.name.as_str()),
            |_, slice| Cow::Borrowed(slice),
        );
        assert!(matches!(cow, Cow::Borrowed(_)));
    }

    #[test]
    fn reorder_text_returns_empty_borrowed_for_empty_items() {
        let source = Source::from_str("").expect("parses");
        let body = &source.ast().body;
        let cow = reorder_text(
            &source,
            body.as_slice(),
            |stmt: &ruff_python_ast::Stmt| stmt.as_function_def_stmt().map(|f| f.name.as_str()),
            |_, slice| Cow::Borrowed(slice),
        );
        assert!(matches!(cow, Cow::Borrowed("")));
    }

    #[test]
    fn reorder_text_returns_owned_when_render_block_owns_even_without_sort() {
        let src = "def a(): pass\ndef b(): pass\n";
        let source = Source::from_str(src).expect("parses");
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
        assert!(matches!(cow, Cow::Owned(_)));
        assert!((*cow).contains("def A"));
    }
}
