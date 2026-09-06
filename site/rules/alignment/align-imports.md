---
caption : "Pads the space before the `import` keyword across consecutive `from` imports, or before `as` across consecutive aliased imports, so the keywords share one column."
related : [align-colons, align-equals, alphabetize-siblings, bare-imports, space-statements, align-match-case]
layout  : doc
---

# align-imports

<RuleLayout rule="align_imports">

`align-imports` pads the space before the `import` keyword across consecutive `from … import …` statements (*or before `as` across consecutive `import … as …` statements*) so the keyword sits at one column, leaving the module names flush left and the imported names starting at one column to the right. An import block pairs the module a name comes from with the name it brings in, and at varying widths neither reads as a column until the keyword between them lines up.

The rule reads each block as the run of consecutive imports at the same indentation. A blank line, an own-line comment, or a statement of another kind ends the run. [[alphabetize-siblings]] sorts the entries within each block and [[space-statements]] separates the sections by category, both before this rule measures a column. [[bare-imports]] reports on the choice between a bare and a `from` import without rewriting, so its finding is advice to the author rather than an input to this rule. The [**Pipeline Order**](/reference/pipeline-order) reference lists where each runs.

<template #configuration>

<RuleConfigTable />

`max-shift` limits how much padding one `import` keyword may take. The rule reads each block of imports in source order and extends a column while the gap between the widest and narrowest module names stays within the limit, starting a new column at the first import that would exceed it. Setting `max-shift` to `false` removes the limit, so a block of any width aligns on one column, and `0` forbids padding altogether. The [**per-rule facets**](/reference/configuration#per-rule-facets) reference covers the full semantics.

</template>

</RuleLayout>
