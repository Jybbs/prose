//! Reorders sibling AST nodes by a `classify` closure. Items
//! returning `None` pin in their source slot, and items returning
//! `Some(key)` redistribute across the remaining slots in `key`
//! order. Each item's extent comes from its `Ranged` impl, and
//! interstitial text between adjacent items stays in source
//! position.

use ruff_python_ast::token::TokenKind;
use ruff_source_file::{LineRanges, UniversalNewlines};
use ruff_text_size::{Ranged, TextRange};

use crate::{
    primitives::{inline::display_width, range::blocks_span, tokens::tokens_within},
    source::Source,
};

mod assemble;
mod blocks;
mod permute;

pub(crate) use assemble::{
    Assembly, assemble_blocks, assemble_separated, reorder_separated, reorder_text,
};
pub(crate) use blocks::{block_ranges, member_blocks, opens_its_line, rendered_member_blocks};
pub(crate) use permute::{permute_full, permute_in_place, permute_runs};

use blocks::{last_member_has_comma, leading_attached_start, tail_end};
use permute::is_identity;

/// Slot indices `i` in `0..order.len() - 1` where the adjacent pair
/// `(order[i], order[i + 1])` satisfies `pred`, the sorted `Vec<usize>` an
/// `assemble_*` gap override binary-searches. `pred` receives the slot
/// alongside the pair, so a predicate keyed off the new-order position (a
/// section boundary) reads it without re-deriving.
pub(crate) fn adjacent_slots(
    order: &[usize],
    mut pred: impl FnMut(usize, usize, usize) -> bool,
) -> Vec<usize> {
    order
        .windows(2)
        .enumerate()
        .filter_map(|(slot, w)| pred(slot, w[0], w[1]).then_some(slot))
        .collect()
}

/// True when any adjacent pair of items in `body` shares one physical
/// line, as a `;`-joined statement run or a comma-packed entry run does.
pub(crate) fn any_sibling_shares_line<T: Ranged>(source: &Source, body: &[T]) -> bool {
    body.windows(2)
        .any(|pair| source.same_line(pair[0].end(), pair[1].start()))
}

/// True when every line of `span` rewritten to `assembled` fits inside
/// `budget` display columns or runs no wider than the widest line the
/// source span held, the head and tail of the boundary lines counted in.
pub(crate) fn reordered_lines_fit(
    source: &Source,
    span: TextRange,
    assembled: &str,
    budget: usize,
) -> bool {
    let text = source.text();
    let outer = text.lines_range(span);
    let head = source.slice(TextRange::new(outer.start(), span.start()));
    let tail = source.slice(TextRange::new(span.end(), outer.end()));
    let cap = source
        .slice(outer)
        .universal_newlines()
        .map(|line| display_width(line.as_str()))
        .fold(budget, usize::max);
    format!("{head}{assembled}{tail}")
        .universal_newlines()
        .all(|line| display_width(line.as_str()) <= cap)
}

/// True when `order` moves a member whose range spans lines, the
/// relocation an in-place swap cannot make because the member's
/// interior rows keep their source columns.
pub(crate) fn swap_relocates_spanning(
    source: &Source,
    order: &[usize],
    range_of: impl Fn(usize) -> TextRange,
) -> bool {
    order
        .iter()
        .enumerate()
        .any(|(slot, &idx)| slot != idx && source.contains_line_break(range_of(idx)))
}

/// True when a comment sits inside the swap span of `items`, from the
/// first member's start through the last member's tail.
pub(crate) fn swap_span_commented<T: Ranged>(source: &Source, items: &[T]) -> bool {
    let (Some(first), Some(last)) = (items.first(), items.last()) else {
        return false;
    };
    source.intersects_comment(TextRange::new(first.start(), tail_end(source, last.end())))
}

/// True where a leaf group over `items` holds its order as laid out:
/// `swapped` where the group packs members onto shared rows, opens
/// mid-row, or carries code in a gap, spanning lines, with a comment
/// inside the swap span.
pub(crate) fn swap_span_holds<T: Ranged>(source: &Source, items: &[T], swapped: bool) -> bool {
    swapped
        && items.len() > 1
        && source.contains_line_break(blocks_span(items))
        && swap_span_commented(source, items)
}

/// True when a group over `items` swaps member slices in place rather
/// than reordering one-per-line blocks: one packing members onto shared
/// rows, opening mid-row, or carrying code in its gaps.
pub(crate) fn swaps_in_place<T: Ranged>(source: &Source, items: &[T]) -> bool {
    items.first().is_some_and(|first| {
        any_sibling_shares_line(source, items)
            || !opens_its_line(source, first.start())
            || gaps_carry_code(source, items)
    })
}

/// True when a gap between two consecutive members of `items` carries
/// a token of its own past the separators and comments inside it, the
/// shape a positional argument sitting between two keywords takes.
pub(crate) fn gaps_carry_code<T: Ranged>(source: &Source, items: &[T]) -> bool {
    items.windows(2).any(|pair| {
        tokens_within(source, TextRange::new(pair[0].end(), pair[1].start()))
            .any(|token| !token.kind().is_trivia() && token.kind() != TokenKind::Comma)
    })
}

#[cfg(test)]
mod tests {

    use rstest::rstest;

    use super::*;
    use crate::testing::parse;

    #[test]
    fn adjacent_slots_collects_pairs_satisfying_the_predicate() {
        let order = [0, 2, 4, 6];
        let slots = adjacent_slots(&order, |slot, a, b| slot == 0 || a + b == 10);
        assert_eq!(slots, vec![0, 2]);
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
}
