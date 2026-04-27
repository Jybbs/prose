//! Reorders sibling AST nodes by a key function while preserving
//! each item's leading attached comments and the rest of its last
//! line. `reorder` is the entry point. Items partition into
//! sections by an explicit `group` key, and each section sorts
//! independently. Interstitial text between adjacent items stays
//! in source position.

use ruff_diagnostics::Edit;
use ruff_python_trivia::CommentRanges;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::source::Source;

/// Returns the source-level extent of `items[i]`: the item's own
/// range plus any comment-only lines directly above it (no
/// intervening blank line) and the rest of its last line. Bounded
/// by the previous item's end (or `outer.start()` for the first
/// item) and the next item's start (or `outer.end()` for the last).
pub fn block_range<T: Ranged>(
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

/// Reorders `items` by `key` and pushes one replacement edit onto
/// `edits` when the resulting order differs from the input. Items
/// partition into sections by `group`: consecutive items sharing
/// the same value form one section, and each section sorts
/// independently. Interstitial content between adjacent items
/// (blank lines, detached comments) stays in source position.
/// `outer` bounds the first item's leading-comment scan and the
/// last item's trailing-content scan, typically the parent body's
/// range. Inputs of fewer than two items emit nothing.
pub fn reorder<'a, T, K, G>(
    source: &Source,
    items: &'a [T],
    outer: TextRange,
    mut key: impl FnMut(&'a T) -> K,
    mut group: impl FnMut(&'a T) -> G,
    edits: &mut Vec<Edit>,
) where
    T: Ranged,
    K: Ord,
    G: Eq,
{
    let (keys, groups): (Vec<K>, Vec<G>) = items.iter().map(|t| (key(t), group(t))).unzip();
    let Some(order) = section_sorted(&keys, &groups) else {
        return;
    };
    let blocks: Vec<TextRange> = (0..items.len())
        .map(|i| block_range(source, items, i, outer))
        .collect();
    edits.push(splice(source, &blocks, &order));
}

/// Returns indices `[0, keys.len())` reordered so that consecutive
/// equal-`groups` runs are sorted by `keys` independently. Returns
/// `None` when every adjacent same-group pair is already in order,
/// signaling that no reorder is needed.
fn section_sorted<K: Ord, G: Eq>(keys: &[K], groups: &[G]) -> Option<Vec<usize>> {
    keys.windows(2)
        .zip(groups.windows(2))
        .any(|(k, g)| g[0] == g[1] && k[0] > k[1])
        .then(|| {
            let mut order: Vec<usize> = (0..keys.len()).collect();
            for chunk in order.chunk_by_mut(|&a, &b| groups[a] == groups[b]) {
                chunk.sort_by_key(|&i| &keys[i]);
            }
            order
        })
}

/// Builds the spliced replacement edit: each output position writes
/// the block at `order[i]`, with the original gap between blocks
/// `i` and `i + 1` between consecutive writes.
fn splice(source: &Source, blocks: &[TextRange], order: &[usize]) -> Edit {
    let span = blocks[0].cover(*blocks.last().expect("non-empty blocks"));
    let mut out = String::with_capacity(span.len().to_usize());
    for (i, &idx) in order.iter().enumerate() {
        out.push_str(source.slice(blocks[idx]));
        if let Some(next) = blocks.get(i + 1) {
            out.push_str(source.slice(TextRange::new(blocks[i].end(), next.start())));
        }
    }
    Edit::range_replacement(out, span)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use indoc::indoc;
    use ruff_python_ast::Stmt;
    use ruff_text_size::TextLen;

    use super::*;

    fn body_range(source: &Source) -> TextRange {
        TextRange::up_to(source.text().text_len())
    }

    fn name(stmt: &Stmt) -> &str {
        match stmt {
            Stmt::ClassDef(c) => c.name.as_str(),
            Stmt::FunctionDef(f) => f.name.as_str(),
            _ => "",
        }
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
    fn reorder_already_sorted_input_emits_nothing() {
        let src = "def a(): pass\ndef b(): pass\n";
        let source = Source::from_str(src).expect("parses");
        let mut edits = Vec::new();
        reorder(
            &source,
            &source.ast().body,
            body_range(&source),
            name,
            |_| (),
            &mut edits,
        );
        assert!(edits.is_empty());
    }

    #[test]
    fn reorder_empty_input_emits_nothing() {
        let source = Source::from_str("").expect("parses");
        let mut edits = Vec::new();
        reorder(
            &source,
            &source.ast().body,
            body_range(&source),
            name,
            |_| (),
            &mut edits,
        );
        assert!(edits.is_empty());
    }

    #[test]
    fn reorder_single_item_input_emits_nothing() {
        let source = Source::from_str("def a(): pass\n").expect("parses");
        let mut edits = Vec::new();
        reorder(
            &source,
            &source.ast().body,
            body_range(&source),
            name,
            |_| (),
            &mut edits,
        );
        assert!(edits.is_empty());
    }

    #[test]
    fn section_sorted_already_sorted_returns_none() {
        let keys = ["a", "b", "c"];
        let groups = [(); 3];
        assert_eq!(section_sorted(&keys, &groups), None);
    }

    #[test]
    fn section_sorted_distinct_groups_return_none() {
        // Each item is its own group, so no within-section pair
        // can be out of order. No reorder is needed.
        let keys = ["b", "a", "d", "c"];
        let groups = [0, 1, 2, 3];
        assert_eq!(section_sorted(&keys, &groups), None);
    }

    #[test]
    fn section_sorted_sorts_within_each_section_independently() {
        // Two sections (group 0: indices 0..2, group 1: indices 2..4).
        // Each section sorts by key without crossing the boundary.
        let keys = ["b", "a", "d", "c"];
        let groups = [0, 0, 1, 1];
        assert_eq!(section_sorted(&keys, &groups), Some(vec![1, 0, 3, 2]));
    }

    #[test]
    fn section_sorted_unsorted_single_section_sorts_fully() {
        let keys = ["c", "a", "b"];
        let groups = [(); 3];
        assert_eq!(section_sorted(&keys, &groups), Some(vec![1, 2, 0]));
    }

    #[test]
    fn splice_writes_blocks_in_reordered_positions_with_original_gaps() {
        let src = "def b(): pass\ndef a(): pass\n";
        let source = Source::from_str(src).expect("parses");
        let blocks = [
            TextRange::new(0u32.into(), 13u32.into()),
            TextRange::new(14u32.into(), 27u32.into()),
        ];
        let edit = splice(&source, &blocks, &[1, 0]);
        assert_eq!(edit.start().to_u32(), 0);
        assert_eq!(edit.end().to_u32(), 27);
        assert_eq!(edit.content(), Some("def a(): pass\ndef b(): pass"));
    }
}
