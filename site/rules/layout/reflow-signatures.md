---
caption : "Writes a function signature either on one line or one parameter per line, expanding it once it overflows `code-line-length`, passes `max-params`, or carries a parameter spanning rows."
related : [align-colons, align-equals, reflow-calls, reflow-collections, strip-trailing-commas]
layout  : doc
---

# reflow-signatures

<RuleLayout rule="reflow_signatures">

`reflow-signatures` writes every function signature in one of two forms, a one-line declaration or one parameter per line, the choice set by `code-line-length` and `max-params`. A mixed form (*part on the `def` line, the rest indented underneath*) forces the reader to track two layouts at once, so it is rewritten to one or the other.

The rule expands a signature where any of the following is true:

1. Its inline form overflows the configured `code-line-length`.
2. Its parameter count exceeds `max-params`.
3. A parameter's own annotation or default spans rows.

Otherwise the signature collapses to a single line. A comment inside the parameter list keeps the existing layout, because moving the parameters would separate the comment from the line it describes. The expanded form puts each parameter on its own line, indented one step past the `def`, with the closing `)` flush with the `def`, the return annotation trailing on the same line as the `)`, and the final parameter ending without a comma, the form [[strip-trailing-commas]] accepts. A parameter the author wrote across rows moves whole into the expanded form the way [[reflow-collections]] moves a member it keeps as written, and a `*args` or `**kwargs` annotation moves the same way. A call inside a parameter's annotation or default is reshaped where that parameter ends up, so a nested call is measured against its expanded row rather than the one-line signature it started on.

<template #configuration>

<RuleConfigTable />

The line-length budget comes from the top-level [`code-line-length`](/reference/configuration#top-level-keys) key *(default <ConfigDefault facet="code-line-length" />)*, which the rule reads directly. Setting `max-params` to `false` makes the rule expand on line length alone, so a signature that fits the budget stays inline whatever its parameter count.

</template>

</RuleLayout>
