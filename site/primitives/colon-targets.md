# ColonTargets

<PrimitivesComposition :initial-focus="'colon-targets'" />

*ColonTargets* constructs alignment members at every `:` context the alignment and singleton rules consume. **Five** distinct contexts in the Python grammar carry a colon worth aligning, and the walker that finds them is identical across rules, so the walker lives once in *ColonTargets* and each consuming rule supplies a receiver that handles the discovered members.


## Public Surface (`0.2.x`)

*ColonTargets* lives at `src/primitives/colon_targets.rs` and is `pub(crate)`. Two consumers use it today: [[align-colons]] *(which aligns multi-item groups in every context)* and [[singleton-rule]] *(which strips pre-colon padding from groups that have no column to align to)*. The downstream-visible consequence is the rewrites both rules emit through the diagnostic stream.

The internal API stabilizes toward `1.0` where consumer-implemented colon-shaped rules become reachable.

## The Five Contexts

1. **Dict items.** `{key: value, key: value}` literals, where each `key: value` pair contributes a member
2. **Pydantic-style class fields.** Class bodies whose statements are `field: Annotation = default`, where each field's annotation colon contributes a member
3. **Annotated function parameters.** `def f(param: T, param: T)` signatures, where each annotated parameter contributes a member
4. **Google / numpy docstring `Args:` entries.** Inside docstring structured sections, each `name: description` line contributes a member
5. **Match-arm cases.** `match x: case Pattern: ...`, where each case's pattern-to-body colon contributes a member

Each context resolves a `Member` for the alignment math, with `width` being the display-column width of the left-hand side and `gap` being the whitespace range immediately before the colon.

## Internal Surface

The receiver trait carries the per-context handlers:

```rust
pub(crate) trait ColonEmitter {
    fn dict(&mut self, d: &ExprDict, members: &[aligner::Member]);
    fn handle(&mut self, members: &[aligner::Member]);
    fn match_arms(&mut self, members: &[aligner::Member]);
    fn walk(&mut self, source: &Source);
}
```

`handle` is the catch-all for class fields, docstring args, and parameters. `dict` carries the surrounding `ExprDict` for consumers that need its range *(e.g., the `# prose: keep` suppression check on dict literals)*. `match_arms` is split out so a rule can opt out of match-arm alignment by overriding it to a no-op *(which is what [[align-colons]] does, since [[match-case-align]] owns the match-arm context)*.

`walk(source)` drives the receiver across `source`'s module body, recursing into nested classes, functions, matches, and expressions so a single call covers the whole tree.

## Build Pattern

A rule implementing `ColonEmitter` carries a single accumulator *(typically `Vec<Vec<Member>>` for grouped members)* and pushes into it from each handler. After `walk(source)` returns, the accumulator carries every group the rule cares about, and the rule emits `Vec<Edit>` by calling [[aligner]]'s `emit_group` against each group.

## How Grouping Works

Within each context, *ColonTargets* groups members by **same-indentation contiguous lines**. A blank line, a comment-only line, or a change in indentation breaks the group. Each group is handed to the receiver as one `&[Member]` slice, so the consumer aligns within the group without seeing cross-group state.

## Reuse Pattern

Adding a colon-shaped rule is shaped as *"implement `ColonEmitter`, override the contexts the rule cares about, call `walk(source)` from inside the rule's `apply` method"*. The five-context walker, the same-indentation grouping, and the per-context member construction all carry through without re-implementation.

## Related

- [[aligner]] is the math the produced `Member` lists feed into
- [[align-colons]] aligns multi-item groups across every context
- [[singleton-rule]] strips padding from singleton groups
- [[match-case-align]] owns the match-arm context exclusively
- [[source]] is the input the walker reads against
