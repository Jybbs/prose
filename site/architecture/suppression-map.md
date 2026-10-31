---
consumedBy: [alphabetize-siblings, band-constants, inert-directives, line-overflow, modernize-annotations, pipeline, reflow-imports]
consumes: [rule-id, source]
layer: analysis
stability: internal
summary: "Per-source index of every `# fmt: off`, `# fmt: skip`, and `# prose: ignore[...]` directive, read before an edit or a lint is emitted."
tagline: directive index
---

# The `SuppressionMap` Type

<PrimitiveLayout primitive="suppression-map">

*SuppressionMap* is the per-source index of every suppression directive a file declares. Every source file gets one scan for directives during [[source]] construction, and the map is the result. It records the format-suppression spans *(`# fmt: off` / `# fmt: on`, `# prose: off` / `# prose: on`, and the `# yapf: disable` / `# yapf: enable` aliases)*, the statement-level format markers *(`# fmt: skip` and its `# prose: skip` alias)*, the per-rule format directives *(`# prose: skip[<rule>]`)*, and the per-line lint directives *(`# prose: ignore` and `# prose: ignore[<rule>]`)*. The [[pipeline]] reads the map at the point edits are emitted and drops a suppressed fix group or lint diagnostic before it reaches the caller.

## Public Surface

The *SuppressionMap* type itself is `pub(crate)`, so neither the type nor its methods are reachable from a downstream Rust consumer. The suppression behavior is reachable through [**`Pipeline::run`**](/architecture/pipeline), which filters emitted edits and lint diagnostics against the map.

A downstream consumer works with suppression through the directives a source file declares inline (*`# fmt: off`, `# fmt: skip`, `# prose: skip[<rule>]`, `# prose: ignore[<rule>]`*), and `run` drops the diagnostics and edits a directive covers from the returned vectors with no notice, whereas [[inert-directives]] reports a `skip` or `ignore` directive that holds nothing out of the run.

## Internal Surface

For code inside the *Prose* crate, the map exposes a constructor and a set of predicates:

1. `from_comments(source, comments, tokens, first_code_offset, cell_offsets) -> Self` builds the map by scanning the comment ranges for the directive forms the map recognizes. `tokens` resolves the logical line each skip directive covers, `first_code_offset` is what `file_is_suppressed` compares an unmatched opener against, and `cell_offsets` closes an unmatched opener at the end of its notebook cell.
2. `file_is_suppressed() -> bool` returns true when an unmatched `# prose: off` *(or `# fmt: off`)* sits at or before the first non-blank, non-comment line of the file, so the pipeline returns the source unchanged before any rule runs.
3. `has_format_suppression() -> bool` reports whether any `# prose: off` region, bare `# prose: skip` span, or `# prose: skip[<rule>]` directive sits in the file.
4. `has_lint_suppression() -> bool` reports the same for `# prose: ignore` directives.
5. `ignore_at(line: OneIndexed) -> Option<&Directive>` returns the `# prose: ignore` directive on the line, or `None` where the line carries none.
6. `ignores() -> &[(OneIndexed, Directive)]` lists every `# prose: ignore` directive beside the line it sits on, in source order.
7. `intersects<R: Ranged>(ranged: R) -> bool` returns true when the given range overlaps a `# prose: off` region, which is the check the lint filter reads, so a bare `# prose: skip` opens nothing here and a lint diagnostic on its statement survives.
8. `is_lint_suppressed_at(line: OneIndexed, rule: RuleId) -> bool` returns true when the line carries a `# prose: ignore` directive that names the rule, or a bare directive that covers every rule.
9. `pins<R: Ranged>(ranged: R, rule: RuleId) -> bool` returns true when a reorder by the rule leaves the range in place, meaning the range overlaps a span `suppresses` reports for the rule or holds the line of a `# prose: on` that closes a region.
10. `skipped<R: Ranged>(ranged: R) -> bool` returns true when the span of a skip directive overlaps the range, whatever rules it names, which is the check the lint filter reads before closing a finding's message on the `# prose: ignore[<rule>]` that silences it.
11. `skips() -> &[(TextRange, Directive)]` lists every skip directive beside the span it suppresses, in source order.
12. `suppresses<R: Ranged>(ranged: R, rule: RuleId) -> bool` returns true when the given range overlaps a `# prose: off` region, a bare `# prose: skip` span, or a `# prose: skip[<rule>]` span naming the rule, which is the check the rewrite filter reads.

Each directive the map records is a `Directive`, holding the comment that carries it and the rules its chunks name, beside every bracketed id among them that names no registered rule, as written. Its `skip_removal(source_text: &str) -> Vec<TextRange>` returns the ranges whose deletion removes the directive's skip chunks, taking the whole comment and the whitespace before it where the comment holds nothing else, so the [[inert-directives]] audit runs a rule over a copy of the source with that one directive gone.

### Directive Recognition

The directives that feed the map share a grammar the [**Suppression Directives**](/reference/suppression-directives) reference covers, and the forms the map records are these:

1. `# fmt: off` opens a format-suppression span and `# fmt: on` closes it. A span with no closer runs to the end of the file, or to the end of its notebook cell. Nested or overlapping `# fmt: off` markers flatten, so the first `# fmt: on` after any number of `off` markers closes the span.
2. `# prose: off` and `# prose: on` use the same span machinery, so a project picks whichever prefix reads better. When `# prose: off` sits at or before the first non-blank, non-comment line of the file and no `# prose: on` follows, the map sets `file_is_suppressed`.
3. `# yapf: disable` and `# yapf: enable` are recognized as aliases for `# fmt: off` and `# fmt: on`, so a project's existing yapf markers keep working. Other yapf directives are not recognized.
4. `# fmt: skip` at the end of a line exempts that one logical line from rewrites, scoped to that statement, and leaves its lint diagnostics to report. `# prose: skip` is the equivalent alias, and either one on a comment carrying a `# prose: skip[<rule>]` widens it to every rule.
5. `# prose: skip[<rule>, <rule>, …]` exempts the same logical line `# fmt: skip` covers from the rewrites of the listed rules alone, with whitespace inside the brackets tolerated and two bracketed directives on one line unioning their rule sets. An unknown rule slug suppresses nothing and stays on the directive as written, where [[inert-directives]] reports it.
6. `# prose: ignore[<rule>, <rule>, …]` exempts the listed lints on the directive's line, with the same bracket-whitespace tolerance and union behavior. A bare `# prose: ignore` covers every lint on the line.

## Re-Using This Primitive

The [[pipeline]] is the canonical consumer, applying the filter at the point edits are emitted, so a new rule emits its edits unconditionally and the pipeline drops the suppressed groups. A rule whose fix group would otherwise span an exempt row reads `suppresses` itself before grouping, the way the alignment rules exclude a skip-exempt row through the aligner's `is_held` before a column is resolved. [[alphabetize-siblings]], [[band-constants]], and [[modernize-annotations]] read it the same way, and [[inert-directives]] reads the directives themselves, judging each against the run. [[line-overflow]] reads it to report a row whose split a skip withholds from its layout rule, and [[reflow-imports]] reads `pins` for the statements a reorder by [[alphabetize-siblings]] leaves in place as it forecasts that sort. The map is built once per source and handed to every reader by reference.

<template #related>

- The [**Suppression Directives**](/reference/suppression-directives) reference covers the directives the map records, with the syntax for block markers, line markers, and lint directives, and the [**Suppression**](/guide/suppression) chapter covers choosing one for a scope.
- [[source]] builds the map during construction and exposes it through `suppression_map()`.
- [[pipeline]] reads the map at the point edits are emitted, dropping suppressed entries before they reach the caller.
- [[rule-id]] is the handle the bracketed directives name inside the `# prose: skip[<slug>]` and `# prose: ignore[<slug>]` syntax.

</template>

</PrimitiveLayout>
