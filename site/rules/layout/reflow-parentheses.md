---
caption : "Reflows a grouping parenthesis pair against the line budget, shedding one that binds nothing and breaking one that overflows into a row per operand."
related : [shed-backslash-continuations, reflow-collections, reflow-signatures, line-overflow]
layout  : doc
---

# reflow-parentheses

<RuleLayout rule="reflow_parentheses">

A parenthesis pair wrapped around an expression only to span lines, or out of habit, is visual weight the expression does not carry meaning through. A pair holding a condition wider than its row is the opposite, in that it is the one bracket able to bring that row back inside the budget. `reflow-parentheses` reads a pair against both readings, dropping one that binds nothing and breaking one whose joined form crosses `code-line-length`.

The decision is structural rather than textual, so a pair sheds only where removing it leaves the parse unchanged. A precedence-bearing pair such as `(a + b) * c` stays because dropping it would rebind the multiplication, a generator and a walrus binding keep the parentheses the grammar requires of them, and the parentheses that form a one-element tuple stay part of the tuple rather than wrapping it. A pair whose interior carries a comment stays too, since folding the break would strand the comment off the line it describes.

A wrapped multi-line grouping folds onto one line when the joined form fits the budget. The fold reads whether closing a soft wrap would respace a string's own interior rather than what kind of leaf the expression carries, so a comparison against a string literal joins exactly as one against a number does, and a run closing against a bracket closes to nothing rather than to a space so no padding strands behind it. A pair whose every break sits inside a bracket the interior itself opens, a call's argument list being the common one, sheds in place whatever the joined width, since the pair holds none of those breaks and [[reflow-calls]] settles the rows inside.

Where the joined form crosses the budget, the pair breaks rather than staying as the author left it. The opening bracket takes the row alone, the closing bracket opens the row beneath the last operand, and the interior lands between them one indent step in. An interior that fits a row of its own takes that row whole, and one that does not takes a row per operand, each row led by the operator joining it to the row above so a reader finds every `and` in one column rather than at three different row ends. A pair the author already broke takes the same render, so the operator arrives in the same place whether the rule opened the row or the author did.

The break reshapes only a pair that already exists, never adding one, so an over-budget expression carrying no parentheses is left for [[line-overflow]] to report. It reaches an operator chain alone, holding any other interior at the shape its author wrote, and it declines a pair wrapping one operand of a wider chain, since opening rows inside a row that overflows either way buys the reader nothing. A pair sitting inside a bracket the pass leaves standing declines for the same reason, leaving the construct that bracket belongs to to lay out the rows around it.

Both directions land in one pass. A pair nested inside another redundant pair sheds in the same run, each pair weighs its own fold against the text the pass's earlier sheds produce, and a break renders its operands through those same sheds, so the rows it opens carry the text the pass leaves rather than the text it was handed.

<template #configuration>

<RuleConfigTable />

</template>

<template #related-after>

For per-statement opt-outs, the [**Suppression**](/usage/suppression) chapter covers the `# prose: skip[reflow-parentheses]` directive, which holds every line a wrapped statement spans.

</template>

</RuleLayout>
