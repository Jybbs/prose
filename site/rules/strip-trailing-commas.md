---
category : auto-fix
family   : formatting
caption  : "removes trailing commas from collections, signatures, calls, and every other bracketed container."
related  : [collection-layout, align-colons]
layout   : doc
---

# strip-trailing-commas

<RuleLayout rule="strip_trailing_commas">

A trailing comma on the last entry of a multi-line collection adds a small **visual hiccup** at every block boundary without earning its keep. Each entry already has its own line, so a new entry adds a new line of its own, which leaves the trailing comma on the previous last entry with no diff-stability win to bring. `strip-trailing-commas` removes the trailing comma from any bracketed container that carries one, and it leaves tuples alone because Python uses the trailing comma to disambiguate single-element tuples from parenthesized expressions.

The rule walks every bracketed container (*dictionaries, lists, sets, function signatures, function calls, class bases, parenthesized argument lists*) and strips the comma after the last entry when one is present. Whether the container spans one line or many doesn't affect the strip itself, since single-line atomic collections happen not to carry trailing commas in idiomatic Python and the rule rarely fires on them. Pair with [[collection-layout]] for the multi-line expansion that brings the trailing comma into reach in the first place.

<template #configuration>

<RuleConfigTable />

The strip is unconditional within the contexts named above, so the rule carries `enabled` as its only knob. Tuple literals are exempt by construction because Python uses the trailing comma to disambiguate single-element tuples from parenthesized expressions, leaving no project-level switch to flip on the tuple carve-out.

</template>

<template #related-after>

For per-line opt-outs *(projects that prefer the trailing comma for diff stability even on multi-line forms)*, [**Suppression**](/usage/suppression) covers the `# fmt: off` / `# fmt: on` block markers.

</template>

</RuleLayout>
