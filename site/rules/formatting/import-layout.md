---
caption : "Splits an over-budget `from ... import ...` into a run of repeated-prefix statements packed to the import budget."
related : [align-imports, alphabetize, bare-imports, collection-layout, signature-layout]
layout  : doc
---

# import-layout

<RuleLayout rule="import_layout">

*Prose* holds code to a line-length budget but leaves imports exempt by default, so a `from a.deeply.nested.module import x, y, z, ...` runs past the margin however wide its roster grows. `import-layout` gives a long from-import a deliberate shape once it overflows a dedicated import budget, rewriting it into a run of `from ... import ...` statements. Each statement repeats the module prefix and greedily packs as many alphabetized names as fit before the next line opens, so the imported names begin at the column the eye reaches after `import` on every line and a deep module path never drives them rightward.

The rule acts on a single-line `from ... import ...` only. A `from ... import *`, a from-import already within budget, and a multi-line (*parenthesized or backslash-continued*) import stay untouched, and a lone name whose own line still overflows keeps its place rather than splitting further. Pair with [[alphabetize]] to sort each roster before the split packs it, and with [[align-imports]] to align the `import` keyword across the resulting run, which already carries one identical prefix per line.

<template #configuration>

| Key | Type | Default | Meaning |
|---|---|---|---|
| `enabled` | bool | `true` | Toggle the rule on or off |

The wrap budget comes from the top-level [`import-line-length`](/reference/configuration#top-level-keys) key *(default `120`)*, governing the import wrap independently of `code-line-length`. An import is a roster [[alphabetize]] already orders, so it stays scannable at a width where dense expression code would not, earning more horizontal room before a wrap pays off. Setting `import-line-length` to `false` drops the dedicated budget, so the import wrap falls back to `code-line-length`.

</template>

</RuleLayout>
