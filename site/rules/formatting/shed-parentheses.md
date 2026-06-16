---
caption : "Sheds a grouping parenthesis pair that binds nothing, reflowing the expression onto the line it now fits."
related : [collection-layout, signature-layout]
layout  : doc
---

# shed-parentheses

<RuleLayout rule="shed_parentheses">

A parenthesis pair wrapped around an expression only to span lines, or out of habit, is visual weight the expression does not carry meaning through. [[collection-layout]] joins a wrapped construct back onto the line it fits, yet it leaves the surrounding parentheses in place, because removing syntax belongs to no layout rule. `shed-parentheses` closes that gap, dropping a grouping pair that binds nothing and reflowing the expression onto the line it now fits.

The decision is structural rather than textual, so a pair sheds only where removing it leaves the parse unchanged. A precedence-bearing pair such as `(a + b) * c` stays because dropping it would rebind the multiplication, a generator and a walrus binding keep the parentheses the grammar requires of them, and the parentheses that form a one-element tuple stay part of the tuple rather than wrapping it. A pair whose interior carries a comment stays too, since folding the break would strand the comment off the line it describes.

A wrapped multi-line grouping folds onto one line when the bare form fits the budget and stays wrapped when it would overflow, so a short boolean condition reads as one clean line whereas a long one keeps its parentheses across the lines it needs. A pair nested inside another redundant pair sheds in the same pass.

<template #configuration>

| Key | Type | Default | Meaning |
|---|---|---|---|
| `enabled` | bool | `true` | Toggle the rule on or off |

</template>

<template #related-after>

For per-line opt-outs, the [**Suppression**](/usage/suppression) chapter covers the `# prose: skip[shed-parentheses]` directive.

</template>

</RuleLayout>
