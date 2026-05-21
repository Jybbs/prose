---
category : auto-fix
family   : formatting
caption  : "*Prose* splits list, tuple, dict, and set literals into one-entry-per-line layout once they cross a threshold."
related  : [align-colons, alphabetize, singleton-rule, strip-trailing-commas]
---

# collection-layout

A dictionary, list, or set with five non-trivial entries on one line reads as a **single chunky token**, and the reader's eye flicks across to find the entry it wants. The same data on five separate lines reads as a **column of entries**, each one a unit. `collection-layout` expands multi-entry collections to the one-per-line shape whenever the entries cross the atomicity threshold, and it leaves short single-line collections alone when each entry is already small enough to skim.

The rule fires on dictionary, list, set, and tuple literals. A literal expands when any entry is non-atomic (*a function call, a nested collection, a computed expression*) or when the entry count exceeds `max-atomics-per-line`. Single-line collections of atomic literals (*ints, floats, strings, single-name identifiers*) inside the cap stay on one line. Pair with [[align-colons]] for the dict-key alignment after the expansion, with [[alphabetize]] for sibling sorting where ordering doesn't matter, and with [[strip-trailing-commas]] for the trailing-comma sweep on the multi-line form.

## Configuration

| Key | Type | Default | Meaning |
|---|---|---|---|
| `enabled` | bool | `true` | Toggle the rule on or off |
| `max-atomics-per-line` | positive int | `8` | Keep short collections on one line when each entry is an atomic literal and the run fits the cap |

A short tuple inside a function-call argument list, like `numpy.zeros((3, 4))`, stays inline at the default cap. A `dict` literal with eight non-atomic entries expands regardless of length.

## The Canonical Case

A dict literal with non-atomic entries expands to one entry per line, and the reader reads the entries as a column of key-value pairs.

<Fixture rule="collection_layout" case="dict_literal" />

## More Examples

<Fixture rule="collection_layout" case="list_literal" title="Lists Follow the Same Expansion Threshold as Dicts" />

<Fixture rule="collection_layout" case="atomic_kinds" title="Atomic Entries inside the Cap Stay on One Line" />

<Fixture rule="collection_layout" case="nested" title="Nested Collections Expand Level by Level Independently" />

<Fixture rule="collection_layout" case="matrix" title="A Matrix Literal Expands Each Row Independently" />

<Fixture rule="collection_layout" case="comprehensions" title="Comprehensions Stay on One Line When They Fit" />

<Fixture rule="collection_layout" case="idempotent" title="Already-Expanded Source Is Left Alone" />

## Related

<RelatedRulesInline />
