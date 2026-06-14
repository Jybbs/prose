---
caption : "aligns the `=` separator across consecutive single-target assignments, annotated function-parameter defaults, and an exploded call's keyword arguments."
related : [align-colons, align-imports, align-match-case, strip-align-padding]
layout  : doc
---

# align-equals

<RuleLayout rule="align_equals">

A run of consecutive bindings sits at the same indentation, the eye walks down the page, and every `=` sign lands at a **different column**. The reader stops at each line to find where the assignment splits. `align-equals` gathers those runs into a **single column**, so a stretch of bindings reads as a list of values rather than a stack of expressions.

The rule walks consecutive single-target assignments at the same indentation level, picking up type annotations when present and treating augmented assignments (*`+=`, `|=`*) and walrus operators (*`:=`*) as non-members. The same alignment also runs across consecutive annotated function-parameter default values, so a signature with several `param: type = default` entries aligns its `=` column the same way a stretch of module-level bindings does. It reaches an exploded call's keyword arguments too, so a call written one keyword per line at a shared column aligns its `name = value` column the way a signature aligns its defaults. Every exploded keyword carries a one-space buffer on each side of its `=` when no shared column exists, whether it stands alone or packs several keywords onto one physical line. A positional argument, a `**` unpacking, or an interior comment ends the run, whereas a multi-line value or default joins its run and then closes it, so the entries past it align as a separate group. A blank line, a comment line, or a non-assignment statement resets the group, leaving each contiguous run aligned in isolation. Once an alignment group lands, [[strip-align-padding]] prunes any one-member residue so a lone binding reads as plain code.

<template #configuration>

<RuleConfigTable />

`max-shift` bounds how far a row may shift to align. The rule walks each run of assignments in source order and grows a column while its width spread stays within the cap, breaking a fresh column at the first row that would exceed it. A `max-shift` of `false` lifts the cap so a contiguous run folds into one column, and `0` forbids any shift so every `=` sits flush. The [**per-rule knobs**](/reference/configuration#per-rule-knobs) reference covers the full semantics.

</template>

</RuleLayout>
