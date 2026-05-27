# Aligner

<PrimitiveLayout primitive="aligner">

*Aligner* computes padding widths and emits the alignment edits that every alignment rule consumes. The shipped consumers ([[align-colons]], [[align-comparisons]], [[align-equals]], [[align-imports]], [[match-case-align]]) share the same column-resolution math, so the math lives once in *Aligner* and each rule supplies a member list plus a knob-set rather than re-implementing the resolution from scratch.


## Public Surface

*Aligner* lives at `src/primitives/aligner.rs` and is `pub(crate)`, so the type is reachable from inside the *Prose* crate but not from a downstream Rust caller in `0.2.x`. The downstream-visible consequence is the diagnostic stream the alignment rules emit, with the resolved column landing in the `Edit` each rule produces.

A downstream consumer in `0.2.x` can:

- Observe aligned source after running `prose format` or `prose check`
- See the resolved padding in the diagnostic `fix` payload of any alignment rule

A downstream consumer in `0.2.x` cannot directly construct a `Member`, drive `emit_group`, or read `Settings`. The `1.0` line opens the surface so a downstream can ship its own alignment rule against the same math.

## Internal Surface

The types every consumer touches:

1. `Member { gap: TextRange, line_start: TextSize, op_width: usize, width: usize }` describes one row in an alignment group. `gap` is the whitespace range immediately before the aligned token, rewritten into padding. `line_start` is the offset of the source-line start, used by `is_alignment_candidate` to confirm each member sits on its own line. `op_width` is the display width of variable-width operators *(`==`, `!=`, `<=`)* opting into right-alignment. `width` is the display-column width from member start to gap start, which is what the math compares to find the target column.
2. `Settings { max_shift, policy, strip_singleton_subgroup }` carries the rule's `[tool.prose.rules.<rule>]` knobs. `From<&AlignmentConfig>` builds the canonical settings, and `with_singleton_subgroup_strip` flips the singleton-collapse behavior on.
3. `AlignWalker { edits: Vec<Edit>, settings: Settings, source: &'a Source }` is the carrier each rule's visitor struct wraps. `AlignWalker::new(source, settings)` builds one with an empty `edits` vector, and `AlignWalker::emit_group(&mut self, members)` pushes alignment edits for a group into `self.edits`.

The entry point `emit_group(source: &Source, members: &[Member], settings: Settings, edits: &mut Vec<Edit>)` resolves the target column across `members`, falls back through `policy` *(`split` / `drop` / `skip`)* when the widest member exceeds `max_shift`, and pushes one `Edit` per row that needs padding into the caller's accumulator. A singleton group collapses its gap to one space, or to zero when `settings.strip_singleton_subgroup` is set.

### Supporting Helpers

A consuming rule rarely hand-builds the walker from raw AST traversal, since `aligner.rs` exposes a set of `pub(crate)` helpers covering the common shapes a new alignment rule needs:

1. `line_adjacent_groups(items, member_of)` partitions `items` into runs of line-adjacent siblings via `Source::is_line_adjacent`, then maps each item through `member_of`. Single-member runs drop out.
2. `parameter_split_groups(params, qualify)` walks a `Parameters` node and splits at the first parameter that does not qualify, used by rules over annotated function signatures.
3. `line_anchored_member(source, anchor)` builds a `Member` whose `gap` starts at `anchor` and whose `width` measures the leading display column on the line.
4. `line_anchored_member_at_kind(source, line, kind)` finds the first token of `kind` on `line` and anchors a `Member` at its end.
5. `range_anchored_member_single_line(source, range, anchor_of)` builds a `Member` whose `width` is the display-column width of `range`'s slice, for left-hand sides that are sub-ranges of one line.
6. `space_padding_edit(source, range, n)` produces a `Some(Edit)` replacing `range` with `n` spaces, or `None` when the current contents already match.
7. `is_alignment_candidate(members)` returns `true` when the group has at least two members and every member sits on a distinct line.

## How the Math Resolves

Aligners always carry a **one-space buffer** between content and the aligned token. The target column for a group is `max(member.width) + 1`, so every row whose existing column falls short of the target gets an `Edit` replacing its `gap` range with the right number of spaces, and rows already at the target stay unchanged without an edit.

When the widest member exceeds `max_shift`, the policy decides what happens next.

The `split` policy partitions the group into sub-groups of contiguous rows where the within-group widest is under `max_shift`, resolving each sub-group independently. The `drop` policy excludes the widest members from the padding calculation, leaving the group aligned to the widest non-overflow row. The `skip` policy leaves the whole group unaligned, with no edits emitted at all.

Variable-width operators opt in to right-alignment by setting `op_width`, shifting each row's padding inward by `max(op_width) - row.op_width`. [[align-comparisons]] is the shipped consumer of this hook, with the infrastructure leaving the door open for future variable-width-operator rules to land as a grouping walker plus a knob set rather than a from-scratch implementation.

## Build Pattern

Each alignment rule wraps an `AlignWalker` in its visitor struct, walks the AST, collects `Vec<Member>` per group, and calls `walker.emit_group(&members)` once per group. The grouping shapes are rule-specific *(consecutive assignments, dict items, `import` keywords, match-arm patterns)*, because the per-rule definition of *"what counts as a group"* varies, but the math afterward is shared across every alignment rule.

A rule's `apply` method takes the canonical shape:

```rust
struct Visitor<'a> {
    walker: AlignWalker<'a>,
}

impl Rule for MyAlignmentRule {
    fn apply(&self, source: &Source) -> Vec<Edit> {
        let mut visitor = Visitor {
            walker: AlignWalker::new(source, self.settings),
        };
        visitor.visit_body(&source.ast().body);
        visitor.walker.edits
    }
}

impl Visitor<'_> {
    fn process_body(&mut self, body: &[Stmt]) {
        for members in line_adjacent_groups(self.walker.source, body, |s| qualify(s)) {
            self.walker.emit_group(&members);
        }
    }
}
```

`line_adjacent_groups` handles the grouping for the common contiguous-statements shape, with the per-item qualifier folding through `line_anchored_member` or `line_anchored_member_at_kind` depending on whether the gap anchors at a known offset or at a specific token. `walker.emit_group(&members)` pushes per-row edits into `self.edits`, so the rule never has to thread a returned `Vec<Edit>` per group, and `apply` drains the accumulator from `visitor.walker.edits` at the end.

When the alignment context is `:`-shaped *(dict items, class fields, annotated parameters, docstring args, match arms)*, the grouping logic lives in [[colon-targets]] instead. A new colon-shaped rule implements `ColonEmitter`'s `handle` and `dict`/`match_arms` overrides, calls `walk(source)`, and forwards each yielded `&[Member]` slice to `walker.emit_group(&members)`.

## Re-Using This Primitive

Writing a new alignment rule comes down to wrapping an `AlignWalker` in a visitor struct, building the grouping logic that yields `Vec<Member>` per source-line run, and calling `walker.emit_group(&members)` per group. The padding math, the policy fallbacks, the singleton handling, and the right-alignment hook all carry through, leaving the rule to focus on its own grouping logic.

<template #related>

- [[align-colons]], [[align-comparisons]], [[align-equals]], [[align-imports]], and [[match-case-align]] are the consumers.
- [[colon-targets]] constructs `Member` lists from every `:` context, consumed by [[align-colons]] and [[singleton-rule]].
- [[edit]] is the shape `emit_group` pushes into the caller's accumulator.
- [[orderer]] composes line-adjacency grouping differently *(by source-range block extents rather than `Member` widths)*, so a rule whose math is reorder-shaped rather than padding-shaped reaches for that primitive instead.

</template>

</PrimitiveLayout>
