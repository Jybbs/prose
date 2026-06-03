---
category : auto-fix
family   : alignment
caption  : "aligns the `:` separator across dict literals, dataclass and Pydantic fields, function-signature annotations, and docstring `Args:` blocks."
related  : [align-equals, align-imports, alphabetize, collection-layout, align-match-case, strip-align-padding]
layout   : doc
---

# align-colons

<RuleLayout rule="align_colons">

The `:` separator appears across the contexts below, wherein columns of values sit beside columns of names and the reader's eye wants a tidy table rather than a ragged margin. `align-colons` gathers those contexts into a single shared alignment surface, so dictionary keys, dataclass and Pydantic fields, function-signature parameter annotations, and docstring `Args:` blocks all read as parallel two-column entries. Single-expression `match` arms live in a separate dispatch table owned by [[align-match-case]].

The rule walks each context independently, treating a group as the consecutive members sharing the same indentation level and parent shape. A blank line, an own-line comment, or a non-member statement resets the group. Alignment honors the [[strip-align-padding]] so that one-member contexts skip padding altogether, leaving a one-key dict reading as plain code instead of a one-row table.

<template #configuration>

<RuleConfigTable />

`max-shift` caps the per-line padding the alignment can introduce. When a group's widest member would push the column past the cap, `max-shift-policy` decides the fallback shape *(`"split"` partitions the group, `"drop"` excludes the widest members from the padding calculation)*. The [**per-rule knobs**](/reference/configuration#per-rule-knobs) reference covers the full semantics.

</template>

</RuleLayout>
