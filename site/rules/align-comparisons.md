---
category : auto-fix
family   : alignment
caption  : "aligns comparison operators vertically across the operands of a multi-line `and`-chain or `or`-chain."
related  : [align-colons, align-equals, align-imports, alphabetize, match-case-align]
layout   : doc
---

# align-comparisons

<RuleLayout rule="align_comparisons">

A multi-line boolean chain (*`and` or `or`*) of comparison operands reads as a small table where the operator anchors the relationship between left and right. When each operator sits at a different column the eye walks across each line individually, treating the chain as five sentences rather than one parallel structure. `align-comparisons` gathers the operator column into a shared one, so every comparator's last character lands on the same offset and the chain reads top to bottom as a single judgment.

The rule walks each `BoolOp` whose operands are all `Expr::Compare`. The widest operand's left side fixes the shared column, with variable-width operators (*`==`, `<=`, `is not`*) right-aligning so the operator's last character sits in the shared column. A chained compare (*`0 < x < 100`*) anchors on its first operator only. A non-comparison operand, a multi-line operand, or a blank line in the gap breaks the run.

<template #configuration>

<RuleConfigTable />

`max-shift` caps the per-line padding the alignment can introduce, and `max-shift-policy` resolves the fallback when a group's widest operand would push the operator column past the cap. The [**per-rule knobs**](/reference/configuration#per-rule-knobs) reference covers the `"split"` / `"drop"` / `"skip"` semantics.

</template>

<template #canonical-lead>

Three single-comparator `==` operands in a multi-line `and`-chain share an alignment column. The operator at each row sits one space past the widest operand's left side.

</template>

</RuleLayout>
