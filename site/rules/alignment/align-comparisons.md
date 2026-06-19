---
caption : "Aligns comparison operators vertically across the operands of a multi-line `and`-chain or `or`-chain."
related : [align-colons, align-equals, align-imports, alphabetize, align-match-case]
layout  : doc
---

# align-comparisons

<RuleLayout rule="align_comparisons">

A multi-line boolean chain (*`and` or `or`*) of comparison operands reads as a small table where the operator anchors the relationship between left and right. When each operator sits at a different column the eye walks across each line individually, treating the chain as five sentences rather than one parallel structure. `align-comparisons` gathers the operator column into a shared one, so every comparator's last character lands on the same offset and the chain reads top to bottom as a single judgment.

The rule walks each `BoolOp` whose operands are all `Expr::Compare`. The widest operand's left side fixes the shared column, with variable-width operators (*`==`, `<=`, `is not`*) right-aligning so the operator's last character sits in the shared column. A chained compare (*`0 < x < 100`*) anchors on its first operator only. A non-comparison operand, a multi-line operand, or a blank line in the gap breaks the run.

<template #configuration>

<RuleConfigTable />

`max-shift` bounds how far an operator may shift to align. The rule walks each run of comparisons in source order and grows a column while its width spread stays within the cap, breaking a fresh column at the first row that would exceed it. A `max-shift` of `false` lifts the cap so a contiguous run folds into one column, and `0` forbids any shift. The [**per-rule facets**](/reference/configuration#per-rule-facets) reference covers the full semantics.

</template>

</RuleLayout>
