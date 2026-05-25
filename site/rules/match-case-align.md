---
category : auto-fix
family   : alignment
caption  : "aligns the post-pattern `:` across single-expression case bodies inside a `match` statement."
related  : [align-colons, align-equals, align-imports, singleton-rule]
layout   : doc
---

# match-case-align

<RuleLayout rule="match_case_align" canonical="expr_bodies">

A `match` whose case bodies all collapse to a single expression reads naturally as a dispatch table, with patterns on the left and results on the right. `match-case-align` gathers consecutive single-expression cases into a shared column for the post-pattern `:` separator, so the pattern column flushes left and the body column flushes right, and the reader reads the table by scanning rows rather than tracing each case body.

The rule fires only on runs of single-expression cases at the same indentation. A multi-statement case body, a comment between cases, or a nested `match` breaks the run and leaves the surrounding cases aligned in isolation. Pair with [[singleton-rule]] to skip padding on one-arm matches and with [[align-colons]] to align separators inside dict-returning case bodies.

<template #configuration>

<RuleConfigTable />

`max-shift` caps the per-line padding the alignment can introduce. When a `match`'s widest pattern would push the post-pattern `:` column past the cap, `max-shift-policy` decides the fallback shape *(the `budget_gate` fixture above shows the default `"split"` behavior)*. The [**per-rule knobs**](/reference/configuration#per-rule-knobs) reference covers the `"drop"` and `"skip"` policies.

</template>

<template #canonical-lead>

A `match` whose arms each return a literal aligns on the post-pattern `:` separator.

</template>

<template #more-examples>

<Fixture rule="match_case_align" case="mixed_arms" title="Multi-Statement Arms Break the Alignment Run" />

<Fixture rule="match_case_align" case="multiline_body" title="Multi-Line Expressions Break the Run, Too" />

<Fixture rule="match_case_align" case="or_pattern" title="Or-Patterns Pad to the Widest Member" />

<Fixture rule="match_case_align" case="nested_match" title="Nested Matches Align Independently" />

<Fixture rule="match_case_align" case="comment_between_cases" title="Comments between Cases Reset the Run" />

<Fixture rule="match_case_align" case="budget_gate" title="A Widest Pattern past `max-shift` Gates the Group" />

</template>

</RuleLayout>
