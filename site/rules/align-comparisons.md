---
category : auto-fix
family   : alignment
caption  : "aligns comparison operators vertically across the operands of a multi-line `and`-chain or `or`-chain."
related  : [align-colons, align-equals, align-imports, alphabetize, match-case-align]
layout   : doc
---

# align-comparisons

<RuleLayout rule="align_comparisons" canonical="basic">

A multi-line boolean chain (*`and` or `or`*) of comparison operands reads as a small table where the operator anchors the relationship between left and right. When each operator sits at a different column the eye walks across each line individually, treating the chain as five sentences rather than one parallel structure. `align-comparisons` gathers the operator column into a shared one, so every comparator's last character lands on the same offset and the chain reads top to bottom as a single judgment.

The rule walks each `BoolOp` whose operands are all `Expr::Compare`. The widest operand's left side fixes the shared column, with variable-width operators (*`==`, `<=`, `is not`*) right-aligning so the operator's last character sits in the shared column. A chained compare (*`0 < x < 100`*) anchors on its first operator only. A non-comparison operand, a multi-line operand, or a blank line in the gap breaks the run.

<template #canonical-lead>

Three single-comparator `==` operands in a multi-line `and`-chain share an alignment column. The operator at each row sits one space past the widest operand's left side.

</template>

<template #more-examples>

<Fixture rule="align_comparisons" case="or_chain" title="`or`-Chains Align with the Same Column Math as `and`-Chains" />

<Fixture rule="align_comparisons" case="mixed_operators" title="Mixed-Width Operators Right-Align on the Shared Column" />

<Fixture rule="align_comparisons" case="chained_compare" title="A Chained Compare Anchors on Its First Operator" />

<Fixture rule="align_comparisons" case="identity_operators" title="`is` and `is not` Participate in the Same Column" />

<Fixture rule="align_comparisons" case="call_on_left" title="Call Expressions on the Left Pad the Same as Names" />

<Fixture rule="align_comparisons" case="blank_line_breaks" title="A Blank Line in the Gap Breaks the Run" />

<Fixture rule="align_comparisons" case="multi_line_operand_breaks" title="A Multi-Line Operand Breaks the Run" />

<Fixture rule="align_comparisons" case="with_comments" title="Trailing Comments Extend with the Operator Column" />

<Fixture rule="align_comparisons" case="idempotent" title="Already-Aligned Source Is Left Alone" />

</template>

</RuleLayout>
