---
stability: internal
---

# Edit

<PrimitiveLayout primitive="edit">

*Edit* is the unit every rule emits and the [[pipeline]] applies. A rule's `apply(&Source) -> Vec<Vec<Edit>>` method returns its replacement spans partitioned into fix groups, the pipeline maps each group to one diagnostic and splices their edits into a fresh buffer between rules, and the rewritten source feeds the next rule. *Prose* re-exports the upstream `ruff_diagnostics::Edit` type rather than defining its own, so the shape matches what Ruff and other Astral-stack consumers expect.


## Public Surface

`Edit` itself is `pub` *(re-exported from `ruff_diagnostics`)*, and the `Diagnostic` type a rule emits through the pipeline carries an `Option<Vec<Edit>>` in its `fix` field, visible in every [**output format**](/reference/output-formats) the CLI emits *(json, github, sarif)*. A downstream consumer reading the json output sees every edit's range and content in the `fix.edits[]` array.

The edit-shaping helpers *(`apply_edits`, `apply_inline_edits`, `narrow_edit`)* live at `src/primitives/edit.rs` and are `pub(crate)`. The helpers move to `pub` at `1.0` alongside the `Rule` trait, so a downstream rule can splice edits into source the same way the bundled rules do.

## The Shape

Each `Edit` carries:

- A `range: TextRange` covering the source span the edit replaces. `TextRange` is byte-indexed *(through `ruff_text_size`)*, so offsets count UTF-8 bytes rather than Unicode scalars or display columns.
- A `content: Option<String>` carrying the replacement text *(or `None` for a pure deletion)*

A zero-length range with non-empty content is an insertion. A non-empty range with empty content is a deletion. A non-empty range with non-empty content is a substitution. The three shapes compose, so a rule rewriting one logical change as several local edits emits each as its own `Edit` without coordinating.

Edits span newlines freely, so a rule rewriting a multi-line construct emits one `Edit` whose `range` covers the whole construct and whose `content` carries the rewritten body. Line-ending style in `content` follows the rule's emission, and the pipeline does not normalize, so a rule on a CRLF source should emit CRLF in any newline it inserts. The [[source]] primitive exposes `newline_str()` for the per-file convention.

## Internal Surface

Three helpers at `src/primitives/edit.rs` cover the common shaping needs.

### `apply_edits(text, edits) -> String`

Splices a sorted edit list into a source string, serving as the [[pipeline]]'s transform between rules. Linear in source length regardless of edit count, since the function walks the list once. Debug builds assert the sorted edits are non-overlapping, a rule-authoring invariant.

### `apply_inline_edits(source, range, edits) -> Cow<'src, str>`

Folds a list of edits into a source range, returning `Cow::Borrowed` when no edit applies. Used by [[orderer]] when rendering each block's text, wherein blocks that don't themselves rewrite can reference the source slice directly.

### `narrow_edit(source, range, content) -> Edit`

Trims a candidate replacement to its minimal divergent range against the source. A rule producing `"foo = 1\n"` to replace `"foo = 1\n"` emits no edit at all, whereas a rule producing `"foo   = 1\n"` emits an edit covering only the `   ` insertion. Minimal-range edits are cheaper to apply and surface cleaner diffs.

## Conflict Discipline

The [[pipeline]] applies each rule's edits sequentially, reparsing between rules so the next rule reads against a settled AST. Two rules emitting edits to overlapping ranges within the same pass would conflict, but the pipeline structure prevents this, because each rule sees the rewritten source from previous rules and the second rule's edits land against the first rule's output rather than against the original.

Within one rule, debug assertions catch overlapping edits at `apply_edits` time. Release builds skip the assertion, which means overlapping edits in release silently produce wrong output, so the debug-build check is the load-bearing gate authors rely on. Test every new rule under debug builds against fixture sources that exercise the rule's edge cases before shipping, since an overlap that escapes the test suite will mis-format real user code. A rule that produces overlapping edits is a rule-authoring bug, caught early via the debug assertion and never expected to ship.

## Build Pattern

A rule emits `Vec<Vec<Edit>>` from its `apply(&Source)` method, each inner vector one fix group, with each edit's range naming the source span to rewrite and its content carrying the replacement. The pipeline handles sorting, splicing, and reparsing on the rule's behalf.

A rule that computes one logical change as several edits *(an alignment rule padding several rows)* returns them as a single group, so the pipeline maps the whole change to one diagnostic and sorts every group's edits together at apply time. Rules whose edits are mutually independent return one single-edit group per edit.

## Re-Using This Primitive

Every rule reaches for `Edit` to express a rewrite. The [[aligner]] primitive emits `Edit` lists for padding rewrites. The [[orderer]] primitive composes through `apply_inline_edits` when rendering reordered blocks. The [[docstring]] walker hands ranges to rules that emit `Edit` against docstring bodies.

<template #related>

- [[pipeline]] is the consumer that applies edit lists between rules.
- [[aligner]], [[orderer]], and [[docstring]] all produce `Edit` lists.
- [[binding-analysis]] answers the offset-and-scope questions binding-aware rules consult before shaping their edits.
- [[suppression-map]] filters edits at the emission boundary before the pipeline applies them.

</template>

</PrimitiveLayout>
