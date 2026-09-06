---
caption : "Removes a grouping parenthesis pair that changes nothing, and breaks one whose joined form overflows `code-line-length` into a row per operand."
related : [shed-backslash-continuations, reflow-collections, reflow-signatures, line-overflow]
layout  : doc
---

# reflow-parentheses

<RuleLayout rule="reflow_parentheses">

`reflow-parentheses` removes a parenthesis pair that groups nothing, one wrapped around an expression only to span lines or out of habit, and breaks a pair whose joined form crosses `code-line-length` into a row per operand, since that pair is the one bracket able to bring the row back inside the budget.

The removal test is structural rather than textual, so a pair is removed only where removing it leaves both the parse and the grouping a reader sees unchanged. That test keeps each of the following pairs in place:

1. A precedence-bearing pair such as `(a + b) * c`, because dropping it would rebind the multiplication.
2. A generator and a walrus binding, each keeping the parentheses the grammar requires of it.
3. The parentheses that form a one-element tuple, which are part of the tuple rather than a wrapper around it.
4. A pair around one boolean operator inside a chain of the other, since `(a and b) or c` parses the same without it and would then read as three peers rather than two.
5. A pair whose interior carries a comment, since closing the break would move the comment off the line it describes.

A multi-line pair joins onto one line when the joined form fits the budget. The join tests whether closing a soft line break would change the spacing inside a string rather than what kind of leaf the expression carries, so a comparison against a string literal joins exactly as one against a number does. A run closing against a bracket closes to nothing rather than to a space, so no padding is left behind it. A pair whose every break sits inside a bracket the interior itself opens, a call's argument list being the common case, is removed in place whatever the joined width, since the pair carries none of those breaks and [[reflow-calls]] settles the rows inside.

Where the joined form crosses the budget, the pair breaks rather than staying as the author left it. The opening bracket takes its row alone, the closing bracket opens the row beneath the last operand, and the interior sits between them one indent step in. An interior that fits a row of its own takes that row whole, and one that does not takes a row per operand, each row led by the operator joining it to the row above, so a reader finds every `and` in one column rather than at three different row ends. A pair the author already broke takes the same layout, so the operator arrives in the same place whether the rule opened the row or the author did.

The break reshapes only a pair that already exists and never adds one, so an over-budget expression carrying no parentheses is left for [[line-overflow]] to report. It reaches an operator chain alone and leaves any other interior at the layout its author wrote. A pair wrapping one operand of a wider chain is left as written too, since opening rows inside a row that overflows either way gains the reader nothing. A pair sitting inside a bracket the rule leaves standing is left as written for the same reason, so the construct that bracket belongs to lays out the rows around it.

Both directions are written in one pass, so a pair nested inside another redundant pair is removed in the same run and each pair tests its own join against the text the earlier removals produce. A break writes its operands through those same removals, leaving the rows it opens carrying the text the rule leaves rather than the text it was handed.

<template #configuration>

<RuleConfigTable />

</template>

<template #related-after>

For per-statement opt-outs, the [**Suppression**](/usage/suppression) chapter covers the `# prose: skip[reflow-parentheses]` directive, which keeps every line a wrapped statement spans as written.

</template>

</RuleLayout>
