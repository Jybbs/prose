---
caption : "Aligns the post-pattern `:` across single-expression case bodies inside a `match` statement."
related : [align-colons, align-equals, align-imports, strip-align-padding]
layout  : doc
---

# align-match-case

<RuleLayout rule="align_match_case">

A `match` whose case bodies all collapse to a single expression reads naturally as a dispatch table, with patterns on the left and results on the right. `align-match-case` gathers consecutive single-expression cases into a shared column for the post-pattern `:` separator, so the pattern column flushes left and the body column flushes right, and the reader reads the table by scanning rows rather than tracing each case body.

The rule fires only on runs of single-expression cases at the same indentation. A multi-statement case body, a comment between cases, or a nested `match` breaks the run and leaves the surrounding cases aligned in isolation. Pair with [[strip-align-padding]] to skip padding on one-arm matches and with [[align-colons]] to align separators inside dict-returning case bodies.

<template #configuration>

<RuleConfigTable />

`max-shift` bounds how far the post-pattern `:` may shift to align. The rule walks each run of arms in source order and grows a column while its width spread stays within the cap, breaking a fresh column at the first arm that would exceed it. A `max-shift` of `false` lifts the cap so a contiguous run folds into one column, and `0` forbids any shift so every `:` sits flush. The [**per-rule knobs**](/reference/configuration#per-rule-knobs) reference covers the full semantics.

</template>

</RuleLayout>
