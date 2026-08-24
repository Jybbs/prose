---
caption : "Sheds a grouping parenthesis pair that binds nothing, reflowing the expression onto the line it now fits."
related : [shed-backslash-continuations, reflow-collections, reflow-signatures]
layout  : doc
---

# shed-parentheses

<RuleLayout rule="shed_parentheses">

A parenthesis pair wrapped around an expression only to span lines, or out of habit, is visual weight the expression does not carry meaning through. [[reflow-collections]] joins a wrapped construct back onto the line it fits, yet it leaves the surrounding parentheses in place, because removing syntax belongs to no layout rule. `shed-parentheses` closes that gap, dropping a grouping pair that binds nothing and reflowing the expression onto the line it now fits.

The decision is structural rather than textual, so a pair sheds only where removing it leaves the parse unchanged. A precedence-bearing pair such as `(a + b) * c` stays because dropping it would rebind the multiplication, a generator and a walrus binding keep the parentheses the grammar requires of them, and the parentheses that form a one-element tuple stay part of the tuple rather than wrapping it. A pair whose interior carries a comment stays too, since folding the break would strand the comment off the line it describes.

A wrapped multi-line grouping folds onto one line when the bare form fits the budget. One whose joined line would overflow sheds in place when an enclosing bracket already holds its breaks, leaving the layout rules to seat the rows, and stays wrapped only where no enclosing bracket exists, so a long boolean condition at a statement head keeps its parentheses across the lines it needs. A pair whose every break sits inside a bracket the interior itself opens, a call's argument list being the common one, sheds in place whatever the joined width, since the pair holds none of those breaks and [[reflow-calls]] settles the rows inside. A pair nested inside another redundant pair sheds in the same pass, and each pair weighs its own fold against the text the pass's earlier sheds produce, so two sibling pairs on one statement answer the budget the joined line actually reaches rather than the width the source opened with.

<template #configuration>

<RuleConfigTable />

</template>

<template #related-after>

For per-statement opt-outs, the [**Suppression**](/usage/suppression) chapter covers the `# prose: skip[shed-parentheses]` directive, which holds every line a wrapped statement spans.

</template>

</RuleLayout>
