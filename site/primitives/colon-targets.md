---
consumedBy: [align-colons, strip-stranded-padding]
consumes: [aligner, docstring, source]
layer: analysis
stability: internal
summary: "Reads through every `:` context in a module and builds an alignment member for each."
tagline: colon context walker
---

# ColonTargets

<PrimitiveLayout primitive="colon-targets">

*ColonTargets* builds alignment members at every `:` context the alignment and singleton rules consume. The contexts in the Python grammar that carry a colon worth aligning are listed below, and the code that finds them is the same for every rule, so the walker lives once in *ColonTargets* and each consuming rule supplies a receiver that handles the members it finds.


## Public Surface

*ColonTargets* lives at `crate/src/primitives/colon_targets/` and is `pub(crate)`. The consumers today are [[align-colons]] *(which aligns multi-item groups in every context)* and [[strip-stranded-padding]] *(which removes pre-colon padding from a group that has no column to align to)*. What a downstream caller sees is the rewrites both rules emit through the diagnostic stream.

At `1.0` the trait becomes `pub`, so a downstream can implement a `:`-context rule of its own.

## The Contexts

1. **Dict items.** `{key: value, key: value}` literals, where each `key: value` pair contributes a member.
2. **Annotated assignments.** Statements of the form `target: Annotation = default` in any scope *(module, function, or class body)*, where each annotation colon contributes a member.
3. **Annotated function parameters.** `def f(param: T, param: T)` signatures, where each annotated parameter contributes a member.
4. **Docstring entry runs.** Every `Args:`, `Returns:`, `Raises:`, or other Title-case-headed section, plus every contiguous run of `name (type):` heads standing outside them, where each entry line contributes a `:` member, each entry naming a type contributes a second member anchored on its `(`, and each run aligns independently.
5. **Match-arm cases.** `match x: case Pattern: ...`, where each case's pattern-to-body colon contributes a member.

Each context resolves directly to an [[aligner]] `Member`, carrying its `width` *(the display-column width of the left-hand side)*, its `gap` *(the whitespace immediately before the colon)*, and its optional `value_gap` *(the post-colon span an aligned or stripped row rewrites to one space)*. Match arms and docstring entries carry no `value_gap`, leaving their post-colon spacing to [[align-match-case]] and to the source as written.

## Internal Surface

The receiver trait carries the per-context handlers, with `rule` and `handle` as the required methods, where `rule` names the consuming rule so the group builders can exclude its skip-suppressed rows from alignment, whereas `docstring_entries` and `match_arms` each carry a default a consuming rule overrides for context-specific handling:

```rust
pub(crate) trait ColonEmitter {
    fn docstring_entries(&mut self, run: &EntryColumns) {
        self.handle(run.colons());
    }

    fn handle(&mut self, members: &[aligner::Member]);

    fn match_arms(&mut self, members: &[aligner::Member]) {
        self.handle(members);
    }

    fn rule(&self) -> RuleId;

    fn walk(&mut self, source: &Source) where Self: Sized { /* provided */ }
}
```

`handle` is the catch-all for annotated assignments, dict entries, and parameters. `docstring_entries` and `match_arms` are split out so a rule can handle either context on its own terms, and each defaults to `handle` for a rule that reads every context through the one callback. [[align-colons]] overrides `match_arms` to a no-op, since [[align-match-case]] owns the match-arm context, and overrides `docstring_entries` to settle the run's type-group column ahead of its `:` column, both without the length cap its other contexts take.

`walk(source)` is the provided driver across `source`'s module body, recursing into nested classes, functions, matches, and expressions so a single call covers the whole tree. A consuming rule never overrides `walk`, because calling the provided method is enough to drive the receiver across every relevant context.

`match_case(source, case) -> Option<aligner::Member>` is exposed `pub(crate)` alongside the trait for [[align-match-case]], which builds members one match arm at a time rather than through the receiver. A new rule whose grouping follows contiguous lines should use the trait, whereas a rule that emits one member per construct should call `match_case` directly.

## Build Pattern

A rule implementing `ColonEmitter` carries a single accumulator *(typically `Vec<Vec<aligner::Member>>` for grouped members)* and pushes into it from each handler. After `walk(source)` returns, the accumulator carries every group the rule handles, and the rule emits `Vec<Edit>` by calling [[aligner]]'s `emit_if_candidate` against each group.

## How Grouping Works

Each context defines its own grouping, because what counts as *"adjacent"* inside a dict literal differs from what counts as *"adjacent"* across class-body statements:

1. **Dict items** group by line-adjacency between one key's end and the next item's start. A trailing comment stays with its entry and a `**spread` entry skips the colon scan, and neither breaks the run, so the rest of the dict aligns around them.
2. **Annotated assignments** group through `line_adjacent_groups` over each scope's statements, treating any statement that is not a `target: T` as a divider.
3. **Annotated function parameters** group through `parameter_split_groups`, splitting at the first parameter that does not qualify *(an un-annotated argument, a `*args` or `**kwargs`, a `/` or `*` separator)*.
4. **Match arms** group one per `match` statement, with every arm's colon contributing a member. A pattern may span multiple lines, so the alignment column is per `match` rather than per line run.
5. **Docstring entry runs** group one per Title-case-headed section and one per contiguous run of type-bearing heads outside them, with the structured-section parser called inline to find each run's entries, so one run's entries align without reaching across the break that ends it.

Each group is handed to the receiver as one `&[aligner::Member]` slice, so the consumer aligns within the group without seeing cross-group state. The docstring-args context borrows [[docstring]]'s `body_docstring` to find a body's leading docstring literal, then runs its own line scan for each entry's `:` position, because the two primitives return different things. [[docstring]] yields entry names with the byte range a reorder carries along, whereas the colon walker yields each line's colon anchor for the aligner's padding math.

## Re-Using This Primitive

A new `:`-context rule implements `ColonEmitter`, overrides the handlers for the contexts it covers, and calls `walk(source)` from inside its `apply` method. The shared walker, the same-indentation grouping, and the per-context member construction come with the trait.

<template #related>

- [[aligner]] is the math the produced `Member` lists feed into.
- [[align-colons]] aligns multi-item groups across every context.
- [[strip-stranded-padding]] removes padding from singleton groups.
- [[align-match-case]] owns the match-arm context exclusively.
- The `=`-context sibling builds its members in `equal_targets`, described under [[aligner]].

</template>

</PrimitiveLayout>
