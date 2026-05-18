---
category: lint
related : [unused-future-annotations]
---

# legacy-union-syntax

`Union[X, Y]` and `Optional[X]` come from the `typing` module, were the canonical union shapes for years, and still read clearly today. On Python 3.10 and later, the PEP 604 pipe-union shapes (*`X | Y`, `X | None`*) read more directly and consume one fewer import, which over the course of a large codebase adds up to genuinely clearer type signatures. *Legacy-union-syntax* surfaces the legacy form as a lint, leaving the rewrite to a future migration pass that picks up the lint output.

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

## Configuration

<RuleConfigTable preset="toggle" />

The `target-version` field from the top-level [**Configuration**](/guide/configuration#top-level-keys) gates the lint per project.

## The Canonical Case

A `from typing import Optional` followed by `Optional[X]` surfaces the lint, recommending `X | None`.

<Fixture rule="legacy_union_syntax" case="optional_basic" />

## More Examples

<Fixture rule="legacy_union_syntax" case="modern_pipe_preserved" title="Already-Modern Pipe-Union Syntax Stays Clean" />

<Fixture rule="legacy_union_syntax" case="nested_subscript" title="Nested Subscripts Inside `Union` Are Recognized" />

<Fixture rule="legacy_union_syntax" case="aliased_import" title="Aliased Imports of `typing.Optional` Are Flagged Too" />

<Fixture rule="legacy_union_syntax" case="qualified_typing_optional" title="Qualified `typing.Optional` Is Flagged the Same Way" />

<Fixture rule="legacy_union_syntax" case="parens_extended_range" title="Parenthesized Union Expressions Are Recognized" />

<Fixture rule="legacy_union_syntax" case="typing_list_not_flagged" title="Non-Union `typing` Imports Like `List` Stay Quiet" />

<Fixture rule="legacy_union_syntax" case="idempotent" title="Pipe-Union Source Is Left Alone" />

## Related

The version-gated surface composes with one other rule on the same `target-version` axis.

- [[unused-future-annotations]] removes `from __future__ import annotations` when removal is provably safe, also reading `target-version` for the gate.

For per-line opt-outs, the [**Suppression**](/guide/suppression#lint-directives) chapter covers the `# prose: ignore[legacy-union-syntax]` directive.
