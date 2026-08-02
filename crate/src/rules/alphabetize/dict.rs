//! Dict-literal reordering. Sorts a dict's keyed entries scalar-valued
//! before collection-valued and alphabetizes within each partition by the
//! key's source slice, a `**` spread bounding each run so no key crosses
//! it, folding nested reorders into each item block and declining when the
//! reassembled dict no longer parses.

use ruff_diagnostics::Edit;
use ruff_python_ast::{DictItem, ExprDict};
use ruff_python_parser::parse_expression;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::{
    primitives::{
        comments::has_keep_marker,
        edit::{any_owned, apply_inline_edits, splice_parses},
        effect::value_is_effectful,
        layout::is_layoutable,
        orderer::{
            adjacent_slots, any_sibling_shares_line, assemble_blocks, assemble_separated,
            block_ranges, permute_runs, runs_where,
        },
        range::blocks_span,
    },
    source::Source,
};

/// Rewrites a dict literal's items span. Returns `Some((span, text))`
/// when reordering, partition, or any nested reorder folded from `edits`
/// produces text different from the source slice. Returns `None` for
/// empty dicts, dicts marked `# prose: keep`, single-item dicts, and any
/// already-canonical case. `edits` are the leaf edits collected from the
/// dict's descendants, folded into each item block.
pub(super) fn rewrite_dict_text(
    source: &Source,
    d: &ExprDict,
    edits: &[Edit],
) -> Option<(TextRange, String)> {
    let [first, .., last] = d.items.as_slice() else {
        return None;
    };
    if has_keep_marker(source, d) {
        return None;
    }
    let multi_line = source.contains_line_break(first.range().cover(last.range()));
    // The block model decomposes one item per line. A multi-line dict that
    // packs entries onto a shared physical line has no such decomposition, so a
    // block reorder would reflow it. Decline, leaving it in source order.
    if multi_line && any_sibling_shares_line(source, &d.items) {
        return None;
    }
    // Widen each item to its value's paren-aware end, so a parenthesized
    // value keeps its closing parens inside the block rather than shedding
    // them into the separator tail.
    let item_ranges: Vec<TextRange> = d
        .items
        .iter()
        .map(|item| {
            let value = source.paren_aware_range((&item.value).into(), d.into());
            TextRange::new(item.start(), value.end())
        })
        .collect();
    let expanded = multi_line.then(|| block_ranges(source, &item_ranges, d.range()));
    let blocks = expanded.as_deref().unwrap_or(&item_ranges);
    let span = blocks_span(blocks);
    let block_texts: Vec<_> = blocks
        .iter()
        .map(|&block| apply_inline_edits(source, block, edits))
        .collect();
    let mut order: Vec<usize> = (0..d.len()).collect();
    let permuted = permute_runs(
        &mut order,
        &d.items,
        runs_where(&d.items, |item| item.key.is_some()),
        |item| dict_sort_key(source, item),
    );
    let assembled = if multi_line {
        let divider_slots = partition_divider_slots(source, &order, &d.items, &item_ranges);
        let source_last_has_comma = source.trailing_comma(d.range()).is_some();
        let value_ends: Vec<TextSize> = item_ranges.iter().map(Ranged::end).collect();
        assemble_separated(
            &value_ends,
            blocks,
            &block_texts,
            &order,
            &divider_slots,
            source_last_has_comma,
        )
    } else {
        assemble_blocks(source, blocks, &block_texts, &order, |_| None)
    };
    if !permuted && !any_owned(&block_texts) && assembled == source.slice(span) {
        return None;
    }
    // Decline the reorder when the reassembled dict no longer parses, the
    // safety net for irregular layouts (entries sharing a line, comments
    // inside a `**`-spread's parentheses) the block model cannot shuffle
    // cleanly.
    if !splice_parses(source, d.range(), span, &assembled, parse_expression) {
        return None;
    }
    Some((span, assembled))
}

/// Composite within-run dict-entry sort key, scalar-valued entries sorting
/// before collection-valued and alphabetizing within each partition by the
/// key's source slice. A keyless `**` spread returns `None`, as does an
/// entry whose value runs code, pinning that entry in its source slot.
fn dict_sort_key<'a>(source: &'a Source, item: &DictItem) -> Option<(bool, &'a str)> {
    let key = item
        .key
        .as_ref()
        .filter(|_| !value_is_effectful(&item.value))?;
    Some((is_layoutable(&item.value), source.slice(key)))
}

/// Returns the new-order slot indices after which a blank-line divider
/// should sit, one on either side of each keyed entry whose block spans
/// lines. A dict with fewer than two such entries yields none, leaving a
/// lone block entry to sit against its neighbors rather than behind a
/// divider.
fn partition_divider_slots(
    source: &Source,
    order: &[usize],
    items: &[DictItem],
    item_ranges: &[TextRange],
) -> Vec<usize> {
    let spans_lines =
        |i: usize| items[i].key.is_some() && source.contains_line_break(item_ranges[i]);
    if order.iter().filter(|&&i| spans_lines(i)).nth(1).is_none() {
        return Vec::new();
    }
    adjacent_slots(order, |_, a, b| spans_lines(a) || spans_lines(b))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{first_value, parse};

    #[rstest]
    #[case::string_literal("x = {\"a\": \"lit\"}\n", false)]
    #[case::implicit_concatenation("x = {\"a\": (\n    \"one \"\n    \"two\"\n)}\n", false)]
    #[case::subscript("x = {\"a\": data[k]}\n", false)]
    #[case::dict("x = {\"a\": {\"b\": 1}}\n", true)]
    #[case::list("x = {\"a\": [1, 2]}\n", true)]
    #[case::set("x = {\"a\": {1, 2}}\n", true)]
    #[case::tuple("x = {\"a\": (1, 2)}\n", true)]
    fn dict_sort_key_partitions_on_the_value_ast_kind(#[case] src: &str, #[case] collection: bool) {
        let source = parse(src);
        let dict = first_value(&source).as_dict_expr().expect("dict value");
        assert_eq!(
            dict_sort_key(&source, &dict.items[0]),
            Some((collection, "\"a\"")),
        );
    }

    #[rstest]
    fn dict_sort_key_pins_an_entry_whose_value_runs_code(
        #[values(
            "x = {\"a\": load()}\n",
            "x = {\"a\": [i for i in y]}\n",
            "x = {\"a\": [make()]}\n"
        )]
        src: &str,
    ) {
        let source = parse(src);
        let dict = first_value(&source).as_dict_expr().expect("dict value");
        assert!(dict_sort_key(&source, &dict.items[0]).is_none());
    }

    #[test]
    fn dict_sort_key_returns_none_for_a_spread_entry() {
        let source = parse("x = {**base, \"a\": 1}\n");
        let dict = first_value(&source).as_dict_expr().expect("dict value");
        assert!(dict_sort_key(&source, &dict.items[0]).is_none());
        assert!(dict_sort_key(&source, &dict.items[1]).is_some());
    }
}
