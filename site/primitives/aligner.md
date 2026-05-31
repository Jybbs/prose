---
stability: internal
---

# Aligner

<PrimitiveLayout primitive="aligner">

*Aligner* computes padding widths and emits the alignment edits that every alignment rule consumes. The shipped consumers ([[align-colons]], [[align-comparisons]], [[align-equals]], [[align-imports]], [[align-match-case]]) share the same column-resolution math, so the math lives once in *Aligner* and each rule supplies a member list plus a knob-set rather than re-implementing the resolution from scratch.


## Public Surface

*Aligner* lives at `src/primitives/aligner.rs` and is `pub(crate)`, so the type is reachable from inside the *Prose* crate but not from a downstream Rust caller in `0.2.x`. The downstream-visible consequence is the diagnostic stream the alignment rules emit, with the resolved column landing in the `Edit` each rule produces.

A downstream consumer in `0.2.x` can:

- Observe aligned source after running `prose format` or `prose check`
- See the resolved padding in the diagnostic `fix` payload of any alignment rule

A downstream consumer in `0.2.x` cannot directly construct a `Member`, drive `emit_group`, or read `Settings`. The `1.0` line opens the surface so a downstream can ship its own alignment rule against the same math.

## Internal Surface

The types every consumer touches:

1. `Member { gap: TextRange, line_start: TextSize, op_width: usize, width: usize }` describes one row in an alignment group. `gap` is the whitespace range immediately before the aligned token, rewritten into padding. `line_start` is the offset of the source-line start, used by `is_alignment_candidate` to confirm each member sits on its own line. `op_width` is the display width of variable-width operators *(`==`, `!=`, `<=`)* opting into right-alignment. `width` is the display-column width from member start to gap start, which is what the math compares to find the target column.
2. `Settings { max_shift, policy, strip_singleton_subgroup }` carries the rule's `[rules]` knobs. `From<&AlignmentConfig>` builds the canonical settings, and `with_singleton_subgroup_strip` flips the singleton-collapse behavior on.
3. `AlignWalker { groups: Vec<Vec<Edit>>, rule: RuleId, settings: Settings, source: &'a Source }` is the carrier each rule's visitor struct wraps. `AlignWalker::new(source, settings, rule)` builds one with an empty `groups` accumulator, where each entry is one fix the pipeline maps to a single diagnostic. `emit_group(&mut self, members)` records a group's alignment edits, the `group_edits` / `push_group` pair lets a rule fold extra edits into a group before committing it, and `is_held(anchor)` reports whether a row's line is skip-suppressed for `rule`.

The entry point `emit_group(source: &Source, members: &[Member], settings: Settings, edits: &mut Vec<Edit>)` resolves the target column across `members`, falls back through `policy` *(`split` / `drop`)* when the widest member exceeds `max_shift`, and pushes one `Edit` per row that needs padding into the caller's accumulator. A singleton group collapses its gap to one space, or to zero when `settings.strip_singleton_subgroup` is set.

### Supporting Helpers

A consuming rule rarely hand-builds the walker from raw AST traversal, since `aligner.rs` exposes a set of `pub(crate)` helpers covering the common shapes a new alignment rule needs:

1. `line_adjacent_groups(items, member_of)` partitions `items` into runs of line-adjacent siblings via `Source::is_line_adjacent`, then maps each item through `member_of`. Single-member runs drop out.
2. `keyed_line_adjacent_groups(items, key_of, member_of)` is the same shape with a per-item key that further partitions adjacent items into sub-groups by key.
3. `parameter_split_groups(params, qualify)` walks a `Parameters` node and splits at the first parameter that does not qualify, used by rules over annotated function signatures.
4. `line_anchored_member(source, anchor)` builds a `Member` whose `gap` starts at `anchor` and whose `width` measures the leading display column on the line.
5. `line_anchored_member_at_kind(source, line, kind)` finds the first token of `kind` on `line` and anchors a `Member` at its end.
6. `range_anchored_member_single_line(source, range, anchor_of)` builds a `Member` whose `width` is the display-column width of `range`'s slice, for left-hand sides that are sub-ranges of one line.
7. `space_padding_edit(source, range, n)` produces a `Some(Edit)` replacing `range` with `n` spaces, or `None` when the current contents already match.
8. `is_alignment_candidate(members)` returns `true` when the group has at least two members and every member sits on a distinct line.

## How the Math Resolves

Aligners always carry a **one-space buffer** between content and the aligned token. The target column for a group is `max(member.width) + 1`, so every row whose existing column falls short of the target gets an `Edit` replacing its `gap` range with the right number of spaces, and rows already at the target stay unchanged without an edit.

When the widest member exceeds `max_shift`, the policy decides what happens next.

The `split` policy partitions the group into sub-groups of contiguous rows where the within-group widest is under `max_shift`, resolving each sub-group independently. The `drop` policy excludes the widest members from the padding calculation, leaving the group aligned to the widest non-overflow row.

A row carrying a line-level skip directive *(`# prose: skip`, `# fmt: skip`, or `# prose: skip[<rule>]`)* is **held** out of its group: excluded from the column math, emitting no edit, and transparent to the run so the rows on either side align as one block around it. The grouping treats a held row's own trailing skip comment as not breaking the run, while a standalone comment or blank line between rows still does. This is the same exclude-then-align shape as the `drop` policy, chosen by the author rather than by width.

Variable-width operators opt in to right-alignment by setting `op_width`, shifting each row's padding inward by `max(op_width) - row.op_width`. [[align-comparisons]] is the shipped consumer of this hook, with the infrastructure leaving the door open for future variable-width-operator rules to land as a grouping walker plus a knob set rather than a from-scratch implementation.

## Build Pattern

Each alignment rule wraps an `AlignWalker` in its visitor struct, walks the AST, collects `Vec<Member>` per group, and calls `walker.emit_group(&members)` once per group. The grouping shapes are rule-specific *(consecutive assignments, dict items, `import` keywords, match-arm patterns)*, because the per-rule definition of *"what counts as a group"* varies, but the math afterward is shared across every alignment rule.

A rule's `apply` method takes the canonical shape:

```rust
struct Visitor<'a> {
    walker: AlignWalker<'a>,
}

impl Rule for MyAlignmentRule {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let mut visitor = Visitor {
            walker: AlignWalker::new(source, self.settings, Self::SLUG),
        };
        visitor.visit_body(&source.ast().body);
        visitor.walker.groups
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

- [[align-colons]], [[align-comparisons]], [[align-equals]], [[align-imports]], and [[align-match-case]] are the consumers.
- [[colon-targets]] constructs `Member` lists from every `:` context, consumed by [[align-colons]] and [[strip-align-padding]].
- [[edit]] is the shape `emit_group` pushes into the caller's accumulator.
- [[orderer]] composes line-adjacency grouping differently *(by source-range block extents rather than `Member` widths)*, so a rule whose math is reorder-shaped rather than padding-shaped reaches for that primitive instead.

</template>

</PrimitiveLayout>
