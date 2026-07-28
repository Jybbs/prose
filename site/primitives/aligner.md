---
consumedBy: [align-colons, align-comparisons, align-equals, align-imports, align-match-case]
consumes: [edit, source]
layer: orchestration
stability: internal
summary: "Computes padding widths and emits the alignment edits every alignment rule consumes."
tagline: shared alignment math
---

# Aligner

<PrimitiveLayout primitive="aligner">

*Aligner* computes padding widths and emits the alignment edits that every alignment rule consumes. The shipped consumers ([[align-colons]], [[align-comparisons]], [[align-equals]], [[align-imports]], [[align-match-case]]) share the same column-resolution math, so the math lives once in *Aligner* and each rule supplies a member list plus a facet-set rather than re-implementing the resolution from scratch.


## Public Surface

*Aligner* lives at `crate/src/primitives/aligner/` and is `pub(crate)`, so the type is reachable from inside the *Prose* crate but not from a downstream Rust caller today. The downstream-visible consequence is the diagnostic stream the alignment rules emit, with the resolved column landing in the `Edit` each rule produces.

A downstream consumer can:

- Observe aligned source after running `prose format` or `prose check`
- See the resolved padding in the diagnostic `fix` payload of any alignment rule

A downstream consumer cannot directly construct a `Member`, drive `emit_group`, or read `Settings`. The `1.0` line opens the surface so a downstream can ship its own alignment rule against the same math.

## Internal Surface

The types every consumer touches:

1. `Member { gap: TextRange, line_start: TextSize, op_width: usize, value_gap: Option<TextRange>, width: usize }` describes one row in an alignment group. `gap` is the whitespace range immediately before the aligned token, rewritten into padding. `line_start` is the offset of the source-line start, used by `is_alignment_candidate` to confirm each member sits on its own line. `op_width` is the display width of variable-width operators *(`==`, `!=`, `<=`)* opting into right-alignment. `value_gap` is the span from just past the operator to the value, which an aligned row rewrites to one space, left `None` by a rule that leaves the post-operator spacing alone. `width` is the display-column width from member start to gap start, which is what the math compares to find the target column.
2. `Settings { line_length, max_shift, strip_singleton }` carries the rule's `[rules]` facets plus the governing length cap. `From<&AlignmentConfig>` builds the canonical settings, `with_line_length` supplies the cap the run resolves within, and `with_singleton_strip` flips the singleton-collapse behavior on.
3. `AlignWalker { groups: Vec<Vec<Edit>>, rule: RuleId, source: &'a Source }` is the carrier each rule's visitor struct wraps, holding its `Settings` privately. `AlignWalker::new(source, settings, rule)` builds one with an empty `groups` accumulator, where each entry is one fix the pipeline maps to a single diagnostic. `emit_if_candidate(&mut self, members)` records a group's alignment edits together with a one-space rewrite of each member's `value_gap`, `emit_if_candidate_under(settings, members)` does the same under a caller-supplied `Settings` rather than the walker's own, the `group_edits` / `push_group` pair lets a rule fold extra edits into a group before committing it, and `is_held(anchor)` reports whether a row's line is skip-suppressed for `rule`.

The entry point `emit_group(source: &Source, members: &[Member], settings: Settings, edits: &mut Vec<Edit>)` splits `members` into contiguous groups whose width spread stays within `max_shift`, resolves each group's column at its widest member, and pushes one `Edit` per row that needs padding into the caller's accumulator. A singleton group collapses its gap to one space, or to zero when `settings.strip_singleton` is set.

### Supporting Helpers

A consuming rule rarely hand-builds the walker from raw AST traversal, since the aligner module exposes a set of `pub(crate)` helpers covering the common shapes a new alignment rule needs:

1. `line_adjacent_groups(source, body, rule, qualify)` partitions `body` into runs of line-adjacent siblings via `Source::consecutive_lines`, then maps each statement through `qualify`. A trailing comment sits inside its own row and leaves the run intact, whereas an own-line comment or a blank line closes it.
2. `keyed_line_adjacent_groups(source, body, rule, qualify)` is the same shape with a per-statement key that further partitions adjacent statements into sub-groups by key.
3. `parameter_split_groups(params, qualify)` walks a `Parameters` node and splits at the first parameter that does not qualify, used by rules over annotated function signatures.
4. `line_anchored_member(source, anchor)` builds a `Member` whose `gap` starts at `anchor` and whose `width` measures the leading display column on the line.
5. `line_anchored_member_at_kind(source, lhs_start, search, kind)` finds the first token of `kind` in `search` and anchors a `Member` at its end.
6. `range_anchored_member_single_line(source, target, search, predicate, extra_width)` builds a `Member` whose `width` is the display-column width of `target`'s slice plus `extra_width`, for left-hand sides that are sub-ranges of one line.
7. `space_padding_edit(source, range, n)` produces a `Some(Edit)` replacing `range` with `n` spaces, or `None` when the current contents already match.
8. `is_alignment_candidate(source, members)` returns `true` when the group has at least two members, each on a distinct line and opening at a shared column baseline, so the padding lands on a column every row can reach.

## How the Math Resolves

Aligners always carry a **one-space buffer** between content and the aligned token. The target column for a group is `max(member.width) + 1`, so every row whose existing column falls short of the target gets an `Edit` replacing its `gap` range with the right number of spaces, and rows already at the target stay unchanged without an edit.

A run also resolves within its governing length cap, so a group grows only while every member's aligned line stays inside it. The width each member is measured at is the line *Prose* will emit rather than the line it read, meaning both spaces an aligned row carries around its operator count against the budget even when the source holds neither. A member whose padded line would cross the cap partitions out of the run unpadded, and a member already past the cap before any padding stays in the run rather than partitioning to no gain.

When a run's width spread exceeds `max_shift`, the walk regroups it in source order.

`emit_group` walks each run from the first row, growing a group while its width spread stays within `max_shift` and breaking a fresh group at the first row that would exceed it. Each group aligns to its widest member, and a row left alone keeps its minimal spacing, so a column never reaches past a narrow row to gather wider neighbors. `max_shift` reads as `false` to lift the cap so a contiguous run always folds into one column, a positive `N` to bound the spread at `N`, and `0` to forbid any shift so every row sits flush.

A row carrying a skip directive *(`# prose: skip`, `# fmt: skip`, or `# prose: skip[<rule>]`)* is **held** out of its group, excluded from the column math, and emitting no edit. A directive trailing a wrapped statement holds every line that statement spans. A held single-line statement stays transparent to the run, so the rows on either side align as one block around it, whereas a held multi-line statement closes the run, because adjacency also requires the prior statement to fit on one source line. A held row's own trailing skip comment sits inside its row the way any trailing comment does, leaving the run intact, whereas a standalone comment or blank line between rows still breaks it.

Variable-width operators opt in to right-alignment by setting `op_width`, shifting each row's padding inward by `max(op_width) - row.op_width`. [[align-comparisons]] is the shipped consumer of this hook, with the infrastructure leaving the door open for future variable-width-operator rules to land as a grouping walker plus a facet set rather than a from-scratch implementation.

## Build Pattern

Each alignment rule wraps an `AlignWalker` in its visitor struct, walks the AST, collects `Vec<Member>` per group, and calls `walker.emit_if_candidate(&members)` once per group. The grouping shapes are rule-specific *(consecutive assignments, dict items, `import` keywords, match-arm patterns)*, because the per-rule definition of *"what counts as a group"* varies, but the math afterward is shared across every alignment rule.

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
        let source = self.walker.source;
        for members in line_adjacent_groups(source, body, self.walker.rule, |s| qualify(source, s)) {
            self.walker.emit_if_candidate(&members);
        }
    }
}
```

`line_adjacent_groups` handles the grouping for the common contiguous-statements shape, with the per-item qualifier folding through `line_anchored_member` or `line_anchored_member_at_kind` depending on whether the gap anchors at a known offset or at a specific token. `walker.emit_if_candidate(&members)` records each group's edits in the walker's `groups` accumulator, so the rule never has to thread a returned `Vec<Edit>` per group, and `apply` returns `visitor.walker.groups` at the end.

When the alignment context is `:`-shaped *(dict items, annotated assignments, annotated parameters, docstring sections, match arms)*, the grouping logic lives in [[colon-targets]] instead. A new colon-shaped rule implements `ColonEmitter`'s required `rule` and `handle` methods plus the `docstring_entries` and `match_arms` overrides where the rule takes them, calls `walk(source)`, and forwards each yielded `&[aligner::Member]` slice to the walker for emission.

When the context is `=`-shaped *(single-target assignments, exploded-call keyword arguments, annotated parameter defaults)*, the per-row member construction lives in `equal_targets`, which carries no walker because its consumers group differently. [[align-equals]] builds its runs with a multi-line break and calls `emit_group` to pad each `=`, whereas [[collection-layout]] treats a collapsing value as single-line and reads `operator_columns` to predict where each `=` shifts, testing its collapse against the value's resulting column so the decision survives the alignment that runs later. A new `=`-shaped rule calls `equal_targets`'s `assignment` or `parameter` per row and groups the members to its own adjacency, or `keyword_groups` for an exploded call's pre-grouped keyword runs.

## Re-Using This Primitive

Writing a new alignment rule comes down to wrapping an `AlignWalker` in a visitor struct, building the grouping logic that yields `Vec<Member>` per source-line run, and calling `walker.emit_if_candidate(&members)` per group. The padding math, the reading-order regrouping, the singleton handling, and the right-alignment hook all carry through, leaving the rule to focus on its own grouping logic.

<template #related>

- [[align-colons]], [[align-comparisons]], [[align-equals]], [[align-imports]], and [[align-match-case]] are the consumers.
- [[colon-targets]] constructs `Member` lists from every `:` context, consumed by [[align-colons]] and [[strip-align-padding]].
- [[edit]] is the shape `emit_group` pushes into the caller's accumulator.
- [[orderer]] composes line-adjacency grouping differently *(by source-range block extents rather than `Member` widths)*, so a rule whose math is reorder-shaped rather than padding-shaped reaches for that primitive instead.

</template>

</PrimitiveLayout>
