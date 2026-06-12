---
category : auto-fix
family   : formatting
caption  : "normalizes function signatures to one line or one parameter per line, gated by line length and inline-parameter count."
related  : [align-colons, align-equals, collection-layout, strip-trailing-commas]
layout   : doc
---

# signature-layout

<RuleLayout rule="signature_layout">

A function signature reads as either a one-line declaration or a stacked column of parameters. Mixed shapes (*part on the `def` line, the rest indented underneath*) force the reader to track two layout idioms at once. `signature-layout` collapses every signature to the binary canonical form, deciding the shape from `code-line-length` and `max-inline-params`.

The rule expands a signature when its inline form overflows the configured `code-line-length`, or when its parameter count exceeds `max-inline-params`. Otherwise the signature collapses to a single line. A comment inside the parameter list pins the existing shape, because moving the parameters would orphan the comment from its anchor. The expanded form lays each parameter on its own line, indented one step past the `def`, with the closing `)` flush left, the return annotation trailing on the same line, and the final parameter ending bare, the shape [[strip-trailing-commas]] accepts.

<template #configuration>

| Key | Type | Default | Meaning |
|---|---|---|---|
| `enabled` | bool | `true` | Toggle the rule on or off |
| `max-inline-params` | positive int \| `false` | `4` | Cap on the parameter count an inline signature can carry. Setting `false` disables the count trigger and leaves only the line-length budget |

The line-length budget comes from the top-level [`code-line-length`](/reference/configuration#top-level-keys) key *(default `88`)*, which the rule reads directly. Setting `max-inline-params` to `false` makes the rule expand purely on line length, leaving inline-but-long signatures untouched when they fit the budget regardless of parameter count.

</template>

</RuleLayout>
