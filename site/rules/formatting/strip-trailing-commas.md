---
caption : "Removes the trailing comma from a collection, signature, call, class base list, or type-parameter list, leaving tuples alone."
related : [reflow-collections, align-colons]
layout  : doc
---

# strip-trailing-commas

<RuleLayout rule="strip_trailing_commas">

`strip-trailing-commas` removes the comma after the last entry of any bracketed container that carries one, and leaves tuples alone, because Python uses the trailing comma to tell a single-element tuple from a parenthesized expression. A trailing comma on the last entry of a multi-line collection adds a small **visual hiccup** at every block boundary without earning its keep, in that each entry already has its own line, so a new entry adds a new line of its own and the comma on the previous last entry brings no diff-stability win.

The rule reads every bracketed container (*dictionaries, lists, sets, function signatures, function calls, class base lists, parenthesized argument lists, and the type-parameter list on a `def`, a `class`, or a `type` alias*) and removes the comma after the last entry when one is present. The strip applies whether the container spans one line or many, in that `f(a, b, c,)` loses its comma the same way a multi-line call does, though a single-line container rarely carries one in idiomatic Python. A comment between the last entry and the closing bracket is trivia the rule reads past, so the comma goes and the comment stays. Pair with [[reflow-collections]] for the multi-line expansion that puts the trailing comma in reach in the first place.

<template #configuration>

<RuleConfigTable />

The strip is unconditional within the contexts named above, so the rule carries `enabled` as its only facet. Tuple literals are exempt by construction, because Python uses the trailing comma to tell a single-element tuple from a parenthesized expression, leaving no project-level switch on the tuple carve-out.

</template>

<template #related-after>

For block-level opt-outs *(a project that keeps the trailing comma for diff stability even on multi-line forms)*, [**Suppression**](/usage/suppression) covers the `# fmt: off` / `# fmt: on` block markers.

</template>

</RuleLayout>
