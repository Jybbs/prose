---
caption : "Reports a parameter with no type annotation and a value-returning function with no return annotation."
related : [strip-none-return, reflow-signatures, modernize-annotations, restated-types]
layout  : doc
---

# signature-annotations

<RuleLayout rule="signature_annotations">

`signature-annotations` reports a parameter that carries no type annotation, leaving a method's `self`, a classmethod's `cls`, and the `*args` and `**kwargs` variadics outside the rule. An unannotated parameter is a legibility gap, because the reader meets the function without knowing what it takes, whereas the annotated form lays out cleanly under the [[align-colons]] and [[align-equals]] columns.

The rule also reports a function whose body returns a value and carries no return annotation. A procedure that returns nothing stays silent, so in a clean file a signature without a return annotation reads as a function that returns nothing. The companion [[strip-none-return]] rule enforces the other side of that convention, removing an explicit `-> None` because the omission already says it.

*Prose* reads source without resolving types, so the rule never writes an annotation for the author. The report carries a suggestion the reader applies by hand when a confident local signal exists, a literal default *(`threshold=0.8` suggesting `float`)* or in-module call sites passing only literals. A bare `= None` default contributes its `| None` arm only beside another signal, and conflicting or non-literal signals leave the report without a suggestion. The suggestion is recorded as a **display-only** fix, shown to the reader and never applied.

<template #configuration>

<RuleConfigTable />

</template>

<template #related-after>

For per-line opt-outs, the [**Suppression**](/usage/suppression#tagging-a-line) chapter covers the `# prose: ignore[signature-annotations]` directive.

</template>

</RuleLayout>
