---
caption : "Splits list, tuple, dict, and set literals into one-entry-per-line layout once they overflow their width, or a dict crosses an entry-count cap."
related : [align-colons, alphabetize, signature-layout, strip-trailing-commas]
layout  : doc
---

# collection-layout

<RuleLayout rule="collection_layout">

A dictionary, list, or set with five non-trivial entries on one line reads as a **single chunky token**, and the reader's eye flicks across to find the entry it wants. The same data on five separate lines reads as a **column of entries**, each one a unit. `collection-layout` expands multi-entry collections to the one-per-line shape whenever the entries cross the atomicity threshold, and it leaves short single-line collections alone when each entry is already small enough to skim.

The rule fires on dictionary, list, set, and tuple literals. A literal expands when any entry is non-atomic (*a function call, a nested collection, a computed expression*) or when the entry count exceeds `max-atomics-per-line`. Single-line collections of atomic literals (*ints, floats, strings, single-name identifiers*) inside the cap stay on one line. Pair with [[align-colons]] for the dict-key alignment after the expansion, with [[alphabetize]] for sibling sorting where ordering doesn't matter, and with [[strip-trailing-commas]] for the trailing-comma sweep on the multi-line form.

The rule runs the inverse move as well, joining a multi-line construct whose single-line form fits the budget back onto one line. The join reaches the four literals, a multi-line subscript such as `data[key]`, and a multi-line collection key inside a dict, so a tuple key split across lines rejoins and reads as the single clean member [[align-colons]] and [[alphabetize]] fold into their run. A construct that would overflow once joined keeps its source break, leaving the wrapped form for the cross-line guard.

A dict also expands once it holds more than `max-inline-dict-entries` entries, whatever its width, taking any enclosing collection with it. It mirrors [[signature-layout]]'s `max-inline-params`, the same count-gate shape applied to parameters. The trigger is dict-only, since a list or set reads acceptably as a packed run while a dict's key-value pairs earn the vertical layout. Set the knob to `false` to leave width as the only dict gate.

A dict entry whose `key: value` width overflows the budget at the item-indent column breaks at `:` and hangs the value at `item_indent + INDENT_STEP`. The hang applies per-row, so a multi-item dict hangs only the rows that need it. A single-entry dict whose entry overflows enters the expand path and applies the same break. Tuples, lists, and sets stay out of the hang shape because their elements carry no `:` separator.

<template #configuration>

| Key | Type | Default | Meaning |
|---|---|---|---|
| `enabled` | bool | `true` | Toggle the rule on or off |
| `max-atomics-per-line` | positive int \| `false` | `8` | Keep short collections on one line when each entry is an atomic literal and the run fits the cap. Setting `false` removes the cap and packs each line by width alone |
| `max-inline-dict-entries` | positive int \| `false` | `3` | Expand a dict once its entry count exceeds the cap, whatever its width. Setting `false` disables the count trigger and leaves width as the only dict gate |

A short tuple inside a function-call argument list, like `numpy.zeros((3, 4))`, stays inline at the default cap. A `dict` literal with eight non-atomic entries expands regardless of length. A four-entry `dict` expands at the default `max-inline-dict-entries` of `3` even when it fits the line.

</template>

</RuleLayout>
