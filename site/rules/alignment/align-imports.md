---
caption : "Aligns the `import` and `as` keywords across consecutive import statements."
related : [align-colons, align-equals, alphabetize, bare-imports, blank-lines, align-match-case]
layout  : doc
---

# align-imports

<RuleLayout rule="align_imports">

An import block carries two kinds of structure that the reader's eye wants to follow as columns. The module column says where a thing comes from, and the name column says what's pulled in. When the two columns float at varying widths, every line reads as a fresh sentence rather than a row in a table. `align-imports` gathers consecutive `from ... import ...` statements (*or consecutive `import ... as ...` statements*) into a shared column for the `import` (*or `as`*) keyword, leaving the module column flush left and the name column flush right.

The rule reads each block as the run of consecutive imports at the same indentation. A blank line, a comment, or a non-import statement resets the run. Pair with [[alphabetize]] to sort entries within each block before alignment, with [[blank-lines]] to separate import groups by category, and with [[bare-imports]] to canonicalize bare-versus-from before the alignment pass.

<template #configuration>

<RuleConfigTable />

`max-shift` bounds how far the `import` keyword may shift to align. The rule walks each block of imports in source order and grows a column while its width spread stays within the cap, breaking a fresh column at the first import that would exceed it. A `max-shift` of `false` lifts the cap so a contiguous block folds into one column, and `0` forbids any shift. The [**per-rule facets**](/reference/configuration#per-rule-facets) reference covers the full semantics.

</template>

</RuleLayout>
