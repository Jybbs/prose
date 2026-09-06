---
caption : "Expands a list, tuple, dict, or set literal across lines once it overflows `code-line-length` or a dict passes `max-dict-entries`, and rejoins a construct broken anywhere but an entry boundary."
related : [align-colons, alphabetize-siblings, reflow-signatures, strip-trailing-commas]
layout  : doc
---

# reflow-collections

<RuleLayout rule="reflow_collections">

`reflow-collections` expands a multi-entry collection literal across lines once its single-line form overflows `code-line-length`, and leaves a literal that fits on its row. It reaches dict, list, set, and tuple literals, a tuple only where it carries its parentheses.

Inside the expanded form, a non-atomic entry (*a function call, a nested collection, a computed expression*) takes a row of its own, whereas a run of atomic entries (*ints, floats, strings, single-name identifiers*) packs across as few rows as fit, each row taking at most `max-atomics` entries within the budget. A non-atomic entry in the middle of such a run splits it into two runs packed on their own.

The rule runs the inverse move too, rejoining a construct broken somewhere other than an entry boundary, and the rejoin reaches a multi-line subscript, a collection used as a dict key, and a comprehension broken across its `for` and `if` clauses. A subscript and a comprehension only ever rejoin and never expand the way a literal does, so one too wide to fit, or one carrying a comment or a multi-line string, keeps its breaks. A construct that would overflow once joined keeps its break, as does one whose nested call is already written one argument per line, carries a comment, or passes `max-args`. A collection inside an f-string replacement field is left as written whatever its width.

A literal the author already laid out as a bracketed column, its opening bracket ending a line and its closing bracket opening one, is the exception to the rejoin, because its breaks already sit at entry boundaries. `keep-multiline-literals`, on by default, keeps it multi-line and re-expands it to the canonical layout rather than joining it back, so the column survives along with the `:` alignment [[align-colons]] pads. Keeping a column this way needs two or more entries. A construct enclosing a kept literal has no one-line form either, so it keeps its break as well, whereas any other break rejoins.

A member the expansion keeps as written rather than laying out moves whole into the expanded form, its continuation rows shifted to the column the entries sit at, so a kept call or subscript reads under its siblings. A member whose rows align under its own opening bracket keeps the whole construct as written instead, and one running through a multi-line string keeps the string's own columns.

A dict expands once it has more than `max-dict-entries` entries whatever its width, and every collection enclosing it expands with it.

A dict entry whose `key: value` width overflows the budget breaks at the `:` and hangs its value one indent step in, row by row rather than across the whole literal, a layout only a dict takes. Setting `wrap-dict-entries` to `false` leaves such an entry on one line.

Every width the rule reads counts the separator closing an entry's row at the position [[alphabetize-siblings]] leaves it in, on the rejoin as well as the expansion. An entry the sort moves last sheds the comma it carries and one the sort moves up gains one before either is measured, so the layout the rule picks stays put once the sort is written. Each construct is then measured at the column it settles at:

1. A literal written on one row is measured at the width [[strip-stranded-padding]] settles it to, past the padding inside its brackets and at one space after each `:`, which is the width a rejoin writes it back at.
2. A member the expansion moves keeps the calls inside it measured at the columns its rows end up on, and a call the move pushes past the budget explodes in the same pass.
3. A literal following one the rule expands on the same line is measured where that expansion leaves it, on the closer's row at the statement's indent rather than under the continuation column the source wrote.
4. A dict value whose key the rule lays across rows is measured from the key's last row.

Each move sits behind its own facet, `explode` gating the count trigger as well as the width one, whereas the rejoin has none.

<template #configuration>

<RuleConfigTable />

A short tuple inside a call's argument list, like `numpy.zeros((3, 4))`, stays inline, since it fits the budget and `max-dict-entries` reads dicts alone. A `dict` literal with eight non-atomic entries expands whatever its length. A four-entry `dict` expands at the default `max-dict-entries` of `3` even when it fits the line.

</template>

</RuleLayout>
