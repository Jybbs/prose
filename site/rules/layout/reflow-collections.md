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

A literal the author already laid out as a bracketed column, its opening bracket ending a line and its closing bracket opening one, is the exception to the rejoin, because its breaks already sit at entry boundaries. `keep-multiline-literals`, on by default, keeps that column along with the `:` alignment [[align-colons]] pads, and holding one needs two or more entries. A construct enclosing a kept literal has no one-line form either, so it keeps its break as well, whereas any other break rejoins.

A member the expansion keeps as written rather than laying out moves whole into the expanded form, its continuation rows shifted to the column the entries sit at, so a kept call or subscript reads under its siblings. A member whose rows align under its own opening bracket keeps the whole construct as written instead, and one running through a multi-line string keeps the string's own columns.

A dict expands once it has more than `max-dict-entries` entries whatever its width, and every collection enclosing it expands with it.

Every width the rule reads counts the separator closing an entry's row at the position [[alphabetize-siblings]] leaves it in, on the rejoin as well as the expansion. An entry the sort moves last sheds the comma it carries and one the sort moves up gains one before either is measured, so the layout the rule picks stays put once the sort is written. Each construct is then measured at the column it settles at:

1. A literal written on one row is measured at the width [[strip-stranded-padding]] settles it to, past the padding inside its brackets and at one space after each `:`, which is the width a rejoin writes it back at.
2. A member the expansion moves keeps the calls inside it measured at the columns its rows end up on, and a call the move pushes past the budget explodes in the same pass.
3. A literal following one the rule expands on the same line is measured where that expansion leaves it, on the closer's row at the statement's indent rather than under the continuation column the source wrote.
4. A dict value whose key the rule lays across rows is measured from the key's last row.

<template #facets>

Each move sits behind its own facet, whereas the rejoin has none.

### `explode`

`explode` gates the expansion itself, covering the count trigger as well as the width one. At `false` every expansion stops and the count cap has nothing left to act on, whereas the rejoin runs either way.

### `keep-multiline-literals`

`keep-multiline-literals` keeps a literal the author wrote as a bracketed column of two or more entries, re-expanding it to the canonical layout rather than joining it back. Setting it to `false` joins one onto a single line wherever it fits the budget, and any other multi-line layout rejoins either way.

### `max-atomics`

`max-atomics` caps how many atomic entries one packed row of an expanded collection carries, defaulting to <ConfigDefault rule="reflow-collections" facet="max-atomics" />, and `false` removes the cap so each row packs by width alone.

### `max-dict-entries`

`max-dict-entries` expands a dict once its entry count passes the cap, whatever its width, so a four-entry `dict` expands at the default <ConfigDefault rule="reflow-collections" facet="max-dict-entries" /> even when it fits the line. The cap reads dicts alone, which is why a short tuple inside a call's argument list, like `numpy.zeros((3, 4))`, stays inline, whereas at `false` the width trigger becomes the only one a dict meets.

### `wrap-dict-entries`

`wrap-dict-entries` breaks an over-wide `key: value` at its `:` and hangs the value one indent step in, row by row rather than across the whole literal, a layout only a dict takes. Setting it to `false` leaves the oversized entry on one line.

</template>

</RuleLayout>
