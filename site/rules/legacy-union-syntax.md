---
category : lint
family   : lint
caption  : "surfaces `Union[A, B]` and `Optional[T]` patterns that should read as modern `A | B` and `T | None`."
related  : [unused-future-annotations]
layout   : doc
---

# legacy-union-syntax

<RuleLayout rule="legacy_union_syntax">

`Union[X, Y]` and `Optional[X]` come from the `typing` module, were the canonical union shapes for years, and still read clearly today. On Python 3.10 and later, the PEP 604 pipe-union shapes (*`X | Y`, `X | None`*) read more directly and consume one fewer import, which over the course of a large codebase adds up to genuinely clearer type signatures. `legacy-union-syntax` surfaces the legacy form as a lint, leaving the rewrite to a future migration pass that picks up the lint output.

The rule fires only on projects whose `target-version` is 3.10 or higher, where the pipe-union shapes are runtime-supported. Pre-3.10 projects and projects with `target-version` unset stay quiet, since recommending the pipe form on those projects would mislead. The lint is non-rewriting, so the diagnostic surfaces without touching the source.

::: tabs key:prose-target-version
== Python 3.10
The rule fires. `Optional[X]` reads as `X | None` in the diagnostic message.

== Python 3.11
The rule fires. Same diagnostic as 3.10.

== Python 3.12
The rule fires. Same diagnostic as 3.10.

== Python 3.13
The rule fires. Same diagnostic as 3.10.

== Python 3.14
The rule fires. Same diagnostic as 3.10, and pairs naturally with the deferred-annotation runtime that [[unused-future-annotations]] reads on the same axis.
:::

<template #configuration>

<RuleConfigTable />

The `target-version` field from the top-level [**Configuration**](/reference/configuration#top-level-keys) gates the lint per project.

</template>

<template #canonical-lead>

A `from typing import Optional` followed by `Optional[X]` surfaces the lint, recommending `X | None`.

</template>

<template #related-after>

For per-line opt-outs, the [**Suppression**](/usage/suppression#lint-directives) chapter covers the `# prose: ignore[legacy-union-syntax]` directive.

</template>

</RuleLayout>
