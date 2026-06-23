# Layout Rules

The layout rules decide the shape a bracketed construct takes once it outgrows a single line, exploding a call, signature, collection, or `from … import …` to one entry per line so each binding reads on its own and a later edit touches a single row. The trigger is a width budget like `code-line-length`, a count cap like `max-args`, or both, so the inline shape gives way to the stacked one the moment it stops being legible.

<RuleCardList family="layout" />

For the per-rule facets, see the [**Configuration**](/reference/configuration) reference. For the order these rules fire in *(layout settles the bracketed shape early, so the alignment rules measure their columns against the committed layout)*, see the [**Pipeline Order**](/reference/pipeline-order) reference.
