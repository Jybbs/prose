---
caption : "Splits list, tuple, dict, and set literals into one-entry-per-line layout once they overflow their width, or a dict crosses an entry-count cap."
related : [align-colons, alphabetize, signature-layout, strip-trailing-commas]
layout  : doc
---

# collection-layout

<RuleLayout rule="collection_layout">

A dictionary, list, or set with five non-trivial entries on one line reads as a **single chunky token**, and the reader's eye flicks across to find the entry it wants. The same data on five separate lines reads as a **column of entries**, each one a unit. `collection-layout` expands multi-entry collections to the one-per-line shape whenever the entries cross the atomicity threshold, and it leaves short single-line collections alone when each entry is already small enough to skim.

The rule fires on dictionary, list, set, and tuple literals, a tuple expanding only when it carries its parentheses, since a bare comma tuple such as `return a, b, c` has no bracket pair to break on. A literal expands when any entry is non-atomic (*a function call, a nested collection, a computed expression*) or when the entry count exceeds `max-atomics`. Single-line collections of atomic literals (*ints, floats, strings, single-name identifiers*) inside the cap stay on one line. Pair with [[align-colons]] for the dict-key alignment after the expansion, with [[alphabetize]] for sibling sorting where ordering doesn't matter, and with [[strip-trailing-commas]] for the trailing-comma sweep on the multi-line form.

The rule runs the inverse move as well, rejoining a fractured construct whose single-line form fits the budget. The repair reaches a multi-line subscript such as `data[key]` or `matrix[row + step]`, a multi-line collection key inside a dict, and a comprehension or generator expression broken across its `for` and `if` clauses, so a tuple key split across lines rejoins and reads as the single clean member [[align-colons]] and [[alphabetize]] fold into their run. None of those shapes carries an entry boundary for a break to sit on, so the repair runs whatever the facets hold. A subscript and a comprehension only ever rejoin, never expanding the way a literal does, so a comprehension too wide to fit, or one carrying a comment or a multi-line string, keeps its source breaks. A construct that would overflow once joined, or whose subscript index carries a member the single-line form cannot rejoin, keeps its source break for the cross-line guard. A call the author hand-wrapped is not such a member, since its fracture closes the same way [[call-layout]] closes it, whereas a call holding a flush column, one carrying a comment, and one past `max-args` each keep a break the enclosing join then reads. A collection inside an f-string or t-string replacement field is opaque the same way, holding its source shape whatever its width, and a dict there trips the count cap neither for itself nor for the collection enclosing the literal.

A literal the author laid out as a flush bracketed column is a different case, because its breaks sit at the entry boundaries the one-per-line shape is built from. `keep-multiline-literals`, on by default, holds that literal multi-line and re-expands it to the canonical layout rather than joining it back, so the vertical column the author laid down survives along with the `:` alignment [[align-colons]] pads. A held literal keeps its break inside any enclosing repair too, so a subscript standing on a columnar dict finds no single-line form to join to. The hold needs two or more entries, since a column of one carries no `:` to align and reads no differently from the single row it rejoins to. A break falling anywhere else, such as a soft wrap that leaves the opening bracket sharing its line, reads as a fracture and rejoins whatever the facet holds, and clearing the facet restores the join for the columnar case as well. A member the expansion holds rather than lays out itself travels with the row it lands on, its continuation rows moving to the item column, so a held call or subscript reads under its siblings rather than at the column the source left it. A member whose rows align under its own opening bracket, and one running through a multi-line string, each keep their own columns instead.

A dict also expands once it holds more than `max-dict-entries` entries, whatever its width, taking any enclosing collection with it. It mirrors [[signature-layout]]'s `max-params`, the same count-gate shape applied to parameters. The trigger is dict-only, since a list or set reads acceptably as a packed run while a dict's key-value pairs earn the vertical layout. Set the facet to `false` to leave width as the only dict gate.

A dict entry whose `key: value` width overflows the budget at the item-indent column breaks at `:` and hangs the value at `item_indent + INDENT_STEP`. The hang applies per-row, so a multi-item dict hangs only the rows that need it. A single-entry dict whose entry overflows enters the expand path and applies the same break. Tuples, lists, and sets stay out of the hang shape because their elements carry no `:` separator.

Each shape move sits behind its own facet, so a project can switch one off without disturbing the others. `keep-multiline-literals` governs whether an authored multi-line literal holds that shape, `explode` governs every expansion *(the width-driven spread and the `max-dict-entries` count trigger alike, so `false` leaves the cap inert)*, and `wrap-dict-entries` governs the over-wide-entry break at `:`. Each defaults on, preserving the combined behavior above, and clearing one drops that move while the others keep running. Fracture repair sits behind no facet at all, since rejoining a construct broken somewhere other than an entry boundary is correct rendering rather than a preference.

<template #configuration>

<RuleConfigTable />

A short tuple inside a function-call argument list, like `numpy.zeros((3, 4))`, stays inline at the default cap. A `dict` literal with eight non-atomic entries expands regardless of length. A four-entry `dict` expands at the default `max-dict-entries` of `3` even when it fits the line.

</template>

</RuleLayout>
