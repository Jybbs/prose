# Orderer

<PrimitiveLayout primitive="orderer">

*Orderer* reorders sibling AST nodes by a classifier closure while preserving attached comments and the interstitial text between adjacent items. [[alphabetize]] is the canonical consumer, but the primitive is shape-agnostic. Any rule that wants to permute siblings *(class-body members, dict items, `import` lines)* by some key reaches for the same machinery rather than re-implementing comment-attachment and gap-handling.


## Public Surface

*Orderer* lives at `src/primitives/orderer.rs` and is `pub(crate)`. The downstream-visible consequence is the rewrite [[alphabetize]] emits, with the reordered text landing in the `Edit` the rule produces.

The internal API stabilizes toward `1.0` where consumer-implemented ordering rules become reachable.

## Internal Surface

Three composable entry points cover every reorder shape.

### `permute_full(order, items, classify)`

Sorts the full `items` slice. `classify(&T) -> Option<K>` returns `Some(key)` for items participating in the sort and `None` for items that pin to their source slot. Returns `true` when the resulting order differs from source order, signaling the caller to emit an edit.

### `permute_in_place(order, items, range, classify)`

The sub-slice variant for partitioned reorders *(class bodies with a docstring pinned at the top, then alphabetize the rest)*.

### `assemble_blocks(source, blocks, rendered, order, gap_override)`

Splices the reordered children into a final string. `blocks` is the source-extent of each item *(via `block_range`)*, `rendered` is each item's text, `order` is the new arrangement, and `gap_override` lets a caller substitute the source gap between adjacent items when the reorder needs custom interstitial text.

The shape-agnostic `block_range(source, items, i, outer)` covers the *"what slice does item `i` occupy"* question for arbitrary `Ranged` types, including the leading comment-only lines directly above the item and the rest of its last line.

## How Comment Attachment Works

`block_range` extends each item's source extent upward to absorb every comment-only line directly above it *(with no intervening blank line)* and downward to the end of its last line. A comment that hugs a function definition stays glued to that function when the function moves, because the comment is part of the function's *"block"*. A blank line acts as a divider, leaving comments above the blank line attached to whatever sits above them rather than to the next item.

The interstitial text between adjacent items *(blank lines, sectioning comments)* stays in source position by default. A caller wanting per-slot custom interstitials passes a `gap_override` closure that returns `Some(text)` for slot `i` to substitute the inter-slot gap.

## Build Pattern

A rule reaches for *Orderer* in three steps:

1. Compute `Vec<TextRange>` of `blocks` via `block_range` for each item
2. Render each item's text *(`Cow::Borrowed(slice)` when unchanged or `Cow::Owned(...)` when the item itself rewrites)*
3. Compute the new `order` via `permute_full` or `permute_in_place` against a classifier
4. Call `assemble_blocks(source, &blocks, &rendered, &order, gap_override)` to produce the final string

The pattern handles partial reorders cleanly. Items with `classify -> None` pin in their slot, items with `Some(key)` redistribute through the remaining slots in key order.

## Re-Using This Primitive

Adding an ordering rule is shaped as *"decide what's a sibling, decide the classify function, decide whether some items pin"*. [[alphabetize]] is the canonical case where the classify function is the entry's name, every item participates, and `gap_override` substitutes `\n` or `\n\n` based on the per-context blank-line discipline.

<template #related>

- [[alphabetize]] is the canonical consumer
- [[edit]] is the output shape the assembled string folds into
- [[source]] is the input the block-range math reads against

</template>

</PrimitiveLayout>
