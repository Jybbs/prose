---
consumedBy: [alphabetize]
consumes: [edit, source]
layer: orchestration
stability: internal
summary: "Reorders sibling AST nodes by a classifier while preserving attached comments."
tagline: sibling reorder helper
---

# Orderer

<PrimitiveLayout primitive="orderer">

*Orderer* reorders sibling AST nodes by a classifier closure while preserving attached comments and the interstitial text between adjacent items. [[alphabetize]] is the canonical consumer, but the primitive is shape-agnostic. Any rule that wants to permute siblings *(class-body members, dict items, `import` lines)* by some key reaches for the same machinery rather than re-implementing comment-attachment and gap-handling.


## Public Surface

*Orderer* lives at `crate/src/primitives/orderer.rs` and is `pub(crate)`. The downstream-visible consequence is the rewrite [[alphabetize]] emits, with the reordered text landing in the `Edit` the rule produces.

The shape settles at `1.0`, where a downstream can register its own ordering rule against the same entry points.

## Internal Surface

Composable entry points cover every reorder shape, layered so a rule reaches for the highest-level one that fits.

### `reorder_text(source, items, classify, render)`

The high-level wrapper that covers the common case, rendering each item over its bare member span, sorting with `permute_full`, and assembling the result with the separators kept in the verbatim gaps between members. Returns the rewritten text alongside the block-extent span it covers, borrowing when no permutation is needed and every render is borrowed so a no-op rule pays no allocation. A one-member-per-line group whose members carry trailing comments reaches for `reorder_separated` instead.

### `reorder_separated(source, items, classify, render)`

The variant for a comma-separated group laid out one member per line, where a trailing inline comment must travel with its member through the sort. The separating comma is re-emitted per slot rather than left in a verbatim gap, because a comment ends its line and a moved member that gained or lost its source comma would otherwise strand the comma after the comment. An own-line comment sitting above a member rides with it as well, the block reaching back over the attached comment lines so an interstitial comment never strands in the slot the member vacated. A reparse guard declines the rewrite when an irregular layout reassembles into source the parser rejects.

### `permute_full(order, items, classify)`

Sorts the full `items` slice into a new ordering written into `order`. `classify(&T) -> Option<K>` returns `Some(key)` for items participating in the sort and `None` for items that pin to their source slot. Returns `true` when the resulting order differs from source order, signaling the caller to emit an edit. Reach for this when the rule needs to inspect the new order before assembling, or when the render step depends on the sort result.

### `permute_in_place(order, items, range, classify)`

The sub-slice variant for partitioned reorders, where part of the slice is held fixed and the rest reorders. *(A class body where the leading docstring pins at the top and the remaining methods alphabetize is the canonical example.)*

### `assemble_blocks(source, blocks, rendered, order, gap_override)`

Splices the reordered children into a final string the rule can emit as a single `Edit`. `blocks` is the source-extent of each item *(via `block_range`)*, `rendered` is each item's text, `order` is the new arrangement, and `gap_override` lets a caller substitute the gap between adjacent items in the **new** order *(the override's index parameter is the post-sort slot, not the source slot)*. Reach for this directly when a rule has already permuted and rendered through some other path.

### `assemble_separated(value_ends, blocks, block_texts, order, divider_slots, source_last_has_comma)`

The comma-aware counterpart to `assemble_blocks` for one-member-per-line groups. It splits each block into code, separator comma, and trailing comment at `value_ends`, then re-emits the comma after the value and before the comment per slot, so the comment rides with its member. Non-last slots always carry a comma, the new-last slot matches `source_last_has_comma`, and a blank line follows every slot in `divider_slots`. [[alphabetize]]'s dict and leaf reorders share it.

### `assemble_or_borrow(source, blocks, rendered, order, forced, gap)`

The borrow-aware finalizer over `assemble_blocks`, returning the assembled text alongside the block-extent span it covers. It short-circuits to `Cow::Borrowed(source.slice(span))` when no child rewrote and `order` is the identity permutation, so a no-op reorder pays no allocation, and assembles owned otherwise. `forced` overrides the short-circuit for a caller whose `gap` reshapes spacing without reordering, the case of an import run collapsing its blank lines in place. `reorder_text` and the recursive body rewriters in [[alphabetize]] and [[band-constants]] all finalize through it.

### Block-Geometry Helpers

`block_range(source, items, i, outer)` covers the *"what slice does item `i` occupy"* question for arbitrary `Ranged` types, including the leading comment-only lines directly above the item and the rest of its last line. `outer` bounds the leading-comment scan's lower edge to a parent extent *(the previous item's end, or `outer.start()` for the first item)*, while the forward scan reaches the next item's start or, for the last item, its own line end. At module scope a caller passes `TextRange::up_to(source.text().text_len())`, and at nested scope the caller computes the enclosing scope's extent. `blocks_span(blocks)` returns the union of every item's block range, used to size the outer `Edit` that replaces the reordered region.

## How Comment Attachment Works

`block_range` extends each item's source extent upward to absorb every comment-only line directly above it *(with no intervening blank line)* and downward to the end of its last line. A comment that hugs a function definition stays glued to that function when the function moves, because the comment is part of the function's *"block"*. A blank line acts as a divider, leaving comments above the blank line attached to whatever sits above them rather than to the next item.

A member's trailing inline comment travels with it too. For a comma-separated group reordered through `reorder_separated`, the separating comma is re-emitted per slot so the comment stays on its member's line rather than pinning to the slot the member vacated. A comment reached only over a closing `}`, `)`, or `]` belongs to the whole group rather than the last member, so it stays in source position.

The interstitial text between adjacent items *(blank lines, sectioning comments)* stays in source position by default. A caller wanting per-slot custom interstitials passes a `gap_override` closure that returns `Some(text)` for slot `i` to substitute the inter-slot gap.

## Build Pattern

A rule computes its `blocks` and per-item `rendered` text, then hands both to `reorder_text` along with a `classify` closure. `reorder_text` sorts, assembles, and short-circuits to `Cow::Borrowed(source.slice(span))` when no permutation is needed and every render is borrowed, so a no-op rule pays no allocation.

Reaching for `permute_full` and `assemble_blocks` directly is the manual path, useful when the rule needs to inspect the new order before assembling or render against a partially-permuted slice:

1. Compute `Vec<TextRange>` of `blocks` via `block_range` for each item.
2. Render each item's text *(`Cow::Borrowed(slice)` when unchanged or `Cow::Owned(...)` when the item itself rewrites)*.
3. Compute the new `order` via `permute_full` or `permute_in_place` against a classifier.
4. Call `assemble_blocks(source, &blocks, &rendered, &order, gap_override)` to produce the final string.

The pattern handles partial reorders cleanly, with items returning `classify -> None` pinning in their slot while items returning `Some(key)` redistribute through the remaining slots in key order.

## Re-Using This Primitive

Three decisions define an ordering rule: what counts as a sibling, how `classify` keys each sibling, and which items pin. [[alphabetize]] is the canonical case where `classify` returns the entry's name, every item participates, and `gap_override` substitutes `\n` or `\n\n` based on the per-context blank-line discipline.

<template #related>

- [[alphabetize]] is the canonical consumer.
- [[aligner]] composes line-adjacency grouping differently *(by `Member` widths rather than source-range block extents)*, so a rule whose math is padding-shaped rather than reorder-shaped reaches for that primitive instead.
- [[edit]] is the output shape the assembled string folds into.

</template>

</PrimitiveLayout>
