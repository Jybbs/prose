# Edit

<PrimitivesComposition :initial-focus="'edit'" />

*Edit* is the unit every rule emits and the [[pipeline]] applies. A rule's `apply(&Source) -> Vec<Edit>` method returns a list of replacement spans, the pipeline splices them into a fresh buffer between rules, and the rewritten source feeds the next rule. *Prose* re-exports the upstream `ruff_diagnostics::Edit` type rather than defining its own, so the shape matches what Ruff and other Astral-stack consumers expect.


## Public Surface

`Edit` itself is `pub` *(re-exported from `ruff_diagnostics`)*, and the `Diagnostic` type a rule emits through the pipeline carries an `Option<Edit>` in its `fix` field, visible in every [**output format**](/reference/output-formats) the CLI emits *(json, github, sarif)*. A downstream consumer reading the json output sees the edit's range and content in the `fix.edits[]` array.

The edit-shaping helpers *(`apply_edits`, `apply_inline_edits`, `narrow_edit`)* live at `src/primitives/edit.rs` and are `pub(crate)`. They stabilize toward `1.0` where consumer-implemented rules become reachable.

## The Shape

Each `Edit` carries:

- A `range: TextRange` covering the source span the edit replaces
- A `content: Option<String>` carrying the replacement text *(or `None` for a pure deletion)*

A zero-length range with non-empty content is an insertion. A non-empty range with empty content is a deletion. A non-empty range with non-empty content is a substitution. The three shapes compose, so a rule rewriting one logical change as several local edits emits each as its own `Edit` without coordinating.

## Internal Surface

Three helpers at `src/primitives/edit.rs` cover the common shaping needs.

### `apply_edits(text, edits) -> String`

Splices a sorted edit list into a source string, serving as the [[pipeline]]'s transform between rules. Linear in source length regardless of edit count, in that the function walks the list once. Debug builds assert the sorted edits are non-overlapping, a rule-authoring invariant.

### `apply_inline_edits(source, range, edits) -> Cow<'src, str>`

Folds a list of edits into a source range, returning `Cow::Borrowed` when no edit applies. Used by [[orderer]] when rendering each block's text, wherein blocks that don't themselves rewrite can reference the source slice directly.

### `narrow_edit(source, range, content) -> Edit`

Trims a candidate replacement to its minimal divergent range against the source. A rule producing `"foo = 1\n"` to replace `"foo = 1\n"` emits no edit at all, whereas a rule producing `"foo   = 1\n"` emits an edit covering only the `   ` insertion. Minimal-range edits are cheaper to apply and surface cleaner diffs.

## Conflict Discipline

The [[pipeline]] applies each rule's edits sequentially, reparsing between rules so the next rule reads against a settled AST. Two rules emitting edits to overlapping ranges within the same pass would conflict, but the pipeline structure prevents this, in that each rule sees the rewritten source from previous rules and the second rule's edits land against the first rule's output rather than against the original.

Within one rule, debug assertions catch overlapping edits at `apply_edits` time. Release builds skip the assertion, leaving overlapping edits to silently corrupt the spliced output, so the debug-build check is the load-bearing gate authors rely on. A rule that produces overlapping edits is a rule-authoring bug, caught early via the debug assertion and never expected to ship.

## Build Pattern

A rule emits `Vec<Edit>` from its `apply(&Source)` method. Each edit's range names the source span to rewrite, with the content carrying the replacement. The pipeline handles sorting, splicing, and reparsing.

For rules that compute multiple edits per logical change *(an alignment rule that pads several rows in one group)*, each edit is emitted independently. The pipeline sorts them at apply time.

## Re-Using This Primitive

Every rule reaches for `Edit` to express a rewrite. The [[aligner]] primitive emits `Edit` lists for padding rewrites. The [[orderer]] primitive composes through `apply_inline_edits` when rendering reordered blocks. The [[docstring]] walker hands ranges to rules that emit `Edit` against docstring bodies.

## Related

- [[pipeline]] is the consumer that applies edit lists between rules
- [[aligner]], [[orderer]], and [[docstring]] all produce `Edit` lists
- [[source]] is the input every edit's range names a span inside
- [[suppression-map]] filters edits at the emission boundary before the pipeline applies them
