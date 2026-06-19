---
caption : "Aligns the `=` separator across consecutive single-target assignments, annotated function-parameter defaults, and an exploded call's keyword arguments."
related : [align-colons, align-imports, align-match-case, strip-align-padding]
layout  : doc
---

# align-equals

<RuleLayout rule="align_equals">

A run of consecutive bindings sits at the same indentation, the eye walks down the page, and every `=` sign lands at a **different column**. The reader stops at each line to find where the assignment splits. `align-equals` gathers those runs into a **single column**, so a stretch of bindings reads as a list of values rather than a stack of expressions.

The rule walks consecutive single-target assignments at the same indentation level, picking up type annotations when present and folding augmented assignments (*`+=`, `|=`*) into the run with the operator one column before the shared `=`, while walrus operators (*`:=`*) stay non-members. Every aligned row reads as `name = value`, the name side padded to its column and the value side collapsed to one space after the operator. The same alignment also runs across consecutive annotated function-parameter default values, so a signature with several `param: type = default` entries aligns its `=` column the same way a stretch of module-level bindings does. It reaches the keyword arguments that sit alone on their line in an exploded call, so a call written one keyword per line at a shared column aligns its `name = value` column the way a signature aligns its defaults. A keyword that sits alone on its line but shares no column still takes the one-space buffer, whereas a keyword condensed onto a line with another argument keeps its tight `name=value` form the way PEP 8 writes a call-site keyword. A positional argument, a `**` unpacking, an interior comment, or a condensed keyword ends the run, whereas a multi-line value or default joins its run and then closes it, so the entries past it align as a separate group. A blank line, a comment line, or a non-assignment statement resets the group, leaving each contiguous run aligned in isolation. Once an alignment group lands, [[strip-align-padding]] prunes any one-member residue so a lone binding reads as plain code.

<template #configuration>

<RuleConfigTable />

`max-shift` bounds how far a row may shift to align. The rule walks each run of assignments in source order and grows a column while its width spread stays within the cap, breaking a fresh column at the first row that would exceed it. A `max-shift` of `false` lifts the cap so a contiguous run folds into one column, and `0` forbids any shift so every `=` sits flush. The [**per-rule facets**](/reference/configuration#per-rule-facets) reference covers the full semantics.

</template>

</RuleLayout>
