---
description: "The rules that explode a bracketed construct across rows once it outgrows its line."
---

# Layout Rules

The layout rules rewrite a bracketed construct that has outgrown one line, exploding a call to one argument per line, expanding a signature to one parameter per line and a collection across rows, and splitting a `from … import …` into a run of statements. Each rule fires on a width budget such as [`code-line-length`](/reference/configuration#top-level-keys), a count cap such as `max-args`, or both, so the inline form gives way to the stacked one at the point it stops being legible. No layout rule breaks a line inside an f-string or t-string replacement field, because a line break inside one is PEP 701 syntax that fails to parse before Python 3.12, so an over-wide interpolation is left for [[line-overflow]] to report.

<RuleCardList family="layout" />
