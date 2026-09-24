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

The rule reads each context on its own, treating a group as the consecutive members at the same indentation under the same parent construct. A blank line, an own-line comment, or a statement of another kind ends the group. A one-member group takes no padding, since [[strip-stranded-padding]] strips whatever it carries, so a one-key dict reads as plain code rather than a one-row table. In the dict, annotation, and parameter contexts the column stays within `code-line-length`. A dict entry whose padded line would cross the budget still joins the column where [[reflow-collections]] can break its value open, expanding a collection or, while [[reflow-calls]] runs and no skip holds the call for it, exploding a call one argument per row, because the value then takes its multi-line layout at the padded column. The value breaks open only where that leaves the run in fewer columns, or in as many columns with fewer rows unpadded, so an expansion that merely moves the unpadded row elsewhere in the run is skipped. Any other row whose aligned line would cross the budget starts a new column rather than pushing its neighbors past the margin. Where [[alphabetize-siblings]] sorts the dict, an entry expands this way only when it would be the dict's sole entry spanning rows, because that rule sets a blank line on either side of each entry spanning rows once two do, and those blank lines would leave the expanded entry in a run of its own. A docstring section has no such cap, because [[wrap-docstrings]] runs directly after and rewraps each entry's description to `docstring-line-length` from the column the padding sets.

A `name (type):` head at the docstring body indent under no Title-case heading aligns the same way, because [[wrap-docstrings]] passes such a head through as written rather than joining it to the paragraph above. Each contiguous run of those heads resolves its own two columns, so prose or a blank line between two runs keeps one run's widths out of the other's column, whereas a head directly beneath a paragraph joins no run at all, because the wrap folds it into that paragraph. A `(` written flush against its name documents a call rather than a type, as in `divmod(self, other): the pair`, so no type column opens and the call keeps the form its author wrote, whereas its `:` still aligns with the run because the padding sits after the `)`. The rule reads the module's own docstring as well as every class and function docstring.

<template #configuration>

<RuleConfigTable />

`max-shift` limits how much padding one key may take. The rule reads each group of `:` entries in source order and extends a column while the gap between the widest and narrowest keys stays within the limit, starting a new column at the first key that would exceed it. Setting `max-shift` to `false` removes the limit, so a group of any width aligns on one column, and `0` forbids padding altogether, so every `:` sits flush against its key. The [**per-rule facets**](/reference/configuration#per-rule-facets) reference covers the full semantics.

`align-docstring-entries` gates the two docstring columns alone, so the dict, annotation, and parameter contexts keep aligning under `max-shift` whichever way the facet is set. Some docstring parsers, `docstring_parser` and griffe among them, read everything before the `:` as the parameter's name, padding included, so a project that builds help text or API docs from its docstrings sets the facet to `false`. Unlike turning the whole rule off, which leaves existing padding as written, `false` removes the padding a docstring run already carries, so each run resolves as it would under `max-shift = 0`, even where `max-shift` itself is `false`.

</template>

</RuleLayout>
