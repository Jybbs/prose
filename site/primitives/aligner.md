---
consumedBy: [align-colons, align-comments, align-comparisons, align-equals, align-imports, align-match-case]
consumes: [edit, source]
layer: orchestration
stability: internal
summary: "Computes the padding each row of an alignment group needs and emits the edits that write it."
tagline: shared column math
---

# Aligner

<PrimitiveLayout primitive="aligner">

*Aligner* computes the padding each row of an alignment group needs and emits the edits that write it. Every alignment rule ([[align-colons]], [[align-comments]], [[align-comparisons]], [[align-equals]], [[align-imports]], [[align-match-case]]) reads the same column math from it, so each rule supplies its list of members and its facets, and *Aligner* resolves the column once.


## Public Surface

*Aligner* lives at `crate/src/primitives/aligner/` and is `pub(crate)`, so code inside the *Prose* crate can reach it and a downstream Rust caller cannot. What a downstream caller sees is the diagnostics the alignment rules emit, each carrying the resolved column in its `Edit`.

A downstream consumer can:

- Read aligned source after running `prose format` or `prose check`
- Read the resolved padding in the diagnostic `fix` payload of any alignment rule

A downstream consumer cannot construct a `Member`, call `emit_group`, or read `Settings`. The `1.0` line opens these types so a downstream can ship its own alignment rule against the same math.

## Internal Surface

The types every consumer touches are `Member`, `Settings`, and `AlignWalker`. `Member` describes one row in an alignment group:

| Field | Meaning |
|---|---|
| `baseline: usize` | The display column where the row's left-hand side begins |
| `gap: TextRange` | The whitespace range immediately before the aligned token, which the rule rewrites into padding |
| `line_start: TextSize` | The offset where the row's source line starts, which `is_alignment_candidate` reads to confirm each member sits on its own line |
| `op_width: usize` | The display width of a variable-width operator (*`==`, `!=`, `<=`*) that opts into right-alignment |
| `settled_width: usize` | `width` measured after an earlier column in the same pass has padded the content ahead of the gap, equal to `width` for a row no earlier column moves |
| `value_gap: Option<TextRange>` | The span from just past the operator to the value, which an aligned row rewrites to one space, `None` for a rule that leaves the post-operator spacing as written |
| `width: usize` | The display-column width from the member's start to the gap's start as the source is written |

The column math compares `settled_width` whereas the baseline reads `width`, which keeps a widened row on its group's baseline.

`Settings { buffer, cap, max_shift, release_heads, strip_singleton }` carries the rule's `[rules]` facets plus the governing length cap. `From<&AlignmentConfig>` builds the canonical settings, `with_buffer` widens the gap an aligned row keeps ahead of the token, and `within(line_length, stranding, settling)` supplies the cap the run resolves within, together with the padding rule and comment rules whose later edits each line is measured at. `releasing_heads` lets a group's head row stand down for the row its column cut off, and `with_singleton_strip` turns the singleton-collapse behavior on.

`AlignWalker { groups: Vec<Vec<Edit>>, rule: RuleId, source: &'a Source }` is the carrier each rule's visitor struct wraps, keeping its `Settings`, its placed-edit index, and the run's `Widenings` private. `AlignWalker::new(source, settings, rule)` builds one with an empty `groups` accumulator, where each entry is one fix the pipeline maps to a single diagnostic, and `set_widenings` installs the widening entries every later line-cap check reads. `emit_if_candidate(&mut self, members)` records a group's alignment edits together with a one-space rewrite of each member's `value_gap`, `emit_group_with_gaps(members, gaps)` folds caller-supplied gaps into the same fix, `emit_group_or_buffer(members)` falls back to the settings' buffer for a group that does not align, and `emit_unheld(members)` drops the rows a skip directive covers before emitting. `candidate_edits_under(settings, members)` and `column_or_buffer_edits(settings, members)` return the edits under a caller-supplied `Settings` for a rule folding them into a wider group through `push_group`, and `is_held(anchor)` reports whether a row's line is skip-suppressed for `rule`.

The entry point `emit_group(source: &Source, members: &[Member], settings: Settings, widenings: &Widenings, edits: &mut Vec<Edit>)` splits `members` into contiguous groups whose width spread stays within `max_shift`, resolves each group's column at its widest member, and pushes one `Edit` per row that needs padding into the caller's accumulator. A singleton group collapses its gap to the settings' buffer, or to zero when `settings.strip_singleton` is set.

### Supporting Helpers

A consuming rule rarely builds the walker from a raw AST traversal by hand, because the aligner module exposes a set of `pub(crate)` helpers covering the common cases a new alignment rule needs:

1. `line_adjacent_groups(source, body, rule, qualify)` partitions `body` into runs of line-adjacent siblings through `Source::consecutive_lines`, then maps each statement through `qualify`. A trailing comment sits inside its own row and leaves the run intact, whereas an own-line comment or a blank line closes it.
2. `keyed_line_adjacent_groups(source, body, rule, qualify)` does the same with a per-statement key that further partitions adjacent statements into sub-groups by key.
3. `parameter_split_groups(params, qualify)` reads through a `Parameters` node and splits at the first parameter that does not qualify, which the rules over annotated function signatures use.
4. `line_anchored_member(source, anchor)` builds a `Member` whose `gap` ends at `anchor` and whose `width` measures the leading display column on the line.
5. `line_anchored_member_at_kind(source, lhs_start, search, kind)` finds the first token of `kind` in `search` and anchors a `Member` on it, so the member's `gap` ends where that token starts.
6. `range_anchored_member_single_line(source, target, search, predicate, extra_width)` builds a `Member` whose `width` is the display-column width of `target`'s slice plus `extra_width`, for a left-hand side that is a sub-range of one line.
7. `line_gap_before(source, anchor)` returns the whitespace run ending at `anchor`, opening no earlier than that line's start. The member builders find a row's `gap` through it, and `normalize-comment-spacing` calls it directly to find the run ahead of a trailing comment.
8. `space_padding_edit(source, range, n)` returns `Some(Edit)` replacing `range` with `n` spaces, or `None` when the current contents already match.
9. `is_alignment_candidate(members)` returns `true` when the group has at least two members, each on a distinct line and opening at a shared column baseline, so the padding is written to a column every row can reach.

## How the Math Resolves

Every aligner keeps a **settings-supplied buffer** between content and the aligned token, one space for every rule but [[align-comments]], which places its `#` at the two-column trailing gap. The target column for a group is `max(member.width) + buffer`, so every row whose existing column falls short of the target gets an `Edit` replacing its `gap` range with the right number of spaces, and a row already at the target stays as written with no edit.

A run also resolves within its governing length cap, so a group grows only while every member's aligned line stays inside it. Each member is measured at the width of the line *Prose* will write rather than the line it read, so both spaces an aligned row carries around its operator count against the budget even when the source has neither. A member whose padded line would cross the cap is partitioned out of the run unpadded, whereas a member already past the cap before any padding stays in the run, because partitioning it out gains nothing.

A cut made for the cap can strand the row that caused it, where a group's opening row cannot follow the column a wider row beneath it sets while every row between them could. Under `release_heads`, that opening row is released as a singleton instead whenever the cut row would otherwise stand alone and the rows beneath the head form one group together with it, so a cap-wide lead line sits at its own width above a block rather than keeping that block's column down. [[align-equals]] sets `release_heads` because its rows reach their settled width under it. A rule whose rows a later alignment widens, the way [[align-equals]] pads the `=` of an annotated assignment after [[align-colons]] has placed its `:`, keeps the greedy cut instead, because the widened head would re-partition the run on the next pass.

When a run's width spread exceeds `max_shift`, the walker regroups it in source order.

`emit_group` reads each run from the first row, extending a group while its width spread stays within `max_shift` and starting a new group at the first row that would exceed it. Each group aligns to its widest member, and a row left alone keeps its minimal spacing, so a column never reaches past a narrow row to gather wider neighbors. `max_shift` reads as `false` to remove the cap so a contiguous run always aligns on one column, a positive `N` to bound the spread at `N`, and `0` to forbid any shift so every row sits flush.

A row carrying a skip directive *(`# prose: skip`, `# fmt: skip`, or `# prose: skip[<rule>]`)* is **held** out of its group, meaning the column math excludes it and it emits no edit. A directive trailing a wrapped statement covers every line that statement spans. A skipped single-line statement stays transparent to the run, so the rows on either side align as one block around it, whereas a skipped multi-line statement closes the run, because adjacency also requires the prior statement to fit on one source line. A skipped row's own trailing skip comment sits inside its row the way any trailing comment does and leaves the run intact. A standalone comment or blank line between rows still breaks it.

A variable-width operator opts in to right-alignment by setting `op_width`, which shifts each row's padding inward by `max(op_width) - row.op_width`. [[align-comparisons]] is the shipped consumer of this hook, and the hook lets a future variable-width-operator rule be written as a grouping walker plus a facet set rather than from scratch.

## Build Pattern

Each alignment rule wraps an `AlignWalker` in its visitor struct, visits the AST, collects a `Vec<Member>` per group, and calls `walker.emit_if_candidate(&members)` once per group. The grouping is rule-specific *(consecutive assignments, dict items, `import` keywords, match-arm patterns)*, because each rule defines *"what counts as a group"* differently, whereas the math afterward is shared across every alignment rule.

A rule's `apply` method takes the canonical form:

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

`line_adjacent_groups` handles the grouping for the common contiguous-statements case, with the per-item qualifier building each member through `line_anchored_member` or `line_anchored_member_at_kind` depending on whether the gap anchors at a known offset or at a specific token. `walker.emit_if_candidate(&members)` records each group's edits in the walker's `groups` accumulator, so the rule never threads a returned `Vec<Edit>` per group, and `apply` returns `visitor.walker.groups` at the end.

When the alignment context is a `:` *(dict items, annotated assignments, annotated parameters, docstring sections, match arms)*, the grouping logic lives in [[colon-targets]] instead. A new colon rule implements `ColonEmitter`'s required `rule` and `handle` methods plus the `docstring_entries` and `match_arms` overrides where the rule takes them, calls `walk(source)`, and forwards each yielded `&[aligner::Member]` slice to the walker for emission.

When the context is an `=` *(single-target assignments, exploded-call keyword arguments, annotated parameter defaults)*, the per-row member construction lives in `equal_targets`, which carries no walker because its consumers group differently. [[align-equals]] builds its runs with a multi-line break and calls `emit_group` to pad each `=`, whereas [[reflow-collections]] treats a rejoining value as single-line and reads `operator_columns` to predict where each `=` moves, testing that rejoin against the value's resulting column so the decision survives the alignment that runs later. A new `=` rule calls `equal_targets`'s `assignment` or `parameter` per row and groups the members by its own adjacency, or `keyword_groups` for an exploded call's pre-grouped keyword runs.

## Re-Using This Primitive

Writing a new alignment rule comes down to wrapping an `AlignWalker` in a visitor struct, writing the grouping logic that yields a `Vec<Member>` per source-line run, and calling `walker.emit_if_candidate(&members)` per group. The padding math, the reading-order regrouping, the singleton handling, and the right-alignment hook all come with the walker, leaving the rule to write only its own grouping logic.

<template #related>

- [[align-colons]], [[align-comments]], [[align-comparisons]], [[align-equals]], [[align-imports]], and [[align-match-case]] are the consumers.
- [[colon-targets]] constructs `Member` lists from every `:` context, consumed by [[align-colons]] and [[strip-stranded-padding]].
- [[edit]] is the type `emit_group` pushes into the caller's accumulator.
- [[orderer]] groups line-adjacent items differently *(by source-range block extents rather than `Member` widths)*, so a rule that reorders rather than pads uses that primitive instead.

</template>

</PrimitiveLayout>
