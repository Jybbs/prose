---
consumedBy: [alphabetize-siblings]
consumes: [edit, source]
layer: orchestration
stability: internal
summary: "Reorders sibling AST nodes by a classifier closure and keeps each node's attached comments with it."
tagline: sibling reorder helper
---

# Orderer

<PrimitiveLayout primitive="orderer">

*Orderer* reorders sibling AST nodes by a classifier closure while keeping each node's attached comments with it and leaving the text between adjacent items in place. [[alphabetize-siblings]] is the canonical consumer, but the primitive is generic over the node kind. Any rule that permutes siblings *(class-body members, dict items, `import` lines)* by some key uses the same machinery rather than re-implementing comment attachment and gap handling.


## Public Surface

*Orderer* lives at `crate/src/primitives/orderer/` and is `pub(crate)`. What a downstream caller sees is the rewrite [[alphabetize-siblings]] emits, with the reordered text carried in the `Edit` the rule produces.

The API settles at `1.0`, where a downstream can register its own ordering rule against the same entry points.

## Internal Surface

Composable entry points cover every kind of reorder, layered so a rule uses the highest-level one that fits.

### `reorder_text(source, items, classify, render)`

The high-level wrapper for the common case, which renders each item over its bare member span, sorts with `permute_full`, and assembles the result with the separators kept in the verbatim gaps between members. It returns the rewritten text alongside the block-extent span it covers, borrowing when no permutation is needed and every render is borrowed, so a no-op rule pays no allocation. A one-member-per-line group whose members carry trailing comments uses `reorder_separated` instead.

### `reorder_separated(source, items, classify, render)`

The variant for a comma-separated group laid out one member per line, where a trailing inline comment must move with its member through the sort. The separating comma is re-emitted per slot rather than left in a verbatim gap, because a comment ends its line and a moved member that gained or lost its source comma would otherwise leave the comma stranded after the comment. An own-line comment sitting above a member moves with it as well, since the block reaches back over the attached comment lines, so a comment between members never stays behind in the slot the member vacated. A reparse guard declines the rewrite when an irregular layout reassembles into source the parser rejects.

### `permute_full(order, items, classify)`

Sorts the full `items` slice into a new ordering written into `order`. `classify(&T) -> Option<K>` returns `Some(key)` for an item that takes part in the sort and `None` for an item that stays in its source slot. `permute_full` returns `true` when the resulting order differs from source order, the signal for the caller to emit an edit. Use this when the rule inspects the new order before assembling, or when the render step depends on the sort result.

### `permute_in_place(order, items, range, classify)`

The sub-slice variant for a partitioned reorder, where part of the slice stays fixed and the rest reorders. *(A class body where the leading docstring stays at the top and the remaining methods alphabetize is the canonical example.)*

### `assemble_blocks(source, blocks, rendered, order, gap_override)`

Splices the reordered children into a final string the rule emits as a single `Edit`. `blocks` is the source extent of each item *(from `block_range`)*, `rendered` is each item's text, `order` is the new arrangement, and `gap_override` lets a caller substitute the gap between adjacent items in the **new** order *(the override's index parameter is the post-sort slot, not the source slot)*. Call this directly when a rule has already permuted and rendered through some other path.

### `assemble_separated(value_ends, blocks, block_texts, order, divider_slots, source_last_has_comma)`

The comma-aware counterpart to `assemble_blocks` for one-member-per-line groups. It splits each block into code, separator comma, and trailing comment at `value_ends`, then re-emits the comma after the value and before the comment per slot, so the comment stays with its member. Every slot but the last carries a comma, the new last slot matches `source_last_has_comma`, and a blank line follows every slot in `divider_slots`. [[alphabetize-siblings]]'s dict and leaf reorders share it. The `pub(crate)` signature in the crate takes `source` as its first argument ahead of `value_ends`.

### `Assembly { blocks, order, rendered }`

The three values a finalizer reads together, one member block per slot, the text rendered for each, and the slot order they assemble in. `rendered_member_blocks(source, items, outer, render)` builds one with the order seeded to source order, and a recursive body rewriter folds its descendant rewrites into it and permutes it before assembling. Both finalizers below are methods on the assembly.

`assembly.or_borrow(source, forced, gap)` is the borrow-aware finalizer over `assemble_blocks`, returning the assembled text alongside the block-extent span it covers. It short-circuits to `Cow::Borrowed(source.slice(span))` when no child rewrote and `order` is the identity permutation, so a no-op reorder pays no allocation, and assembles an owned string otherwise. `forced` overrides the short-circuit for a caller whose `gap` changes spacing without reordering, the case of an import run collapsing its blank lines in place or a band opening a blank line between its tiers. `reorder_text` and the recursive body rewriters in [[alphabetize-siblings]] and [[band-constants]] all finalize through it.

`assembly.cell_edits(source, forced, gap)` is the fix-group finalizer those same two rules emit from, splitting the assembly into one group per notebook cell the blocks span and one group for an ordinary module, each carrying the narrowed edits of its own slots. It reads `forced` the way `or_borrow` does, and a group whose pieces all reproduce the source is dropped rather than emitted.

### Block-Geometry Helpers

`block_range(source, items, i, outer)` answers the *"what slice does item `i` occupy"* question for arbitrary `Ranged` types, including the comment-only lines directly above the item and the rest of its last line. `outer` bounds the lower edge of the leading-comment scan to a parent extent *(the previous item's end, or `outer.start()` for the first item)*, and inside a notebook that edge is raised to the item's own cell start whenever that sits later, so an attached comment never reaches back across a cell boundary. The forward scan reaches the next item's start or, for the last item, its own line end. At module scope a caller passes `TextRange::up_to(source.text().text_len())`, and at nested scope the caller computes the enclosing scope's extent. `blocks_span(blocks)` returns the union of every item's block range, which sizes the outer `Edit` that replaces the reordered region. That helper is generic over `Ranged` and lives beside the parenthesis-aware ranges in `crate/src/primitives/range/`, since [[group-imports]], [[alphabetize-siblings]], [[unsorted-positionals]], and [[shed-backslash-continuations]] all cover a run of items the same way.

## How Comment Attachment Works

`block_range` extends each item's source extent upward to include every comment-only line directly above it *(with no intervening blank line)* and downward to the end of its last line. A comment directly above a function definition moves with that function, because the comment is part of the function's *"block"*. A blank line acts as a divider, leaving comments above the blank line attached to whatever sits above them rather than to the next item.

A member's trailing inline comment moves with it too. For a comma-separated group reordered through `reorder_separated`, the separating comma is re-emitted per slot so the comment stays on its member's line rather than staying behind in the slot the member vacated. A comment reached only over a closing `}`, `)`, or `]` belongs to the whole group rather than the last member, so it stays in source position.

The text between adjacent items *(blank lines, sectioning comments)* stays in source position by default. A caller that needs a custom gap per slot passes a `gap_override` closure that returns `Some(text)` for slot `i` to substitute the gap after that slot.

## Build Pattern

A rule hands `reorder_text` its items, a `classify` closure, and a `render` closure, and the helper computes each item's block and rendered text itself. `reorder_text` sorts, assembles, and short-circuits to `Cow::Borrowed(source.slice(span))` when no permutation is needed and every render is borrowed, so a no-op rule pays no allocation.

Calling `permute_full` and `assemble_blocks` directly is the manual path, for a rule that inspects the new order before assembling or renders against a partially permuted slice:

1. Compute the `Vec<TextRange>` of `blocks` through `block_ranges`, the `pub(crate)` wrapper over `block_range`.
2. Render each item's text *(`Cow::Borrowed(slice)` when unchanged or `Cow::Owned(...)` when the item itself rewrites)*.
3. Compute the new `order` through `permute_full` or `permute_in_place` against a classifier.
4. Call `assemble_blocks(source, &blocks, &rendered, &order, gap_override)` to produce the final string.

The pattern handles a partial reorder cleanly, in that an item returning `classify -> None` stays in its slot while the items returning `Some(key)` redistribute through the remaining slots in key order.

## Re-Using This Primitive

Three decisions define an ordering rule: what counts as a sibling, how `classify` keys each sibling, and which items stay in place. [[alphabetize-siblings]] is the canonical case, where `classify` returns the entry's name, every item takes part, and `gap_override` substitutes `\n` or `\n\n` according to the per-context blank-line rule.

<template #related>

- [[alphabetize-siblings]] is the canonical consumer.
- [[aligner]] groups line-adjacent items differently *(by `Member` widths rather than source-range block extents)*, so a rule that pads rather than reorders uses that primitive instead.
- [[edit]] is the output type the assembled string is written into.

</template>

</PrimitiveLayout>
