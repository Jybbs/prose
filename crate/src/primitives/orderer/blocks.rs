//! The source extent each member block covers, its attached comments
//! and trailing comma included.

use std::borrow::Cow;

use ruff_python_trivia::{CommentRanges, indentation_at_offset};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use super::assemble::Assembly;
use crate::{primitives::comments::bound_block_start, source::Source};

/// [`block_range`] for every slot of `items`, the marker-free counterpart
/// to [`member_blocks`] for a body with no section markers to floor against.
pub(crate) fn block_ranges<T: Ranged>(
    source: &Source,
    items: &[T],
    outer: TextRange,
) -> Vec<TextRange> {
    (0..items.len())
        .map(|i| block_range(source, items, i, outer))
        .collect()
}

/// Member blocks for every slot of `items`, the `Vec<TextRange>` a
/// section partition and a block reorder both read.
pub(crate) fn member_blocks<T: Ranged>(
    source: &Source,
    items: &[T],
    outer: TextRange,
) -> Vec<TextRange> {
    (0..items.len())
        .map(|i| member_block(source, items, i, outer))
        .collect()
}

/// True when only whitespace sits between `offset` and the start of its
/// physical line.
pub(crate) fn opens_its_line(source: &Source, offset: TextSize) -> bool {
    indentation_at_offset(offset, source.text()).is_some()
}

/// The [`Assembly`] over every slot of `items`, each member block
/// paired with the text `render` produces for it and the order seeded
/// to source order, which a recursive body rewriter folds its
/// descendant rewrites into before permuting.
pub(crate) fn rendered_member_blocks<'src, T: Ranged>(
    source: &'src Source,
    items: &'src [T],
    outer: TextRange,
    mut render: impl FnMut(&'src T, TextRange) -> Cow<'src, str>,
) -> Assembly<'src> {
    let (blocks, rendered) = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let block = member_block(source, items, i, outer);
            (block, render(item, block))
        })
        .unzip();
    Assembly {
        blocks,
        order: (0..items.len()).collect(),
        rendered,
    }
}

/// Lower bound of the backward comment scan for `items[i]`, the latest
/// of the previous item's end, `first` when the item has no predecessor,
/// and the start of the notebook cell holding the item. Flooring at the
/// cell keeps a block from reaching back over a cell boundary.
fn block_lower<T: Ranged>(
    source: &Source,
    items: &[T],
    i: usize,
    outer: TextRange,
    first: TextSize,
) -> TextSize {
    items[..i]
        .last()
        .map_or(first, Ranged::end)
        .max(source.cell_start(items[i].start()).unwrap_or(outer.start()))
}

/// Returns the source-level extent of `items[i]`: its own range, any
/// comment-only lines directly above it (no intervening blank line), and its
/// trailing comma and inline comment. Bounded below by the later of the
/// previous item's end (`outer.start()` for the first) and the item's own
/// notebook cell start, and forward by the next item's start, or [`tail_end`]
/// for the last item.
fn block_range<T: Ranged>(source: &Source, items: &[T], i: usize, outer: TextRange) -> TextRange {
    let item = items[i].range();
    let lower = block_lower(source, items, i, outer, outer.start());
    let forward = match items.get(i + 1) {
        Some(next) => source.text().line_end(item.end()).min(next.start()),
        None => tail_end(source, item.end()),
    };
    TextRange::new(leading_attached_start(source, item.start(), lower), forward)
}

/// True when the last member carries a trailing comma on its line.
pub(super) fn last_member_has_comma<T: Ranged>(source: &Source, items: &[T]) -> bool {
    let last = items.last().expect("non-empty items");
    source
        .slice(source.row_tail(last.end()))
        .trim_start()
        .starts_with(',')
}

/// Walks backward through own-line comments preceding `item_start`,
/// stopping at the first comment that is inline (not own-line) or
/// separated from the running attachment point by a blank line.
pub(super) fn leading_attached_start(
    source: &Source,
    item_start: TextSize,
    lower: TextSize,
) -> TextSize {
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

/// [`block_range`] for `items[i]` with its start settled by
/// [`bound_block_start`], so a comment run leading the member binds to
/// it across a blank line while a banner, hash heading, or suppression
/// directive stays in the gap rather than traveling through a reorder.
/// That gap is what [`Sections`](crate::primitives::sections::Sections)
/// reads to divide the body. Binding never reads the blank run, so a
/// block spans the same text either side of `space-statements`.
fn member_block<T: Ranged>(source: &Source, items: &[T], i: usize, outer: TextRange) -> TextRange {
    let raw = block_range(source, items, i, outer);
    // The first member has no predecessor to bound the gap, so its own
    // attached run stands in as the lower bound.
    let lower = block_lower(source, items, i, outer, raw.start());
    TextRange::new(
        bound_block_start(source, lower, items[i].start()),
        raw.end(),
    )
}

/// Extends `item_end` over a trailing comma and inline comment on its line,
/// reached across only commas and whitespace. Stops at any other token, so a
/// comment past a `}`, `)`, or `]` stays disowned.
pub(super) fn tail_end(source: &Source, item_end: TextSize) -> TextSize {
    let row = source.row_tail(item_end);
    let tail = source.slice(row).as_bytes();
    let consumed = tail
        .iter()
        .take_while(|&&byte| matches!(byte, b',' | b' ' | b'\t'))
        .count();
    if tail.get(consumed) == Some(&b'#') {
        return row.end();
    }
    item_end + TextSize::try_from(consumed).expect("a line fits u32")
}

#[cfg(test)]
mod tests {

    use indoc::indoc;

    use super::*;
    use crate::source::Source;
    use crate::testing::{first_class, first_value, parse};

    fn set_elts(source: &Source) -> &[ruff_python_ast::Expr] {
        first_value(source)
            .as_set_expr()
            .expect("set value")
            .elts
            .as_slice()
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
