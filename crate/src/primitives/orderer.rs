//! Reorders sibling AST nodes by a `classify` closure. Items
//! returning `None` pin in their source slot, and items returning
//! `Some(key)` redistribute across the remaining slots in `key`
//! order. Each item's extent comes from its `Ranged` impl, and
//! interstitial text between adjacent items stays in source
//! position.

use std::{borrow::Cow, ops::Range};

use ruff_python_trivia::CommentRanges;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::{
    primitives::{comments::marker_floor, edit::splice_parses},
    source::Source,
};

/// True when any adjacent pair of items in `body` shares one physical line.
/// A block-based reorder decomposes one item per line, so a body packing
/// two onto a line (`;`-joined statements, comma-packed entries) has no such
/// decomposition and keeps source order.
pub(crate) fn any_sibling_shares_line<T: Ranged>(source: &Source, body: &[T]) -> bool {
    body.windows(2)
        .any(|pair| source.same_line(pair[0].end(), pair[1].start()))
}

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

/// Concatenates `block_texts` in `order`, re-emitting each member's comma so
/// it lands after the value and before any trailing comment. `value_ends`
/// split the code from each comma-and-comment tail. Non-last slots carry a
/// comma, the new-last slot matches `source_last_has_comma`, and a blank line
/// follows every slot in `divider_slots`.
pub(crate) fn assemble_separated(
    value_ends: &[TextSize],
    blocks: &[TextRange],
    block_texts: &[Cow<'_, str>],
    order: &[usize],
    divider_slots: &[usize],
    source_last_has_comma: bool,
) -> String {
    let mut out = String::with_capacity(blocks_span(blocks).len().to_usize());
    for (slot, &idx) in order.iter().enumerate() {
        let block_text = &block_texts[idx];
        let tail_len = (blocks[idx].end() - value_ends[idx]).to_usize();
        let (code, tail) = block_text.split_at(block_text.len() - tail_len);
        let (separator, comment) = tail.split_at(tail.find('#').unwrap_or(tail.len()));
        out.push_str(code);
        let is_last = slot + 1 == order.len();
        if !is_last || source_last_has_comma {
            out.push(',');
        }
        if !comment.is_empty() {
            out.extend(separator.chars().filter(|&c| c != ','));
            out.push_str(comment);
        }
        if !is_last {
            out.push('\n');
            if divider_slots.binary_search(&slot).is_ok() {
                out.push('\n');
            }
        }
    }
    out
}

/// Returns the source-level extent of `items[i]`: its own range, any
/// comment-only lines directly above it (no intervening blank line), and its
/// trailing comma and inline comment. Bounded below by the previous item's end
/// (or `outer.start()` for the first), and forward by the next item's start, or
/// for the last item by [`tail_end`], which stops at a closing delimiter on the
/// line rather than crossing it.
pub(crate) fn block_range<T: Ranged>(
    source: &Source,
    items: &[T],
    i: usize,
    outer: TextRange,
) -> TextRange {
    let item = items[i].range();
    let lower = items[..i].last().map_or(outer.start(), Ranged::end);
    let forward = match items.get(i + 1) {
        Some(next) => source.text().line_end(item.end()).min(next.start()),
        None => tail_end(source, item.end()),
    };
    TextRange::new(leading_attached_start(source, item.start(), lower), forward)
}

/// Total source extent covered by `blocks`. Requires non-empty input.
pub(crate) fn blocks_span(blocks: &[TextRange]) -> TextRange {
    blocks[0].cover(*blocks.last().expect("non-empty blocks"))
}

/// Returns the slot ranges of consecutive items whose pairwise neighbors
/// satisfy `adjacent`. Singleton runs drop.
pub(crate) fn chunk_runs<T>(
    items: &[T],
    mut adjacent: impl FnMut(&T, &T) -> bool,
) -> Vec<Range<usize>> {
    let mut start = 0;
    items
        .chunk_by(|a, b| adjacent(a, b))
        .filter_map(|chunk| {
            let end = start + chunk.len();
            let range = (chunk.len() >= 2).then_some(start..end);
            start = end;
            range
        })
        .collect()
}

/// [`block_range`] for `items[i]` with its start pushed below any section
/// marker leading it, so a banner or hash heading stays in the gap above
/// the member rather than traveling with it through a reorder. The
/// marker-bearing gap is what [`Sections`](crate::primitives::sections::Sections)
/// reads to divide the body.
pub(crate) fn member_block<T: Ranged>(
    source: &Source,
    items: &[T],
    i: usize,
    outer: TextRange,
) -> TextRange {
    let raw = block_range(source, items, i, outer);
    TextRange::new(
        marker_floor(source, raw.start(), items[i].start()),
        raw.end(),
    )
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

/// Reorders a comma-separated group laid out one member per line, the comma
/// re-emitted per slot so each member's trailing comment travels with it. Each
/// block reaches back over the own-line comments attached above its member and
/// forward through any trailing comma and comment, so both ride with the member.
/// Declines, returning a borrow, when nothing reorders or the reassembled group
/// no longer parses.
pub(crate) fn reorder_separated<'src, 'a, T, S, F>(
    source: &'src Source,
    items: &'a [T],
    classify: impl FnMut(&'a T) -> Option<S>,
    mut render_block: F,
) -> (Cow<'src, str>, TextRange)
where
    T: Ranged,
    S: Ord,
    F: FnMut(usize, TextRange) -> Cow<'src, str>,
{
    let text = source.text();
    let (blocks, block_texts): (Vec<TextRange>, Vec<Cow<'src, str>>) = items
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let start = match items[..i].last() {
                Some(prev) => leading_attached_start(source, t.start(), prev.end()),
                None => text.line_start(t.start()),
            };
            let block = TextRange::new(start, tail_end(source, t.end()));
            (block, render_block(i, block))
        })
        .unzip();
    let span = blocks_span(&blocks);
    let mut order: Vec<usize> = (0..items.len()).collect();
    let permuted = permute_full(&mut order, items, classify);
    if !permuted && block_texts.iter().all(|c| matches!(c, Cow::Borrowed(_))) {
        return (Cow::Borrowed(source.slice(span)), span);
    }
    let value_ends: Vec<TextSize> = items.iter().map(Ranged::end).collect();
    let assembled = assemble_separated(
        &value_ends,
        &blocks,
        &block_texts,
        &order,
        &[],
        last_member_has_comma(source, items),
    );
    let module = source.module_range();
    if assembled == source.slice(span)
        || !splice_parses(source, module, span, &assembled, str::parse::<Source>)
    {
        return (Cow::Borrowed(source.slice(span)), span);
    }
    (Cow::Owned(assembled), span)
}

/// Reorders sibling members by `classify`, the separators kept in the
/// verbatim gaps between bare member spans, `render_block` rewriting each
/// member's slice. Returns the rewritten text and the span it covers. A
/// multi-line group whose members carry trailing comments uses
/// `reorder_separated` instead.
pub(crate) fn reorder_text<'src, 'a, T, S, F>(
    source: &'src Source,
    items: &'a [T],
    classify: impl FnMut(&'a T) -> Option<S>,
    mut render_block: F,
) -> (Cow<'src, str>, TextRange)
where
    T: Ranged,
    S: Ord,
    F: FnMut(usize, TextRange) -> Cow<'src, str>,
{
    if items.is_empty() {
        return (Cow::Borrowed(""), TextRange::default());
    }
    let (blocks, rendered): (Vec<TextRange>, Vec<Cow<'src, str>>) = items
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let block = t.range();
            (block, render_block(i, block))
        })
        .unzip();
    let span = blocks_span(&blocks);
    let mut order: Vec<usize> = (0..items.len()).collect();
    let permuted = permute_full(&mut order, items, classify);
    if !permuted && rendered.iter().all(|c| matches!(c, Cow::Borrowed(_))) {
        return (Cow::Borrowed(source.slice(span)), span);
    }
    (
        Cow::Owned(assemble_blocks(source, &blocks, &rendered, &order, |_| {
            None
        })),
        span,
    )
}

/// Inverts `order` into the slot each item index occupies, the reverse
/// of the index-per-slot mapping `order` itself holds. Reading
/// `slot_positions(order)[idx]` answers where item `idx` landed.
pub(crate) fn slot_positions(order: &[usize]) -> Vec<usize> {
    let mut positions = vec![0usize; order.len()];
    for (slot, &idx) in order.iter().enumerate() {
        positions[idx] = slot;
    }
    positions
}

/// True when the last member carries a trailing comma on its line.
fn last_member_has_comma<T: Ranged>(source: &Source, items: &[T]) -> bool {
    let last = items.last().expect("non-empty items");
    let line_end = source.text().line_end(last.end());
    source
        .slice(TextRange::new(last.end(), line_end))
        .trim_start()
        .starts_with(',')
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

/// Extends `item_end` over a trailing comma and inline comment on its line,
/// reached across only commas and whitespace. Stops at any other token, so a
/// comment past a `}`, `)`, or `]` stays disowned.
fn tail_end(source: &Source, item_end: TextSize) -> TextSize {
    let line_end = source.text().line_end(item_end);
    let mut consumed = 0u32;
    for &byte in source.slice(TextRange::new(item_end, line_end)).as_bytes() {
        match byte {
            b',' | b' ' | b'\t' => consumed += 1,
            b'#' => return line_end,
            _ => break,
        }
    }
    item_end + TextSize::from(consumed)
}

#[cfg(test)]
mod tests {
    use assert_matches::assert_matches;
    use indoc::indoc;
    use rstest::rstest;

    use super::*;
    use crate::testing::{first_class, first_def, parse};

    fn set_elts(source: &Source) -> &[ruff_python_ast::Expr] {
        source.ast().body[0]
            .as_assign_stmt()
            .expect("assign statement")
            .value
            .as_set_expr()
            .expect("set value")
            .elts
            .as_slice()
    }

    #[rstest]
    #[case("import b\nimport a; x = 1\n", true)]
    #[case("import b\nimport a\n", false)]
    #[case("a = 1; b = 2\n", true)]
    #[case("x = 1\n", false)]
    fn any_sibling_shares_line_detects_line_packed_pairs(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        let source = parse(src);
        assert_eq!(
            any_sibling_shares_line(&source, &source.ast().body),
            expected
        );
    }

    #[test]
    fn assemble_blocks_mixes_overridden_and_source_gaps() {
        let source = parse("def a(): pass\ndef b(): pass\ndef c(): pass\n");
        let blocks: Vec<TextRange> = source.ast().body.iter().map(Ranged::range).collect();
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
        let blocks: Vec<TextRange> = source.ast().body.iter().map(Ranged::range).collect();
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
        let block = block_range(&source, &source.ast().body, 0, source.module_range());
        assert_eq!(source.slice(block), "def a(): pass");
    }

    #[test]
    fn block_range_extends_back_through_attached_comments() {
        let source = parse(indoc! {"
            # one
            # two
            def a(): pass
        "});
        let block = block_range(&source, &source.ast().body, 0, source.module_range());
        assert_eq!(source.slice(block), "# one\n# two\ndef a(): pass");
    }

    #[test]
    fn block_range_extends_forward_through_inline_trailing_comment() {
        let source = parse("def a(): pass  # trailing\n");
        let block = block_range(&source, &source.ast().body, 0, source.module_range());
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
        let block = block_range(&source, &source.ast().body, 0, source.module_range());
        assert_eq!(
            source.slice(block),
            "def a(\n    x,\n    y,\n): pass  # trailing"
        );
    }

    #[test]
    fn block_range_last_item_keeps_trailing_comment_past_outer_end() {
        let source = parse("class C:\n    a = 1\n    b = 2  # trailing\n");
        let class = first_class(&source);
        let block = block_range(&source, &class.body, class.body.len() - 1, class.range());
        assert_eq!(source.slice(block), "    b = 2  # trailing");
    }

    #[test]
    fn block_range_last_item_takes_trailing_comment_at_module_scope() {
        let source = parse("def a(): pass\ndef b(): pass  # trailing\n");
        let body = &source.ast().body;
        let block = block_range(&source, body, body.len() - 1, source.module_range());
        assert_eq!(source.slice(block), "def b(): pass  # trailing");
    }

    #[test]
    fn block_range_lower_bound_blocks_back_extension_into_prior_item() {
        let source = parse("def a(): pass\ndef b(): pass\n");
        let block = block_range(&source, &source.ast().body, 1, source.module_range());
        assert_eq!(source.slice(block), "def b(): pass");
    }

    #[test]
    fn chunk_runs_returns_runs_of_two_or_more_dropping_singletons() {
        let items = [1, 1, 2, 3, 3, 3];
        assert_eq!(chunk_runs(&items, |a, b| a == b), vec![0..2, 3..6]);
    }

    #[test]
    fn last_member_has_comma_false_at_closing_delimiter() {
        let source = parse("x = {\n    a,\n    b\n}\n");
        assert!(!last_member_has_comma(&source, set_elts(&source)));
    }

    #[test]
    fn last_member_has_comma_true_with_trailing_comma() {
        let source = parse("x = {\n    a,\n    b,\n}\n");
        assert!(last_member_has_comma(&source, set_elts(&source)));
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
        let func = first_def(&source);
        let params = &func.parameters;
        let (cow, _) = reorder_text(
            &source,
            &params.args,
            |p| Some(p.parameter.name.as_str()),
            |_, block| Cow::Borrowed(source.slice(block)),
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
        let (cow, _) = reorder_text(
            &source,
            body,
            |stmt| stmt.as_function_def_stmt().map(|f| f.name.as_str()),
            |_, block| Cow::Borrowed(source.slice(block)),
        );
        assert_eq!(&*cow, "def a(): pass\nCONST = 1\ndef b(): pass");
    }

    #[test]
    fn reorder_text_returns_borrowed_when_already_sorted_and_no_render_change() {
        let source = parse("def a(): pass\ndef b(): pass\n");
        let (cow, _) = reorder_text(
            &source,
            &source.ast().body,
            |stmt| stmt.as_function_def_stmt().map(|f| f.name.as_str()),
            |_, block| Cow::Borrowed(source.slice(block)),
        );
        assert_matches!(cow, Cow::Borrowed(_));
    }

    #[test]
    fn reorder_text_returns_empty_borrowed_for_empty_items() {
        let source = parse("");
        let body = &source.ast().body;
        let (cow, _) = reorder_text(
            &source,
            body.as_slice(),
            |stmt: &ruff_python_ast::Stmt| stmt.as_function_def_stmt().map(|f| f.name.as_str()),
            |_, block| Cow::Borrowed(source.slice(block)),
        );
        assert_matches!(cow, Cow::Borrowed(""));
    }

    #[test]
    fn reorder_text_returns_owned_when_render_block_owns_even_without_sort() {
        let source = parse("def a(): pass\ndef b(): pass\n");
        let (cow, _) = reorder_text(
            &source,
            &source.ast().body,
            |stmt| stmt.as_function_def_stmt().map(|f| f.name.as_str()),
            |i, block| {
                let slice = source.slice(block);
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
        let (cow, _) = reorder_text(
            &source,
            &source.ast().body,
            |stmt| stmt.as_function_def_stmt().map(|f| f.name.as_str()),
            |_, block| Cow::Owned(source.slice(block).replace("def ", "DEF ")),
        );
        assert_matches!(cow, Cow::Owned(_));
        assert_eq!(&*cow, "DEF a(): pass\nDEF b(): pass");
    }

    #[test]
    fn slot_positions_inverts_an_order() {
        assert_eq!(slot_positions(&[2, 0, 1]), vec![1, 2, 0]);
    }

    #[test]
    fn tail_end_disowns_comment_past_closing_delimiter() {
        let source = parse("x = {\n    a,\n    b}  # tail\n");
        let last = set_elts(&source).last().expect("two elements");
        assert_eq!(tail_end(&source, last.end()), last.end());
    }

    #[test]
    fn tail_end_owns_comma_and_comment() {
        let source = parse("x = {\n    a,  # keep\n    b,\n}\n");
        let elts = set_elts(&source);
        let end = tail_end(&source, elts[0].end());
        assert_eq!(
            source.slice(TextRange::new(elts[0].start(), end)),
            "a,  # keep"
        );
    }

    #[test]
    fn tail_end_takes_comma_without_a_comment() {
        let source = parse("x = {\n    a,\n    b,\n}\n");
        let elts = set_elts(&source);
        let end = tail_end(&source, elts[0].end());
        assert_eq!(source.slice(TextRange::new(elts[0].start(), end)), "a,");
    }
}
