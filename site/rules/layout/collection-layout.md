---
caption : "Splits list, tuple, dict, and set literals into one-entry-per-line layout once they overflow their width, or a dict crosses an entry-count cap."
related : [align-colons, alphabetize, signature-layout, strip-trailing-commas]
layout  : doc
---

# collection-layout

<RuleLayout rule="collection_layout">

A dictionary, list, or set with five non-trivial entries on one line reads as a **single chunky token**, and the reader's eye flicks across to find the entry it wants. The same data on five separate lines reads as a **column of entries**, each one a unit. `collection-layout` expands multi-entry collections to the one-per-line shape whenever the entries cross the atomicity threshold, and it leaves short single-line collections alone when each entry is already small enough to skim.

The rule fires on dictionary, list, set, and tuple literals, a tuple expanding only when it carries its parentheses, since a bare comma tuple such as `return a, b, c` has no bracket pair to break on. A literal expands when any entry is non-atomic (*a function call, a nested collection, a computed expression*) or when the entry count exceeds `max-atomics`. Single-line collections of atomic literals (*ints, floats, strings, single-name identifiers*) inside the cap stay on one line. Pair with [[align-colons]] for the dict-key alignment after the expansion, with [[alphabetize]] for sibling sorting where ordering doesn't matter, and with [[strip-trailing-commas]] for the trailing-comma sweep on the multi-line form.

The rule runs the inverse move as well, joining a multi-line construct whose single-line form fits the budget back onto one line. The join reaches the four literals, a multi-line subscript such as `data[key]` or `matrix[row + step]`, a multi-line collection key inside a dict, and a comprehension or generator expression broken across its `for` and `if` clauses, so a tuple key split across lines rejoins and reads as the single clean member [[align-colons]] and [[alphabetize]] fold into their run. A subscript and a comprehension only ever join, never expanding the way a literal does, so a comprehension too wide to fit, or one carrying a comment or a multi-line string, keeps its source breaks. A construct that would overflow once joined, or whose subscript index carries a call or other member the single-line form cannot rejoin, keeps its source break for the cross-line guard.

A dict also expands once it holds more than `max-dict-entries` entries, whatever its width, taking any enclosing collection with it. It mirrors [[signature-layout]]'s `max-params`, the same count-gate shape applied to parameters. The trigger is dict-only, since a list or set reads acceptably as a packed run while a dict's key-value pairs earn the vertical layout. Set the facet to `false` to leave width as the only dict gate.

A dict entry whose `key: value` width overflows the budget at the item-indent column breaks at `:` and hangs the value at `item_indent + INDENT_STEP`. The hang applies per-row, so a multi-item dict hangs only the rows that need it. A single-entry dict whose entry overflows enters the expand path and applies the same break. Tuples, lists, and sets stay out of the hang shape because their elements carry no `:` separator.

Each shape move sits behind its own facet, so a project can switch one off without disturbing the others. `collapse` governs the inverse-join back to one line, `explode` governs every expansion *(the width-driven spread and the `max-dict-entries` count trigger alike, so `false` leaves the cap inert)*, and `wrap-dict-entries` governs the over-wide-entry break at `:`. Each defaults on, preserving the combined behavior above, and clearing one freezes that move while the others keep running.

<template #configuration>

| Key | Type | Default | Meaning |
|---|---|---|---|
| `enabled` | bool | `true` | Toggle the rule on or off |
| `collapse` | bool | `true` | Join a fitting multi-line literal, subscript, comprehension, or dict key back to one line. Setting `false` freezes those shapes where they sit |
| `explode` | bool | `true` | Expand an overflowing or over-count collection to one entry per line. Setting `false` suppresses every expansion and leaves `max-dict-entries` inert |
| `max-atomics` | positive int \| `false` | `8` | Keep short collections on one line when each entry is an atomic literal and the run fits the cap. Setting `false` removes the cap and packs each line by width alone |
| `max-dict-entries` | positive int \| `false` | `3` | Expand a dict once its entry count exceeds the cap, whatever its width. Setting `false` disables the count trigger and leaves width as the only dict gate |
| `wrap-dict-entries` | bool | `true` | Break an over-wide `key: value` at its `:` and hang the value beneath. Setting `false` leaves the oversized entry on one line |

A short tuple inside a function-call argument list, like `numpy.zeros((3, 4))`, stays inline at the default cap. A `dict` literal with eight non-atomic entries expands regardless of length. A four-entry `dict` expands at the default `max-dict-entries` of `3` even when it fits the line.

</template>

</RuleLayout>
