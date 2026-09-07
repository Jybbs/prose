---
caption : "Splits a `from … import …` that overflows `import-line-length` into repeated-prefix statements, breaks a comma-joined `import a, b` into one statement per module, and merges repeated `from` statements of one module into one line."
related : [align-imports, alphabetize-siblings, bare-imports, reflow-collections, group-imports, shed-backslash-continuations, reflow-signatures]
layout  : doc
---

# reflow-imports

<RuleLayout rule="reflow_imports">

`reflow-imports` splits a from-import that overflows `import-line-length` into a run of `from ... import ...` statements. Each statement repeats the module prefix and packs as many alphabetized names as fit before the next line opens, so the imported names start at the same column after `import` on every line and a deep module path never pushes them rightward.

Two further facets change what one import line carries. `split-multi-module` breaks a comma-joined `import a, b`, the form pycodestyle flags as `E401`, into one `import` statement per module, since those commas separate distinct modules and nothing ties them to one line. `merge-members` runs the other way on `from`-imports, merging every `from pkg import ...` statement of one module within an import run onto a single line that names each member once, so the module appears once with its members after it. A `from pkg import a, b` line is never broken at its commas, because those commas separate members of one module rather than modules.

The rule runs after [[group-imports]] and before [[alphabetize-siblings]], so each module it splits onto its own line is placed in its import group in the same pass, and the merged member list is written in the order [[alphabetize-siblings]] would leave it. Setting `alphabetize-siblings = false` keeps the authored member order across both moves.

The rule acts on single-line imports that open their own line. A `from ... import *`, a from-import already within budget, a `;`-joined statement, and a parenthesized multi-line import stay as written, and a lone name whose own line still overflows keeps its place rather than splitting further. A backslash-continued import arrives here already rejoined, since [[shed-backslash-continuations]] removes the escape well ahead of it, so every move the rule makes reads the single line that rejoin produced. A comment anywhere on the lines a merge would fold together blocks the merge, since folding those statements together would leave the comment describing nothing, and a notebook's cell boundary blocks a merge the same way.

Pair with [[align-imports]] to align the `import` keyword across the resulting run. The rule forecasts the block as the rules that run between it and [[align-imports]] will lay it out, with each merged member list folded into its lead statement and each run sorted and placed as [[alphabetize-siblings]] and [[band-constants]] leave it. Every row it writes therefore sits at the column [[align-imports]] settles that run to, and a second pass changes nothing.

<template #configuration>

<RuleConfigTable />

Each move sits behind its own facet, so a project can switch one off without touching the others. `split-multi-module` gates the comma-joined break and `merge-members` the same-module merge, both on by default, and the width split runs whatever either is set to.

The wrap budget comes from the top-level [`import-line-length`](/reference/configuration#top-level-keys) key *(default <ConfigDefault facet="import-line-length" />)*, which governs the import wrap independently of `code-line-length`. An import is a list of names [[alphabetize-siblings]] already sorts, so it stays scannable at a width where dense expression code would not, which is why it gets more horizontal room before a wrap pays off. Setting `import-line-length` to `false` drops the dedicated budget, so the import wrap falls back to `code-line-length`.

</template>

</RuleLayout>
