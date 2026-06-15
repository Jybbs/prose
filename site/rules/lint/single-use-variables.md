---
caption : "surfaces single-use local bindings that could inline cleanly."
related : [reassigned-constants, step-narration]
layout  : doc
---

# single-use-variables

<RuleLayout rule="single_use_variables">

A binding that's assigned once and read once usually exists because the author wanted a name for the expression, and the name reads better than the expression at the call site. Sometimes that's a real win, and sometimes the binding is just standing in for inlining the right-hand side. `single-use-variables` surfaces bindings assigned and read exactly once, leaving the inline-or-keep decision to a future refactor pass that picks up the lint output.

The rule consumes the per-`Source` [[binding-analysis]] table to count writes and reads per binding. Bindings matching the `allow-pattern` regex (*defaulting to `^_`, which exempts intentionally-unused names*) stay quiet. Augmented assignments count as both a write and a read, so a binding they target isn't single-use. Loop variables, comprehension targets, and function parameters are introduced implicitly and stay outside the rule's reach. A tuple-unpack target stays exempt when a sibling target reads more than once, because removing one element would split the unpack into an indexed read. When every target reads once, the diagnostic names the subscript rewrite (*`batch[0]` for the first target of `x, y = batch`*) whenever the right-hand side is a plain name or attribute. A walrus bound in the test of an `if`, `elif`, or `while` is exempt because the test already consumes the value, so the single later read is the second use, and inlining would recompute the guarded expression in both the test and the body. The lint is non-rewriting, so the diagnostic surfaces without touching the source.

<template #configuration>

| Key | Type | Default | Meaning |
|---|---|---|---|
| `enabled` | bool | `true` | Toggle the rule on or off |
| `allow-pattern` | regex | `"^_"` | Binding names exempted from the lint |

The default `^_` exempts names starting with an underscore, matching the Python convention for intentionally-unused bindings. Projects with stricter naming can tighten the regex.

</template>

<template #related-after>

For per-line opt-outs, the [**Suppression**](/usage/suppression#lint-directives) chapter covers the `# prose: ignore[single-use-variables]` directive.

</template>

</RuleLayout>
