---
caption : "Aligns the `:` separator across dict literals, annotated assignments, function-signature annotations, and Google-style docstring sections."
related : [align-equals, align-imports, alphabetize, collection-layout, align-match-case, strip-align-padding, docstring-wrap]
layout  : doc
---

# align-colons

<RuleLayout rule="align_colons">

The `:` separator appears across the contexts below, wherein columns of values sit beside columns of names and the reader's eye wants a tidy table rather than a ragged margin. `align-colons` gathers those contexts into a single shared alignment surface, so dictionary keys, annotated assignments (*class fields alongside module- and function-scope variables*), function-signature parameter annotations, and the `name: description` entries of every Google-style docstring section (*`Args:`, `Returns:`, `Raises:`, and the rest*) all read as parallel two-column entries. Each aligned row keeps one space on each side of its `:`, and each docstring section resolves its own column so a wide `Args:` entry never shifts the `Returns:` table. Single-expression `match` arms live in a separate dispatch table owned by [[align-match-case]].

The rule walks each context independently, treating a group as the consecutive members sharing the same indentation level and parent shape. A blank line, an own-line comment, or a non-member statement resets the group. Alignment honors the [[strip-align-padding]] so that one-member contexts skip padding altogether, leaving a one-key dict reading as plain code instead of a one-row table. The dict, annotation, and parameter contexts resolve their columns within `code-line-length`, so a row whose aligned line would cross the budget breaks a fresh column rather than dragging its neighbors past the margin. A docstring section carries no such cap, because [[docstring-wrap]] runs immediately after and reflows each entry's description to `docstring-line-length` from the column the padding lands on.

<template #configuration>

<RuleConfigTable />

`max-shift` bounds how far a key may shift to align. The rule walks each group of `:` entries in source order and grows a column while its width spread stays within the cap, breaking a fresh column at the first key that would exceed it. A `max-shift` of `false` lifts the cap so a contiguous group folds into one column, and `0` forbids any shift so every `:` sits flush. The [**per-rule facets**](/reference/configuration#per-rule-facets) reference covers the full semantics.

</template>

</RuleLayout>
