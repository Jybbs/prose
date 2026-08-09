---
caption : "Splits list, tuple, dict, and set literals into one-entry-per-line layout once they overflow their width, or a dict crosses an entry-count cap."
related : [align-colons, alphabetize, signature-layout, strip-trailing-commas]
layout  : doc
---

# collection-layout

<RuleLayout rule="collection_layout">

`collection-layout` expands a multi-entry collection to one entry per line once its entries cross the atomicity threshold, leaving a short collection of small entries on its row. It reaches dictionary, list, set, and tuple literals, a tuple only where it carries its parentheses, since `return a, b, c` has no bracket pair to break on.

A literal expands when any entry is non-atomic (*a function call, a nested collection, a computed expression*) or when the entry count passes `max-atomics`. A single-line run of atomic literals (*ints, floats, strings, single-name identifiers*) inside that cap stays where it is.

The rule runs the inverse move too, rejoining a construct fractured somewhere other than an entry boundary. That reaches a multi-line subscript such as `data[key]`, a multi-line collection key inside a dict, and a comprehension broken across its `for` and `if` clauses. A subscript and a comprehension only ever rejoin, so one too wide to fit, or carrying a comment or multi-line string, keeps its breaks. A construct that would overflow once joined keeps its break, as does one whose nested call holds a flush column, carries a comment, or runs past `max-args`, and a collection inside an f-string replacement field is opaque whatever its width.

A literal the author laid out as a flush bracketed column is the exception, its breaks already sitting at the entry boundaries the one-per-line shape is built from. `keep-multiline-literals`, on by default, holds that literal multi-line and re-expands it to the canonical layout rather than joining it back, so the column survives along with the `:` alignment [[align-colons]] pads. The hold needs two or more entries and reaches outward, and a break falling anywhere else reads as a fracture and rejoins.

A dict expands once it holds more than `max-dict-entries` entries whatever its width, taking any enclosing collection with it. The trigger is dict-only, a list or set reading acceptably as a packed run.

A dict entry whose `key: value` width overflows the budget breaks at the `:` and hangs its value one indent step in, per row rather than across the literal. Tuples, lists, and sets carry no `:` to break on.

Each move sits behind its own facet, `explode` reaching the count trigger as well as the width one, whereas fracture repair sits behind no facet at all.

<template #configuration>

<RuleConfigTable />

A short tuple inside a function-call argument list, like `numpy.zeros((3, 4))`, stays inline at the default cap. A `dict` literal with eight non-atomic entries expands regardless of length. A four-entry `dict` expands at the default `max-dict-entries` of `3` even when it fits the line.

</template>

</RuleLayout>
