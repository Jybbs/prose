---
caption : "Pads the space before `:` so consecutive dict entries, annotated assignments, signature annotations, and docstring entries share one column, with a docstring entry's parenthesized type in a column of its own."
related : [align-equals, align-imports, alphabetize-siblings, reflow-collections, align-match-case, strip-stranded-padding, wrap-docstrings]
layout  : doc
---

# align-colons

<RuleLayout rule="align_colons">

`align-colons` pads the space before `:` so consecutive entries share one column, and every construct that pairs a name with a value across a `:` then reads as two columns, names on the left and values on the right:

- Dictionary keys.
- Annotated assignments (*class fields, and module- and function-scope variables*).
- Function-signature parameter annotations.
- The `name: description` entries of every Google-style docstring section (*`Args:`, `Returns:`, `Raises:`, and the rest*).

A docstring entry with a parenthesized type gets a second column, its `(` one space past the widest name in the run, so the types read as a field of their own and the `:` column then sits past the widest `name (type)` pair. Both columns resolve in one pass, where the `:` column measures the widths the type-group padding produces rather than the ones the source wrote. Every aligned row keeps one space on each side of its `:`. Each docstring section resolves its own column, so a wide `Args:` entry never moves the `Returns:` table, and single-statement `match` arms likewise form a table of their own, one that [[align-match-case]] aligns.

The rule reads each context on its own, treating a group as the consecutive members at the same indentation under the same parent construct. A blank line, an own-line comment, or a statement of another kind ends the group. A one-member group takes no padding, since [[strip-stranded-padding]] strips whatever it carries, so a one-key dict reads as plain code rather than a one-row table. In the dict, annotation, and parameter contexts the column stays within `code-line-length`, so a row whose aligned line would cross the budget starts a new column rather than pushing its neighbors past the margin. A docstring section has no such cap, because [[wrap-docstrings]] runs directly after and rewraps each entry's description to `docstring-line-length` from the column the padding sets.

A `name (type):` head at the docstring body indent under no Title-case heading aligns the same way, because [[wrap-docstrings]] passes such a head through as written rather than joining it to the paragraph above. Each contiguous run of those heads resolves its own two columns, so prose or a blank line between two runs keeps one run's widths out of the other's column, whereas a head directly beneath a paragraph joins no run at all, because the wrap folds it into that paragraph. A `(` written flush against its name documents a call rather than a type, as in `divmod(self, other): the pair`, so no type column opens and the call keeps the form its author wrote, whereas its `:` still aligns with the run because the padding sits after the `)`. The rule reads the module's own docstring as well as every class and function docstring.

<template #configuration>

<RuleConfigTable />

`max-shift` limits how much padding one key may take. The rule reads each group of `:` entries in source order and extends a column while the gap between the widest and narrowest keys stays within the limit, starting a new column at the first key that would exceed it. Setting `max-shift` to `false` removes the limit, so a group of any width aligns on one column, and `0` forbids padding altogether, so every `:` sits flush against its key. The [**per-rule facets**](/reference/configuration#per-rule-facets) reference covers the full semantics.

</template>

</RuleLayout>
