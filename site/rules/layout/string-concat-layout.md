---
caption : "Breaks an implicitly concatenated string run one literal per line once its joined line overruns the budget."
related : [call-layout, collection-layout, line-overflow, shed-parentheses, signature-layout]
layout  : doc
---

# string-concat-layout

<RuleLayout rule="string_concat_layout">

Adjacent string literals concatenate at compile time, which makes a long message easy to assemble and easy to lose track of, since the run either overruns a single line or scatters across several with the seams landing wherever the author stopped typing. `string-concat-layout` gives the run a settled shape by breaking it one literal per line once its joined line overruns `code-line-length`, so the seam between one literal and the next falls at a line end, where the eye already looks for it. The same one-entry-per-line reading the layout family applies to calls, signatures, and collections reaches a string run too.

A run the enclosing brackets already carry breaks in place, each later literal landing at the indent of the row the run opens on, which is how a call argument, a collection element, and a dict value take the break. A run standing where no bracket holds it, a `return` value or an assignment's right side among them, gains the parentheses the continuation needs, its literals one indent step in and the closing `)` back at the statement's indent.

<Fixture rule="string_concat_layout" case="bracketed_run_breaks_in_place" />

The rule only ever breaks a run, so a run the author already wrote one literal per line holds that shape whatever its width and a deliberate stack survives rather than being rejoined into a line the next edit would have to break again, a hold that covers a backslash-continued run as readily as a parenthesized one. A run spanning several lines with two literals still sharing one is the shape the rule settles, so it normalizes to one per line whatever the width, the ragged seam being the defect rather than the line count.

A run standing as a body's leading expression stays exactly as written and keeps its line however wide it runs, because parenthesizing it would leave a docstring that no longer reads as one. A run holding a triple-quoted part that spans lines keeps its place for a different reason, wherein moving that part would carry its opening line away from the interior the source pinned. A comment anywhere inside the enclosing pair pins the run as well, the rewrite spanning the pair and having nowhere to put the comment.

Bytes runs and runs mixing an f-string or t-string with a plain literal all break the same way, since each is one implicitly concatenated expression. The break falls between the parts and never inside one, so a replacement field keeps its own text untouched. A run held in a docstring slot and a line no break can bring within budget both reach [`line-overflow`](/rules/lint/line-overflow), which reports the line rather than leaving it for a rule that will not act on it.

<template #configuration>

<RuleConfigTable />

The break answers to the top-level [`code-line-length`](/reference/configuration#top-level-keys) key, measured from the column the run lands at once [[align-equals]] settles the row that carries it.

</template>

</RuleLayout>
