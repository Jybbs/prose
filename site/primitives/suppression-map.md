---
consumedBy: [pipeline]
consumes: [rule-id, source]
layer: analysis
stability: internal
summary: "Per-source index of every `# fmt: off`, `# fmt: skip`, and `# prose: ignore[...]` directive, read before an edit or a lint is emitted."
tagline: directive index
---

# SuppressionMap

<PrimitiveLayout primitive="suppression-map">

*SuppressionMap* is the per-source index of every suppression directive a file declares. Every source file gets one scan for directives during [[source]] construction, and the map is the result. It records the format-suppression spans *(`# fmt: off` / `# fmt: on`, `# prose: off` / `# prose: on`, and the `# yapf: disable` / `# yapf: enable` aliases)*, the statement-level format markers *(`# fmt: skip` and its `# prose: skip` alias)*, the per-rule format directives *(`# prose: skip[<rule>]`)*, and the per-line lint directives *(`# prose: ignore` and `# prose: ignore[<rule>]`)*. The [[pipeline]] reads the map at the point edits are emitted and drops a suppressed fix group or lint diagnostic before it reaches the caller.

## Public Surface

The *SuppressionMap* type itself is `pub(crate)` today, so neither the type nor its methods are reachable from a downstream Rust consumer. The suppression behavior is reachable through [**`Pipeline::run`**](/primitives/pipeline), which filters emitted edits and lint diagnostics against the map.

A downstream consumer works with suppression through the directives in the source files:

- A source file declares its directives inline (*`# fmt: off`, `# fmt: skip`, `# prose: skip[<rule>]`, `# prose: ignore[<rule>]`*).
- The [[pipeline]] reads the directives during `run`.
- Diagnostics and edits a directive covers are dropped from the returned vectors, with no notice.

The type opens toward `1.0`, when the lookup methods become public so downstream tooling can inspect suppression spans (*IDE highlighting of suppressed ranges, lint-coverage reports, suppression audits*).

## Internal Surface

For code inside the *Prose* crate, the map exposes a constructor and a set of predicates:

1. `from_comments(source, comments, tokens, first_code_offset, cell_offsets) -> Self` builds the map by scanning the comment ranges for the directive forms the map recognizes. `tokens` resolves the logical line each skip directive covers, `first_code_offset` is what `file_is_suppressed` compares an unmatched opener against, and `cell_offsets` closes an unmatched opener at the end of its notebook cell.
2. `file_is_suppressed() -> bool` returns true when an unmatched `# prose: off` *(or `# fmt: off`)* sits at or before the first non-blank, non-comment line of the file, so the pipeline returns the source unchanged before any rule runs.
3. `has_format_suppression() -> bool` reports whether any `# prose: off` region, bare `# prose: skip` span, or `# prose: skip[<rule>]` directive sits in the file.
4. `has_lint_suppression() -> bool` reports the same for `# prose: ignore` directives.
5. `intersects<R: Ranged>(ranged: R) -> bool` returns true when the given range overlaps a `# prose: off` region, which is the check the lint filter reads, so a bare `# prose: skip` opens nothing here and a lint diagnostic on its statement survives.
6. `is_lint_suppressed_at(line: OneIndexed, rule: RuleId) -> bool` returns true when the line carries a `# prose: ignore` directive that names the rule, or a bare directive that covers every rule.
7. `suppresses<R: Ranged>(ranged: R, rule: RuleId) -> bool` returns true when the given range overlaps a `# prose: off` region, a bare `# prose: skip` span, or a `# prose: skip[<rule>]` span naming the rule, which is the check the rewrite filter reads.

The [[source]] accessor `suppression_map(&self) -> &SuppressionMap` is also `pub(crate)`. Every entry point above opens at `1.0`.

### Directive Recognition

The directives that feed the map share a grammar the [**Suppression**](/usage/suppression) chapter covers, and the forms the map records are these:

1. `# fmt: off` opens a format-suppression span and `# fmt: on` closes it. A span with no closer runs to the end of the file, or to the end of its notebook cell. Nested or overlapping `# fmt: off` markers flatten, so the first `# fmt: on` after any number of `off` markers closes the span.
2. `# prose: off` and `# prose: on` use the same span machinery, so a project picks whichever prefix reads better. When `# prose: off` sits at or before the first non-blank, non-comment line of the file and no `# prose: on` follows, the map sets `file_is_suppressed`.
3. `# yapf: disable` and `# yapf: enable` are recognized as aliases for `# fmt: off` and `# fmt: on`, so a project's existing yapf markers keep working. Other yapf directives are not recognized.
4. `# fmt: skip` at the end of a line exempts that one logical line from rewrites, scoped to that statement, and leaves its lint diagnostics to report. `# prose: skip` is the equivalent alias.
5. `# prose: skip[<rule>, <rule>, …]` exempts the listed auto-fix rules across the same logical line `# fmt: skip` covers, with whitespace inside the brackets tolerated and two bracketed directives on one line unioning their rule sets. An unknown rule slug is dropped with no notice.
6. `# prose: ignore[<rule>, <rule>, …]` exempts the listed lint rules on the directive's line, with the same bracket-whitespace tolerance and union behavior. A bare `# prose: ignore` covers every lint rule on the line.

## Re-Using This Primitive

The [[pipeline]] is the canonical consumer, applying the filter at the point edits are emitted, so a new rule emits its edits unconditionally and the pipeline drops the suppressed groups. A rule whose fix group would otherwise span an exempt row reads `suppresses` itself before grouping, the way the alignment rules exclude a skip-exempt row through the aligner's `is_held` before a column is resolved. [[alphabetize-siblings]], [[band-constants]], and [[modernize-annotations]] read it the same way. The map is built once per source and handed to every reader by reference.

A consumer reusing the suppression directives in a different formatter would build the same map and apply the same filter at its own edit-emission point, taking the whole directive set *(format spans, statement-level format markers, per-rule format directives, and per-line lint directives)* without re-implementing the scan.

The Cargo dependency line *(`prose = { git = "...", tag = "<version>" }`)* lives on the [[source]] page. A downstream consumer reaches the map only through `Pipeline::run` dropping suppressed diagnostics, not through direct method calls, and the user-facing directives are covered in full by the [**Suppression**](/usage/suppression) chapter.

<template #related>

- The [**Suppression**](/usage/suppression) chapter covers the directives the map records, with the syntax for block markers, line markers, and lint directives.
- [[source]] builds the map during construction and exposes it through `suppression_map()`.
- [[pipeline]] reads the map at the point edits are emitted, dropping suppressed entries before they reach the caller.
- [[rule-id]] is the handle the bracketed directives name inside the `# prose: skip[<slug>]` and `# prose: ignore[<slug>]` syntax.

For the rule catalog whose diagnostics this map filters, the [**Rules**](/rules/) page lists every shipped rule by category.

</template>

</PrimitiveLayout>
