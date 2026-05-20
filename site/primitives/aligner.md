# Aligner

<DependencyGraph />

*Aligner* computes padding widths and emits the alignment edits that every alignment rule consumes. Four rules ([[align-equals]], [[align-colons]], [[align-imports]], [[match-case-align]]) share the same column-resolution math, so the math lives once in *Aligner* and each rule supplies a member list plus a knob-set rather than re-implementing the resolution from scratch.


## Public Surface (`0.2.x`)

*Aligner* lives at `src/primitives/aligner.rs` and is `pub(crate)`, so the type is reachable from inside the *prose* crate but not from a downstream Rust caller in `0.2.x`. The downstream-visible consequence is the diagnostic stream the alignment rules emit, with the resolved column landing in the `Edit` each rule produces.

A downstream consumer in `0.2.x` can:

- Observe aligned source after running `prose format` or `prose check`
- See the resolved padding in the diagnostic `fix` payload of any alignment rule

A downstream consumer in `0.2.x` cannot directly construct a `Member`, drive `emit_group`, or read `Settings`. The internal API stabilizes toward `1.0` where consumer-implemented alignment rules become reachable.

## Internal Surface

The two types every consumer touches:

- `Member { gap: TextRange, line_start: TextSize, op_width: usize, width: usize }` describes one row in an alignment group. `width` is the display-column width of the row's left-hand-side region from member start to gap start. `gap` is the whitespace range immediately before the aligned token, rewritten into padding. `op_width` is the display width of variable-width operators *(`==`, `!=`, `<=`)* that opt into right-alignment.
- `Settings { max_shift, policy, strip_singleton_subgroup }` carries the rule's `[tool.prose.rules.<rule>]` knobs. `From<&AlignmentConfig>` builds the canonical settings, and `with_singleton_subgroup_strip` flips the singleton-collapse behavior on.

The entry point `emit_group(source, members, settings) -> Vec<Edit>` resolves the target column across `members`, falls back through `policy` *(`split` / `drop` / `skip`)* when the widest member exceeds `max_shift`, and emits one edit per row that needs padding. A singleton group collapses its gap to one space, or to zero when `settings.strip_singleton_subgroup` is set.

## How the Math Resolves

Aligners always carry a **one-space buffer** between content and the aligned token. The target column for a group is `max(member.width) + 1`. Every row whose existing column falls short of the target gets an `Edit` replacing its `gap` range with the right number of spaces. Rows already at the target stay unchanged.

When the widest member exceeds `max_shift`, the policy decides what happens next.

The `split` policy partitions the group into sub-groups of contiguous rows where the within-group widest is under `max_shift`, resolving each sub-group independently. The `drop` policy excludes the widest members from the padding calculation, leaving the group aligned to the widest non-overflow row. The `skip` policy leaves the whole group unaligned, in that no edits emit at all.

Variable-width operators opt in to right-alignment by setting `op_width`, shifting each row's padding inward by `max(op_width) - row.op_width`. The hook is reserved for future comparison-alignment work *(no shipped rule consumes it at the current release)*, but the infrastructure is in place so that adding a comparison-alignment rule lands as a walker plus a knob set, not a from-scratch implementation.

## Build Pattern

Each alignment rule walks the AST, collects `Vec<Member>` per group, and calls `emit_group(source, &members, settings)`. The walker shapes are rule-specific *(consecutive assignments, dict items, `import` keywords, match-arm patterns)*, because the per-rule definition of *"what counts as a group"* varies. The math afterward is shared.

## Reuse Pattern

Adding a new alignment rule is shaped as *"write the walker that produces `Vec<Member>` groups, then call `emit_group`"*. The padding math, the policy fallbacks, the singleton handling, and the right-alignment hook all carry through without re-implementation.

## Related

- [[align-equals]], [[align-colons]], [[align-imports]], and [[match-case-align]] are the four consumers
- [[colon-targets]] constructs `Member` lists from every `:` context, consumed by [[align-colons]] and [[singleton-rule]]
- [[edit]] is the output shape `emit_group` returns
- [[source]] is the input every alignment walker reads against
