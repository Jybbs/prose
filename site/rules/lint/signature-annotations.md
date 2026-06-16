---
caption : "Flags a signature parameter or a value-returning function that carries no type annotation."
related : [strip-none-return, signature-layout, legacy-union-syntax]
layout  : doc
---

# signature-annotations

<RuleLayout rule="signature_annotations">

An unannotated parameter is a legibility gap, leaving the reader to meet the function without knowing what it takes, whereas the annotated form lays out cleanly through the [[align-colons]] and [[align-equals]] columns. `signature-annotations` reports a parameter that carries no type annotation, leaving a method's `self`, a classmethod's `cls`, and the `*args` and `**kwargs` variadics outside the rule.

The rule also reports a function whose body returns a value yet carries no return annotation. A procedure that returns nothing stays silent, so in a clean file a signature without a return annotation reads as a function that returns nothing. The companion [[strip-none-return]] rule enforces the other side of that convention, dropping an explicit `-> None` the omission already reads as visual weight.

Because *Prose* reads source rather than resolving types, the rule never synthesizes an annotation for the author. When a confident local signal exists, a literal default *(`threshold=0.8` suggesting `float`)* or in-module call sites passing only literals, the report carries a suggestion the reader applies by hand. A bare `= None` default contributes its `| None` arm only alongside another signal, and conflicting or non-literal signals leave the report unsuggested. The suggestion rides as a **display-only** fix, recorded for the reader but never applied.

<template #configuration>

| Key | Type | Default | Meaning |
|---|---|---|---|
| `enabled` | bool | `true` | Toggle the rule on or off |

</template>

<template #related-after>

For per-line opt-outs, the [**Suppression**](/usage/suppression#lint-directives) chapter covers the `# prose: ignore[signature-annotations]` directive.

</template>

</RuleLayout>
