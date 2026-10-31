---
caption : "Splits a `from … import …` that overflows `import-line-length` into repeated-prefix statements, breaks a comma-joined `import a, b` into one statement per module, and merges repeated `from` statements of one module into one line."
related : [align-imports, alphabetize-siblings, bare-imports, reflow-collections, group-imports, shed-backslash-continuations, reflow-signatures]
layout  : doc
---

# reflow-imports

<RuleLayout rule="reflow_imports">

`reflow-imports` splits a from-import that overflows `import-line-length` into a run of `from ... import ...` statements. Each statement repeats the module prefix and packs as many alphabetized names as fit before the next line opens, so the imported names start at the same column after `import` on every line and a deep module path never pushes them rightward. A `lazy` import splits the same way, each statement it writes opening on `lazy`.

The rule runs after [[group-imports]] and before [[alphabetize-siblings]], so each module [`split-multi-module`](#split-multi-module) moves onto a line of its own is placed in its import group in the same pass, and the member list [`merge-members`](#merge-members) writes follows the order [[alphabetize-siblings]] would leave it. Setting `alphabetize-siblings = false` keeps a merged list's members in the order written.

The rule acts on single-line imports that open their own line. A `from ... import *`, a from-import already within budget, a `;`-joined statement, and a parenthesized multi-line import stay as written, and a lone name whose own line still overflows keeps its place rather than splitting further. A backslash-continued import arrives here already rejoined, since [[shed-backslash-continuations]] removes the escape well ahead of it, so every move the rule makes reads the single line that rejoin produced.

Pair with [[align-imports]] to align the `import` keyword across the resulting run. The rule forecasts the block as the rules that run between it and [[align-imports]] will lay it out, with each merged member list folded into its lead statement and each run sorted and placed as [[alphabetize-siblings]] and [[band-constants]] leave it. A comment [[band-constants]] moves above a roster the rule splits leads the roster's first row, so every row of it sits in the run beneath the comment. Every row it writes therefore sits at the column [[align-imports]] settles that run to, and a second pass changes nothing.

<template #configuration>

<RuleConfigTable />

The top-level [`import-line-length`](/reference/configuration#top-level-keys) key *(default <ConfigDefault facet="import-line-length" />)* sets the import wrap's budget apart from `code-line-length`. Its default runs wider, because an import is a list of names [[alphabetize-siblings]] already sorts and stays scannable at a width where dense expression code would not.

</template>

<template #facets>

Each move sits behind its own facet, so a project can switch one off without touching the others, and the width split runs whatever either is set to.

### `merge-members`

`merge-members` folds the repeated `from pkg import ...` statements of one module onto a single line naming each member once. Each folded statement's line is removed with its blank lines the way [[prune-inert-imports]] removes an import. A `lazy from` statement folds only with another `lazy from` statement of its module, so a merge never changes when a member loads. A comment anywhere on the lines a merge would fold together blocks the merge, since folding those statements together would leave the comment describing nothing, and a notebook's cell boundary blocks a merge the same way. Under a class header carrying `# prose: keep`, a merge gathers only consecutive statements of one module, since a merge across another statement moves a member above it. Setting it to `false` keeps each statement as written, so a module named by three imports keeps its three lines.

### `split-multi-module`

`split-multi-module` breaks a comma-joined `import a, b` (*the form pycodestyle flags as `E401`*) into one `import` statement per module, and `false` keeps the comma-joined line as written. A `from pkg import a, b` line is never broken at its commas, because those commas separate members of one module rather than modules.

</template>

</RuleLayout>
