---
caption : "Splits list, tuple, dict, and set literals into one-entry-per-line layout once they overflow their width, or a dict crosses an entry-count cap."
related : [align-colons, alphabetize-siblings, reflow-signatures, strip-trailing-commas]
layout  : doc
---

# reflow-collections

<RuleLayout rule="reflow_collections">

`reflow-collections` expands a multi-entry collection to one entry per line once its entries cross the atomicity threshold, leaving a short collection of small entries on its row. It reaches dictionary, list, set, and tuple literals, a tuple only where it carries its parentheses.

A literal expands when any entry is non-atomic (*a function call, a nested collection, a computed expression*) or the entry count passes `max-atomics`, so a run of atomic literals (*ints, floats, strings, single-name identifiers*) inside that cap stays where it is.

The rule runs the inverse move too, rejoining a construct fractured somewhere other than an entry boundary, reaching a multi-line subscript, a collection key inside a dict, and a comprehension broken across its `for` and `if` clauses. A subscript and a comprehension only ever rejoin, so one too wide to fit, or carrying a comment or multi-line string, keeps its breaks. Neither expands the way a literal does. A construct overflowing once joined keeps its break, as does one whose nested call holds a flush column, carries a comment, or runs past `max-args`. A collection inside an f-string replacement field is opaque whatever its width.

A literal the author laid out as a flush bracketed column is the exception, its breaks already sitting at the entry boundaries. `keep-multiline-literals`, on by default, holds it multi-line and re-expands it to the canonical layout rather than joining it back, so the column survives along with the `:` alignment [[align-colons]] pads. The hold needs two or more entries and reaches outward, and any other break reads as a fracture and rejoins.

A member the expansion holds rather than lays out travels whole into the stacked form, its continuation rows moving to the item column, so a held call or subscript reads under its siblings. A member whose rows align under its own opening bracket holds the construct as written instead, and one running through a multi-line string keeps the string's own columns.

A dict expands once it holds more than `max-dict-entries` entries whatever its width, taking any enclosing collection with it.

A dict entry whose `key: value` width overflows the budget breaks at the `:` and hangs its value one indent step in, per row rather than across the literal. Only a dict carries that shape.

Every width the pass reads counts the separator closing an entry's row at the position [[alphabetize-siblings]] leaves it in, on the rejoin as well as the expansion, so an entry the sort moves last sheds the comma it carries and one the sort moves up gains one before either is measured, and the shape the pass picks holds once the sort lands. Each construct then measures at the column it settles at:

1. A literal written on one row measures at the width [[strip-stranded-padding]] settles it to, past the padding inside its brackets and at one space after each `:`, which is the width a rejoin writes it back at.
2. A member the expansion moves keeps the calls inside it measured at the columns its rows land on, exploding one the move pushes past the budget in the same pass.
3. A literal following one the pass expands on the same line measures where that expansion leaves it, on the closer's row at the statement's indent rather than under the continuation column the source wrote.
4. A dict value whose key the pass lays across rows measures from the key's last row.

Each move sits behind its own facet, `explode` reaching the count trigger as well as the width one, whereas fracture repair has none.

<template #configuration>

<RuleConfigTable />

A short tuple inside a function-call argument list, like `numpy.zeros((3, 4))`, stays inline at the default cap. A `dict` literal with eight non-atomic entries expands regardless of length. A four-entry `dict` expands at the default `max-dict-entries` of `3` even when it fits the line.

</template>

</RuleLayout>
