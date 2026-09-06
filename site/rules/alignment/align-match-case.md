---
caption : "Folds each single-statement `case` arm onto one line and pads the space before its `:` so consecutive arms share one column."
related : [align-colons, align-equals, align-imports, strip-stranded-padding]
layout  : doc
---

# align-match-case

<RuleLayout rule="align_match_case">

`align-match-case` folds each single-statement `case` arm onto its `case` line and pads the space before the `:` so consecutive arms share one column, with the patterns flush left and the bodies starting at one column to the right. A `match` whose arms each contain one statement then reads as a dispatch table, patterns on the left and results on the right, and the reader scans rows rather than tracing each body.

The rule acts only on runs of single-statement arms at the same indentation, and an arm folds only where its one-line form fits within `code-line-length`. An arm stays multi-line and ends the run when it holds more than one statement, when its body is a compound statement or spans several lines, or when its folded form would overflow the budget, leaving the arms on each side of it aligned on their own. An own-line comment between two arms passes through, and the arms on both sides of it still share one column. A nested `match` aligns as a group of its own. Pair the rule with [[strip-stranded-padding]], which strips the padding on a one-arm `match`, and with [[align-colons]], which aligns the separators inside a dict a case body returns.

<template #configuration>

<RuleConfigTable />

`max-shift` limits how much padding one `:` may take. The rule reads each run of arms in source order and extends a column while the gap between the widest and narrowest patterns stays within the limit, starting a new column at the first arm that would exceed it. Setting `max-shift` to `false` removes the limit, so a run of any width aligns on one column, and `0` forbids padding altogether, so every `:` sits flush against its pattern. The [**per-rule facets**](/reference/configuration#per-rule-facets) reference covers the full semantics.

</template>

</RuleLayout>
