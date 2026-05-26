# ColonTargets

<PrimitiveLayout primitive="colon-targets">

*ColonTargets* constructs alignment members at every `:` context the alignment and singleton rules consume. The distinct contexts in the Python grammar that carry a colon worth aligning are listed below, and the walker that finds them is identical across rules, so the walker lives once in *ColonTargets* and each consuming rule supplies a receiver that handles the discovered members.


## Public Surface

*ColonTargets* lives at `src/primitives/colon_targets.rs` and is `pub(crate)`. Two consumers use it today: [[align-colons]] *(which aligns multi-item groups in every context)* and [[singleton-rule]] *(which strips pre-colon padding from groups that have no column to align to)*. The downstream-visible consequence is the rewrites both rules emit through the diagnostic stream.

At `1.0` the trait promotes to `pub`, so a downstream can implement a `:`-context rule of its own.

## The Contexts

1. **Dict items.** `{key: value, key: value}` literals, where each `key: value` pair contributes a member.
2. **Class fields.** Class bodies whose statements are `field: Annotation = default`, where each field's annotation colon contributes a member.
3. **Annotated function parameters.** `def f(param: T, param: T)` signatures, where each annotated parameter contributes a member.
4. **Google / numpy docstring `Args:` entries.** Docstring structured sections, where each `name: description` line contributes a member.
5. **Match-arm cases.** `match x: case Pattern: ...`, where each case's pattern-to-body colon contributes a member.

Each context resolves a `Member` for the alignment math, with `width` being the display-column width of the left-hand side and `gap` being the whitespace range immediately before the colon.

## Internal Surface

The receiver trait carries the per-context handlers, where only `handle` is required with an empty default body and the other three are provided methods a consuming rule overrides on a need-by-need basis:

```rust
pub(crate) trait ColonEmitter {
    fn dict(&mut self, d: &ExprDict, members: &[aligner::Member]) {
        self.handle(members);
    }

    fn handle(&mut self, members: &[aligner::Member]) {}

    fn match_arms(&mut self, members: &[aligner::Member]) {
        self.handle(members);
    }

    fn walk(&mut self, source: &Source) where Self: Sized { /* provided */ }
}
```

`handle` is the catch-all for class fields, docstring args, and parameters. `dict` carries the surrounding `ExprDict` for consumers that need its range *(e.g., the `# prose: keep` suppression check on dict literals)*, with the default delegating to `handle` so a rule that does not care about the surrounding dict gets the same callback. `match_arms` is split out so a rule can opt out of match-arm alignment by overriding it to a no-op *(which is what [[align-colons]] does, since [[match-case-align]] owns the match-arm context)*, with its default also delegating to `handle` for any rule that wants the unified callback.

`walk(source)` is the provided driver across `source`'s module body, recursing into nested classes, functions, matches, and expressions so a single call covers the whole tree. A consuming rule never overrides `walk`, because calling the provided method is enough to drive the receiver across every relevant context.

`match_case(source, case) -> Option<aligner::Member>` is exposed `pub(crate)` alongside the trait for [[match-case-align]], which builds members one match arm at a time rather than through the receiver shape. New rules whose grouping logic is contiguous-line-shaped should reach for the trait, whereas rules that emit one-member-per-construct should reach for `match_case` directly.

## Build Pattern

A rule implementing `ColonEmitter` carries a single accumulator *(typically `Vec<Vec<Member>>` for grouped members)* and pushes into it from each handler. After `walk(source)` returns, the accumulator carries every group the rule cares about, and the rule emits `Vec<Edit>` by calling [[aligner]]'s `emit_group` against each group.

## How Grouping Works

Each context defines its own grouping shape, because what counts as *"adjacent"* inside a dict literal differs from what counts as *"adjacent"* across class-body statements:

1. **Dict items** group by line-adjacency between one key's end and the next item's start. A `**spread` entry skips the colon scan without breaking the run, so the rest of the dict aligns around the spreads.
2. **Class fields** group via `line_adjacent_groups` over the class body's statements, treating any non-`field: T` statement as a divider.
3. **Annotated function parameters** group via `parameter_split_groups`, splitting at the first parameter that does not qualify *(an un-annotated argument, a `*args` or `**kwargs`, a `/` or `*` separator)*.
4. **Match arms** group one per `match` statement, with every arm's colon contributing a member. Patterns may span multiple lines, so the alignment column is per-`match` rather than per-line-run.
5. **Docstring `Args:` entries** group one per docstring, with the structured-section parser invoked inline to find the entries.

Each group is handed to the receiver as one `&[Member]` slice, so the consumer aligns within the group without seeing cross-group state. The seam at the docstring-args context overlaps with [[docstring]]'s walker, where the leading-docstring detection lives in both primitives independently. The duplication is deliberate, because the colon walker reaches for the docstring body without standing up a separate `DocstringHandler`.

## Re-Using This Primitive

A new `:`-context rule implements `ColonEmitter`, overrides the handlers for the contexts it cares about, and calls `walk(source)` from inside its `apply` method. The shared walker, the same-indentation grouping, and the per-context member construction come for free.

<template #related>

- [[aligner]] is the math the produced `Member` lists feed into.
- [[align-colons]] aligns multi-item groups across every context.
- [[singleton-rule]] strips padding from singleton groups.
- [[match-case-align]] owns the match-arm context exclusively.

</template>

</PrimitiveLayout>
