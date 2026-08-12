---
caption : "Stacks an implicitly concatenated string run one literal per line once its joined line overruns the budget."
related : [reflow-calls, reflow-collections, line-overflow, shed-parentheses, reflow-signatures]
layout  : doc
---

# stack-adjacent-strings

<RuleLayout rule="stack_adjacent_strings">

`stack-adjacent-strings` breaks a run of implicitly concatenated string literals one literal per line once its joined line overruns `code-line-length`, so the seam between one literal and the next falls at a line end rather than wherever the author stopped typing.

A run the enclosing brackets already carry breaks in place, each later literal landing at the indent of the row the run opens on, which is how a call argument, a collection element, and a dict value take the break. A run standing where no bracket holds it, a `return` value or an assignment's right side among them, gains the parentheses the continuation needs, its literals one indent step in and the closing `)` back at the statement's indent.

<Fixture rule="stack_adjacent_strings" case="bracketed_run_breaks_in_place" />

The rule only ever breaks a run, so a run already written one literal per line holds that shape whatever its width, backslash-continued and parenthesized alike. A run spanning several lines with two literals still sharing one normalizes to one per line whatever the width, the ragged seam being the defect rather than the line count.

A run standing as a body's leading expression keeps its line however wide, parenthesizing it leaving a docstring that no longer reads as one. A run holding a triple-quoted part that spans lines holds too, moving that part carrying its opening line away from the interior the source pinned, and a comment anywhere inside the enclosing pair pins the run as well.

Bytes runs and runs mixing an f-string or t-string with a plain literal all break the same way, since each is one implicitly concatenated expression. The break falls between the parts and never inside one, so a replacement field keeps its own text untouched. A run held in a docstring slot and a line no break can bring within budget both reach [[line-overflow]].

<template #configuration>

<RuleConfigTable />

The break answers to the top-level [`code-line-length`](/reference/configuration#top-level-keys) key, measured from the column the run lands at once [[align-equals]] settles the row that carries it.

</template>

</RuleLayout>
