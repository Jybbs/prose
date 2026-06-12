---
caption : "aligns the post-pattern `:` across single-expression case bodies inside a `match` statement."
related : [align-colons, align-equals, align-imports, strip-align-padding]
layout  : doc
---

# align-match-case

<RuleLayout rule="align_match_case">

A `match` whose case bodies all collapse to a single expression reads naturally as a dispatch table, with patterns on the left and results on the right. `align-match-case` gathers consecutive single-expression cases into a shared column for the post-pattern `:` separator, so the pattern column flushes left and the body column flushes right, and the reader reads the table by scanning rows rather than tracing each case body.

The rule fires only on runs of single-expression cases at the same indentation. A multi-statement case body, a comment between cases, or a nested `match` breaks the run and leaves the surrounding cases aligned in isolation. Pair with [[strip-align-padding]] to skip padding on one-arm matches and with [[align-colons]] to align separators inside dict-returning case bodies.

<template #configuration>

<RuleConfigTable />

`max-shift` caps the per-line padding the alignment can introduce. When a `match`'s widest pattern would push the post-pattern `:` column past the cap, `max-shift-policy` decides the fallback shape, which defaults to `"split"`. The [**per-rule knobs**](/reference/configuration#per-rule-knobs) reference covers the `"drop"` policy.

</template>

</RuleLayout>
