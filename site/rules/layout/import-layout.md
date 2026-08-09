---
caption : "Lays an import block out one module per line, gathering each module's members behind it and packing an over-budget roster to the import budget."
related : [align-imports, alphabetize, bare-imports, collection-layout, group-imports, shed-backslash-continuations, signature-layout]
layout  : doc
---

# import-layout

<RuleLayout rule="import_layout">

`import-layout` gives a long from-import a deliberate shape once it overflows the dedicated import budget, rewriting it into a run of `from ... import ...` statements. Each statement repeats the module prefix and greedily packs as many alphabetized names as fit before the next line opens, so the imported names begin at the column the eye reaches after `import` on every line and a deep module path never drives them rightward.

Two further facets answer the same question about what one import line holds. `split-multi-module` breaks a comma-joined `import a, b` into one `import` statement per module, the form pycodestyle flags as `E401`, since those commas separate distinct modules and nothing binds them to one line. `merge-members` runs the other direction on `from`-imports, gathering every `from pkg import ...` statement of one module in an import run onto a single line carrying each member once, so the module appears once with its roster behind it. A `from pkg import a, b` line is never broken at its commas, because those separate members of one module rather than modules.

The rule runs ahead of [[group-imports]] and [[alphabetize]], so each module it puts on its own line reaches its canonical group and slot in the same pass, and the gathered roster lands in the order [[alphabetize]] would leave it. Setting `alphabetize = false` holds the authored member order across both moves.

The rule acts on single-line imports that open their own line. A `from ... import *`, a from-import already within budget, a `;`-joined statement, and a parenthesized multi-line import stay untouched, and a lone name whose own line still overflows keeps its place rather than splitting further. A backslash-continued import arrives here already rejoined, since [[shed-backslash-continuations]] sheds the escape well ahead of it, so every move below reads the single line that rejoin produced. A comment anywhere across the lines a gather would clear holds it back, since folding those statements together would leave the comment describing nothing, and a notebook's cell boundary holds a gather the same way. Pair with [[align-imports]] to align the `import` keyword across the resulting run, which already carries one identical prefix per line.

<template #configuration>

<RuleConfigTable />

Each shape move sits behind its own facet, so a project can switch one off without disturbing the others. `split-multi-module` governs the comma-joined break and `merge-members` the same-module gather, both defaulting on, and the width split runs regardless of either.

The wrap budget comes from the top-level [`import-line-length`](/reference/configuration#top-level-keys) key *(default `120`)*, governing the import wrap independently of `code-line-length`. An import is a roster [[alphabetize]] already orders, so it stays scannable at a width where dense expression code would not, earning more horizontal room before a wrap pays off. Setting `import-line-length` to `false` drops the dedicated budget, so the import wrap falls back to `code-line-length`.

</template>

</RuleLayout>
