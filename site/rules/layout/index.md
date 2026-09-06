---
description: "The rules that explode a bracketed construct to one entry per line once it outgrows its line."
---

# Layout Rules

The layout rules rewrite a bracketed construct that has outgrown one line, exploding a call, a signature, a collection, or a `from … import …` to one entry per line so each entry reads on its own and a later edit touches a single row. Each rule fires on a width budget such as `code-line-length`, a count cap such as `max-args`, or both, so the inline form gives way to the stacked one at the point it stops being legible.

<RuleCardList family="layout" />

The [**Configuration**](/reference/configuration) reference lists the per-rule facets, and the [**Pipeline Order**](/reference/pipeline-order) reference lists where these rules run *(layout settles the bracketed form early, so the alignment rules measure their columns against the layout it writes)*.
