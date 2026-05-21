# SuppressionMap

<PrimitiveLayout primitive="suppression-map">

Every source file in *Prose* gets a one-time scan for suppression directives during [[source]] construction, and the result lands in a *SuppressionMap*. The map indexes `# fmt: off` / `# fmt: on` block spans, `# fmt: skip` line markers, the `# yapf: disable` / `# yapf: enable` aliases, and `# prose: ignore[...]` per-line lint directives. The [[pipeline]] consults the map at the edit-emission boundary, dropping suppressed edits and lint diagnostics before they surface to the caller.

## Public Surface

The *SuppressionMap* type itself is `pub(crate)` in `0.2.x`, so neither the type nor its methods are reachable from a downstream Rust consumer. The suppression behavior is reachable indirectly through [**`Pipeline::run`**](/primitives/pipeline), which already filters emitted edits and lint diagnostics against the map.

A downstream consumer in `0.2.x` interacts with suppression through the user-facing surface:

- Source files declare directives inline (*`# fmt: off`, `# fmt: skip`, `# prose: ignore[<rule>]`*).
- The [[pipeline]] consumes the directives during `run`.
- Diagnostics and edits affected by suppression silently drop from the returned vectors.

The type stabilizes toward `1.0`, where the lookup methods open up so consumers can introspect suppression spans for downstream tooling (*IDE highlighting of suppressed ranges, lint-coverage reports, suppression audits*).

## Internal Surface

For consumers reading this from within the *prose* crate, the map exposes four predicates plus a constructor:

- `from_comments(source, comments) -> Self` builds the map by scanning the token stream for the four comment shapes the suppression surface recognizes.
- `has_format_suppression() -> bool` answers whether any block-format suppression sits in the file.
- `has_lint_suppression() -> bool` answers the same for lint directives.
- `intersects<R: Ranged>(ranged: R) -> bool` returns true when the given range overlaps any `# fmt: off` block or `# fmt: skip` line.
- `is_lint_suppressed_at(line: OneIndexed, rule: RuleId) -> bool` returns true when the line carries a `# prose: ignore` directive that names the rule (or a bare directive that widens to every rule).

The [[source]] accessor `suppression_map(&self) -> &SuppressionMap` is also `pub(crate)`. All five entry points open at `1.0`.

## Re-Using This Primitive

The map is built once per source and handed across rule boundaries by reference. The [[pipeline]] is the canonical consumer, applying the filter at the edit-emission boundary so downstream rules and final diagnostics both honor the directives. A consumer reusing the suppression surface in a different formatter would build the same map shape and apply the same filter at their own edit-emission boundary.

A downstream Rust crate consumes *prose* through a Git dependency pinned to a release tag:

```toml
[dependencies]
prose = { git = "https://github.com/Jybbs/prose", tag = "0.2.3" }
```

In `0.2.x` the consumption path is indirect (*through suppressed-diagnostics behavior of [**`Pipeline::run`**](/primitives/pipeline)*) rather than direct method calls. The user-facing surface lives entirely in the source-file directives that the [**Suppression**](/guide/suppression) chapter covers.

<template #related>

- The [**Suppression**](/guide/suppression) chapter walks the directives the map indexes, with the syntax for block markers, line markers, and lint directives.
- [[source]] owns the map and consults it on behalf of consuming rules.
- [[pipeline]] consults the map at the edit-emission boundary, dropping suppressed entries before they surface.
- [[rule-id]] is the handle that lint directives reference inside the `# prose: ignore[<slug>]` syntax.

For the rule catalog that may emit suppressed diagnostics, the [**Rules Overview**](/rules/) page walks every shipped rule by category.

</template>

</PrimitiveLayout>
