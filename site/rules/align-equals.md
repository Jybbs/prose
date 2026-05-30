---
category : auto-fix
family   : alignment
caption  : "aligns the `=` separator across consecutive single-target assignments and annotated function-parameter defaults."
related  : [align-colons, align-imports, align-match-case, strip-align-padding]
layout   : doc
---

# align-equals

<RuleLayout rule="align_equals">

A run of consecutive bindings sits at the same indentation, the eye walks down the page, and every `=` sign lands at a **different column**. The reader stops at each line to find where the assignment splits. `align-equals` gathers those runs into a **single column**, so a stretch of bindings reads as a list of values rather than a stack of expressions.

The rule walks consecutive single-target assignments at the same indentation level, picking up type annotations when present and treating augmented assignments (*`+=`, `|=`*) and walrus operators (*`:=`*) as non-members. The same alignment also runs across consecutive annotated function-parameter default values, so a signature with several `param: type = default` entries aligns its `=` column the same way a stretch of module-level bindings does. A blank line, a comment line, or a non-assignment statement resets the group, leaving each contiguous run aligned in isolation. Once an alignment group lands, [[strip-align-padding]] prunes any one-member residue so a lone binding reads as plain code.

<template #configuration>

<RuleConfigTable />

`max-shift` caps the per-line padding the alignment can introduce. When a group's widest member would force more padding than the cap allows, `max-shift-policy` decides the fallback shape, which defaults to `"split"`. The [**per-rule knobs**](/reference/configuration#per-rule-knobs) reference covers the `"drop"` policy.

</template>

</RuleLayout>
