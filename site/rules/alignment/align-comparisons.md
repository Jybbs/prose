---
caption : "Pads the space before each comparison operator in a multi-line `and` or `or` chain so the operators share one column."
related : [align-colons, align-equals, align-imports, alphabetize-siblings, align-match-case, normalize-comparisons]
layout  : doc
---

# align-comparisons

<RuleLayout rule="align_comparisons">

`align-comparisons` pads the space before each comparison operator in a multi-line `and` or `or` chain so the operators share one column, and the chain then reads top to bottom as one parallel structure, each left operand beside its right, rather than as a stack of separate sentences the eye reads one at a time.

The rule reads each `BoolOp` whose operands are all `Expr::Compare`. The widest left operand sets the shared column, and operators of differing widths (*`==`, `<=`, `is not`*) right-align so the last character of each sits in that column. A chained compare (*`0 < x < 100`*) aligns on its first operator only. A non-comparison operand, a multi-line operand, or a blank line between operands ends the run.

<template #configuration>

<RuleConfigTable />

`max-shift` limits how much padding one operator may take. The rule reads each run of comparisons in source order and extends a column while the gap between the widest and narrowest left operands stays within the limit, starting a new column at the first row that would exceed it. Setting `max-shift` to `false` removes the limit, so a run of any width aligns on one column, and `0` forbids padding altogether. The [**per-rule facets**](/reference/configuration#per-rule-facets) reference covers the full semantics.

</template>

</RuleLayout>
