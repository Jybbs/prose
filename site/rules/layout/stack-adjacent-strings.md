---
caption : "Breaks a run of implicitly concatenated string literals to one literal per line once the joined line overflows `code-line-length`."
related : [reflow-calls, reflow-collections, line-overflow, reflow-parentheses, reflow-signatures]
layout  : doc
---

# stack-adjacent-strings

<RuleLayout rule="stack_adjacent_strings">

`stack-adjacent-strings` breaks a run of implicitly concatenated string literals to one literal per line once the joined line overflows `code-line-length`, so the seam between one literal and the next falls at a line end rather than wherever the author stopped typing.

A run already inside a bracket pair breaks in place, each later literal written at the indent of the row the run opens on, which is how a call argument, a collection element, and a dict value take the break. A run standing where no bracket encloses it, a `return` value or an assignment's right side among them, gains the parentheses the continuation needs, its literals one indent step in and the closing `)` back at the statement's indent.

<Fixture rule="stack_adjacent_strings" case="bracketed_run_breaks_in_place" />

The rule only ever breaks a run, so a run already written one literal per line keeps that layout whatever its width, backslash-continued and parenthesized alike. A run spanning several lines with two literals still sharing one is rewritten to one per line whatever the width, because the ragged seam is the defect rather than the line count.

A run keeps its line however wide in each of the following cases:

1. A run standing as a body's leading expression, because parenthesizing it would leave a docstring that no longer reads as one.
2. A run with a triple-quoted part that spans lines, because moving that part would shift its opening line and change the interior the source pinned.
3. A run with a comment anywhere inside the enclosing pair, the comment keeping the run in place.

Bytes runs and runs mixing an f-string or t-string with a plain literal all break the same way, since each is one implicitly concatenated expression. The break falls between the parts and never inside one, so a replacement field keeps its own text as written. A run in a docstring slot and a line no break can bring within budget are both left for [[line-overflow]] to report.

<template #configuration>

<RuleConfigTable />

The break reads the top-level [`code-line-length`](/reference/configuration#top-level-keys) key, and the width is measured from the column the run sits at once [[align-equals]] settles the row that carries it.

</template>

</RuleLayout>
