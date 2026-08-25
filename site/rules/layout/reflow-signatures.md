---
caption : "Normalizes function signatures to one line or one parameter per line, gated by line length, parameter count, and a parameter spanning rows."
related : [align-colons, align-equals, reflow-calls, reflow-collections, strip-trailing-commas]
layout  : doc
---

# reflow-signatures

<RuleLayout rule="reflow_signatures">

A function signature reads as either a one-line declaration or a stacked column of parameters. Mixed shapes (*part on the `def` line, the rest indented underneath*) force the reader to track two layout idioms at once. `reflow-signatures` collapses every signature to the binary canonical form, deciding the shape from `code-line-length` and `max-params`.

The rule expands a signature when its inline form overflows the configured `code-line-length`, when its parameter count exceeds `max-params`, or when a parameter's own annotation or default spans rows. Otherwise the signature collapses to a single line. A comment inside the parameter list pins the existing shape, because moving the parameters would orphan the comment from its anchor. The expanded form lays each parameter on its own line, indented one step past the `def`, with the closing `)` flush left, the return annotation trailing on the same line, and the final parameter ending bare, the shape [[strip-trailing-commas]] accepts. A parameter the author wrote across rows travels whole into the expanded form the way [[reflow-collections]] carries a held member, and a `*args` or `**kwargs` annotation moves the same way. A call inside a parameter's annotation or default reshapes where that parameter lands, so a nested call measures against its expanded row rather than the one-line signature it started on.

<template #configuration>

<RuleConfigTable />

The line-length budget comes from the top-level [`code-line-length`](/reference/configuration#top-level-keys) key *(default `88`)*, which the rule reads directly. Setting `max-params` to `false` makes the rule expand purely on line length, leaving inline-but-long signatures untouched when they fit the budget regardless of parameter count.

</template>

</RuleLayout>
