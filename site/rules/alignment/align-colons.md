---
caption : "Aligns the `:` separator across dict literals, annotated assignments, function-signature annotations, and docstring entries, whose parenthesized type groups take a column of their own."
related : [align-equals, align-imports, alphabetize-siblings, reflow-collections, align-match-case, strip-stranded-padding, wrap-docstrings]
layout  : doc
---

# align-colons

<RuleLayout rule="align_colons">

The `:` separator sets a column of values beside a column of names wherever it appears. `align-colons` gathers those contexts into a single shared alignment surface, so dictionary keys, annotated assignments (*class fields alongside module- and function-scope variables*), function-signature parameter annotations, and the `name: description` entries of every Google-style docstring section (*`Args:`, `Returns:`, `Raises:`, and the rest*) all read as parallel two-column entries. A docstring entry naming a parenthesized type carries a second column, its `(` seated one space past the widest name beside it, so the type groups read as their own field and the `:` column then settles past the widest `name (type)` pair. Both columns resolve in the same pass, so the `:` measures the widths the type-group padding leaves rather than the ones the source wrote. Each aligned row keeps one space on each side of its `:`, and each docstring run resolves its own column so a wide `Args:` entry never shifts the `Returns:` table. Single-expression `match` arms live in a separate dispatch table owned by [[align-match-case]].

The rule walks each context independently, treating a group as the consecutive members sharing the same indentation level and parent shape. A blank line, an own-line comment, or a non-member statement resets the group. Alignment honors the [[strip-stranded-padding]] so that one-member contexts skip padding altogether, leaving a one-key dict reading as plain code instead of a one-row table. The dict, annotation, and parameter contexts resolve their columns within `code-line-length`, so a row whose aligned line would cross the budget breaks a fresh column rather than dragging its neighbors past the margin. A docstring section carries no such cap, because [[wrap-docstrings]] runs immediately after and reflows each entry's description to `docstring-line-length` from the column the padding lands on.

A `name (type):` head standing at the docstring body indent under no Title-case heading aligns the same way, since [[wrap-docstrings]] passes such a head through verbatim rather than folding it into the paragraph above it. Each contiguous run of those heads resolves its own two columns, so prose or a blank line between two runs keeps one run's widths off the other's column, whereas a head sitting directly beneath prose joins no run at all because the wrap reflows it into that paragraph. A `(` written flush against its name documents a call rather than a type, the way `divmod(self, other): the pair` does, so no type column opens and the call keeps the shape its author wrote, whereas its `:` still settles with the run because padding after the `)` leaves the call untouched. The walk reaches the module's own docstring alongside every class and function one.

<template #configuration>

<RuleConfigTable />

`max-shift` bounds how far a key may shift to align. The rule walks each group of `:` entries in source order and grows a column while its width spread stays within the cap, breaking a fresh column at the first key that would exceed it. A `max-shift` of `false` lifts the cap so a contiguous group folds into one column, and `0` forbids any shift so every `:` sits flush. The [**per-rule facets**](/reference/configuration#per-rule-facets) reference covers the full semantics.

</template>

</RuleLayout>
