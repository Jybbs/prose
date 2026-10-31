---
caption : "Pads the space before the `import` keyword across consecutive `from` imports, or before `as` across consecutive aliased imports, so the keywords share one column."
related : [align-colons, align-equals, alphabetize-siblings, bare-imports, space-statements, align-match-case]
layout  : doc
---

# align-imports

<RuleLayout rule="align_imports">

`align-imports` pads the space before the `import` keyword across consecutive `from … import …` statements (*or before `as` across consecutive `import … as …` statements*) so the keyword sits at one column, leaving the module names flush left and the imported names starting at one column to the right. An import block pairs the module a name comes from with the name it brings in, and at varying widths neither reads as a column until the keyword between them lines up.

The rule reads each block as the run of consecutive imports at the same indentation. A blank line, an own-line comment, or a statement of another kind ends the run. [[alphabetize-siblings]] sorts the entries within each block and [[space-statements]] separates the sections by category, both before this rule measures a column. A `from` import or an aliased import with no import of its own kind next to it takes no column and keeps whatever padding its source writes.

<template #configuration>

<RuleConfigTable />

Under `max-shift = 0`, every `import` and `as` the rule aligns sits one space after the name before it.

</template>

</RuleLayout>
