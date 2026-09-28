---
caption : "Pads the space before `=` so consecutive assignments, annotated parameter defaults, and an exploded call's keyword arguments share one column."
related : [align-colons, align-imports, align-match-case, reflow-calls, reflow-collections, strip-stranded-padding]
layout  : doc
---

# align-equals

<RuleLayout rule="align_equals">

`align-equals` pads the space before `=` so that consecutive assignments share one column. A stretch of bindings then reads as a list of names beside a list of values, and the eye finds each value by dropping straight down the operators rather than searching for the `=` on every line.

The rule aligns three kinds of run:

1. Consecutive single-target assignments at the same indentation, with or without a type annotation. An augmented assignment (*`+=`, `|=`*) joins the run with its operator one column before the shared `=`, whereas a walrus (*`:=`*) never joins. A PEP 695 `type` statement joins as well, its name side measured from `type` through any type parameter list, so `type Pair[T]` and `type Id` share one column.
2. Consecutive annotated function-parameter defaults, so a signature with several `param: type = default` entries aligns its `=` the way a stretch of module-level bindings does.
3. The keyword arguments that each sit alone on their line in an exploded call, so a call written one keyword per line aligns its `name = value` column the way a signature aligns its defaults.

Every aligned row reads as `name = value`, with the name padded out to the column and one space after the operator. A keyword alone on its line with no column to share still takes one space on each side of its `=`, whereas a keyword that shares its line with another argument keeps the tight `name=value` form PEP 8 gives a call-site keyword.

A positional argument, a `**` unpacking, an interior comment, or a keyword sharing a line with another argument ends the run, whereas a multi-line value or default joins its run and then closes it, leaving the entries after it to align as a separate group. A blank line, a comment line, or a statement of another kind ends a run of assignments, so each contiguous run aligns on its own. Once a group aligns, [[strip-stranded-padding]] removes the padding of any one-member group, and a lone binding then reads as plain code.

Where padding an assignment or a `type` statement out to the column would carry its row past `code-line-length`, [[reflow-calls]] explodes a call value and [[reflow-collections]] expands a collection value in the same pass, so the opening row joins the column, provided the padding stays within `max-shift`. A value broken open ends its run like any other multi-line value, and the rows below it start a run of their own. The layout rules therefore break a value open only where that leaves the run in fewer columns, or in as many columns with fewer rows unpadded, than keeping every value on one row would, so a middle row whose expansion would only move which row stands unpadded keeps its row. Where a layout rule would break a row's value at a bracket inside it rather than at the bracket opening it, the row breaks open only once it crosses the budget with no padding at all. That bracket can belong to any expression nested in the value, such as a list inside a comprehension, a call used as an operand of `+` or as a branch of a conditional expression, or a call whose result is indexed, called again, or read as an attribute. An annotated row above the last row of an [[align-colons]] run also breaks open only once it crosses the budget unpadded, because breaking it open sooner would end that run. The column is read in the order the statements take once [[band-constants]] moves a module's constants into their bands and [[alphabetize-siblings]] sorts a class body, so a run those rules rearrange breaks open where its reordered rows cross the budget.

<template #configuration>

<RuleConfigTable />

`max-shift` limits how much padding one row may take. The rule reads each run in source order and extends a column while the gap between the widest and narrowest names stays within the limit, starting a new column at the first row that would exceed it. Setting `max-shift` to `false` removes the limit, so a run of any width aligns on one column, and `0` forbids padding altogether, so every `=` sits one space after its name. The [**per-rule facets**](/reference/configuration#per-rule-facets) reference covers the full semantics.

</template>

</RuleLayout>
