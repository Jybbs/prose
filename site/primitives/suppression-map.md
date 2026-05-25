# SuppressionMap

<PrimitiveLayout primitive="suppression-map">

Every source file in *Prose* gets a one-time scan for suppression directives during [[source]] construction, and the result lands in a *SuppressionMap*. The map indexes the format-suppression spans *(`# fmt: off` / `# fmt: on`, `# prose: off` / `# prose: on`, and the `# yapf: disable` / `# yapf: enable` aliases)*, the line-level format markers *(`# fmt: skip` and its `# prose: skip` alias)*, the per-rule format directives *(`# prose: skip[<rule>]`)*, and the per-line lint directives *(`# prose: ignore` and `# prose: ignore[<rule>]`)*. The [[pipeline]] consults the map at the edit-emission boundary, dropping suppressed edits and lint diagnostics before they surface to the caller.

## Public Surface

The *SuppressionMap* type itself is `pub(crate)` in `0.2.x`, so neither the type nor its methods are reachable from a downstream Rust consumer. The suppression behavior is reachable indirectly through [**`Pipeline::run`**](/primitives/pipeline), which already filters emitted edits and lint diagnostics against the map.

A downstream consumer in `0.2.x` interacts with suppression through the user-facing surface:

- Source files declare directives inline (*`# fmt: off`, `# fmt: skip`, `# prose: skip[<rule>]`, `# prose: ignore[<rule>]`*).
- The [[pipeline]] consumes the directives during `run`.
- Diagnostics and edits affected by suppression silently drop from the returned vectors.

The type stabilizes toward `1.0`, where the lookup methods open up so consumers can introspect suppression spans for downstream tooling (*IDE highlighting of suppressed ranges, lint-coverage reports, suppression audits*).

## Internal Surface

For consumers reading this from within the *Prose* crate, the map exposes a constructor and predicate set:

1. `from_comments(source, comments, first_code_offset) -> Self` builds the map by scanning the token stream for the comment shapes the suppression surface recognizes, with `first_code_offset` powering the `file_is_suppressed` shortcut.
2. `file_is_suppressed() -> bool` returns true when an unmatched `# prose: off` *(or `# fmt: off`)* sits at or before the first non-blank, non-comment line of the file, letting the pipeline short-circuit to identity before any rule fires.
3. `has_format_suppression() -> bool` answers whether any format-suppression span sits in the file.
4. `has_lint_suppression() -> bool` answers the same for `# prose: ignore` directives.
5. `has_skip_suppression() -> bool` answers the same for `# prose: skip[<rule>]` per-rule format directives.
6. `intersects<R: Ranged>(ranged: R) -> bool` returns true when the given range overlaps any format-suppression span.
7. `is_format_suppressed_at(line: OneIndexed, rule: RuleId) -> bool` returns true when the line carries a `# prose: skip[<rule>]` directive that names the rule.
8. `is_lint_suppressed_at(line: OneIndexed, rule: RuleId) -> bool` returns true when the line carries a `# prose: ignore` directive that names the rule, or a bare directive that widens to every rule.

The [[source]] accessor `suppression_map(&self) -> &SuppressionMap` is also `pub(crate)`. Every entry point above opens at `1.0`.

### Directive Recognition

The directive shapes that feed the map share a grammar living in the [**Suppression**](/usage/suppression) chapter, with the shapes the map indexes shown below:

1. `# fmt: off` opens a format-suppression span and `# fmt: on` closes it. A span without a matching closer runs to EOF. Nested or overlapping `# fmt: off` markers flatten, so the first `# fmt: on` after any number of `off` markers closes the span.
2. `# prose: off` and `# prose: on` share the same span machinery, so a project can pick whichever prefix reads better. When `# prose: off` sits at or before the first non-blank, non-comment line of the file and no `# prose: on` follows, the map sets `file_is_suppressed`.
3. `# yapf: disable` and `# yapf: enable` are recognized as aliases for `# fmt: off` and `# fmt: on`, covering the yapf-conventioned suppression. Other yapf directives are not recognized.
4. `# fmt: skip` at end-of-line marks one logical line for format suppression, scoped to that statement. `# prose: skip` is the equivalent alias.
5. `# prose: skip[<rule>, <rule>, …]` suppresses the listed auto-fix rules at the directive's line, with whitespace inside the brackets tolerated and two bracketed directives on one line unioning their rule sets. Unknown rule slugs are dropped silently.
6. `# prose: ignore[<rule>, <rule>, …]` suppresses the listed lint rules at the directive's line, with the same bracket-whitespace tolerance and union behavior. A bare `# prose: ignore` widens to every lint rule on the line.

## Re-Using This Primitive

Rules do not consult the map directly, because the [[pipeline]] is the canonical consumer and applies the filter at the edit-emission boundary. A rule author writing a new rule emits edits unconditionally and trusts the pipeline to drop the suppressed ones. The map is built once per source and handed across rule boundaries by reference, but downstream of the pipeline's filter step rather than inside any rule's `apply` body.

A consumer reusing the suppression surface in a different formatter would build the same map shape and apply the same filter at their own edit-emission boundary, picking up the directive coverage *(format spans, line-level format markers, per-rule format directives, and per-line lint directives)* without re-implementing the scan.

The Cargo dependency line *(`prose = { git = "...", tag = "<version>" }`)* lives on the [[source]] page. In `0.2.x` the consumption path runs indirectly through `Pipeline::run`'s suppressed-diagnostics behavior rather than direct method calls, and the user-facing surface lives entirely in the source-file directives the [**Suppression**](/usage/suppression) chapter covers.

<template #related>

- The [**Suppression**](/usage/suppression) chapter walks the directives the map indexes, with the syntax for block markers, line markers, and lint directives.
- [[source]] owns the map and consults it on behalf of consuming rules.
- [[pipeline]] consults the map at the edit-emission boundary, dropping suppressed entries before they surface.
- [[rule-id]] is the handle that bracketed directives reference inside the `# prose: skip[<slug>]` and `# prose: ignore[<slug>]` syntax.

For the rule catalog whose diagnostics this map filters, the [**Rules**](/rules/) page walks every shipped rule by category.

</template>

</PrimitiveLayout>
