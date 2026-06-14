---
caption : "drops a redundant `-> None` return annotation, since an omitted one already reads as returning nothing."
related : [signature-annotations, signature-layout, unused-future-annotations]
layout  : doc
---

# strip-none-return

<RuleLayout rule="strip_none_return">

A written `-> None` on a function that returns nothing is visual weight the signature does not need. The omission convention already reads an absent return annotation as a function that returns nothing, leaving the explicit form as noise rather than information. `strip-none-return` rewrites it away.

The rewrite is purely mechanical. It fires only when the return annotation is a bare `None`, leaving a `None` nested inside a larger annotation *(`int | None`, `Callable[..., None]`)* and every parameter annotation untouched. The companion [[signature-annotations]] rule enforces the other side of the convention, reporting where a parameter or a value-returning function lacks the annotation it owes.

<template #configuration>

| Key | Type | Default | Meaning |
|---|---|---|---|
| `enabled` | bool | `true` | Toggle the rule on or off |

</template>

<template #related-after>

For per-line opt-outs, the [**Suppression**](/usage/suppression) chapter covers the `# prose: skip[strip-none-return]` directive.

</template>

</RuleLayout>
