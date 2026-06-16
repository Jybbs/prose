---
caption : "Surfaces a free function whose parameters sit out of alphabetical order."
related : [alphabetize]
layout  : doc
---

# unsorted-parameters

<RuleLayout rule="unsorted_parameters">

Alphabetical parameters give a reader the same landmarks in a signature that [[alphabetize]] gives them everywhere else. Prose will not reorder them for you, though, because a parameter's position is part of the function's contract. Every positional call binds by slot, and a single-file formatter cannot see the callers in other modules, in frameworks, or behind dynamic dispatch, so moving a parameter would silently rebind them. `unsorted-parameters` reports the out-of-order run instead, leaving the reorder to a hand that can weigh the callers.

The lint fires only where acting on it could be reasonable, a module-level or nested free function whose positional-or-keyword parameters are out of alphabetical order, required parameters before optional. It stays silent everywhere a reorder would be unsafe to even suggest. A class-body method draws nothing, since its callers routinely live outside the module. A function under a positional-binding decorator (*`pytest.mark.parametrize`, `click.argument`, and the like*) draws nothing, since the decorator may hand values to the parameters by slot. The `self` and `cls` receivers and the positional-only parameters before the `/` hold their place regardless, so they never count toward the disorder. The lint is non-rewriting, so the diagnostic surfaces without touching the source.

<template #configuration>

| Key | Type | Default | Meaning |
|---|---|---|---|
| `enabled` | bool | `true` | Toggle the rule on or off |

The keyword-only block past the `*` is a separate matter. A keyword-only parameter binds by name at every call site, so reordering it is always behavior-preserving, and [[alphabetize]] sorts it as an auto-fix rather than reporting it here.

</template>

<template #related-after>

For per-line opt-outs, the [**Suppression**](/usage/suppression#lint-directives) chapter covers the `# prose: ignore[unsorted-parameters]` directive.

</template>

</RuleLayout>
