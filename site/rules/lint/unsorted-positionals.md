---
caption : "Reports a run of positionally-bound names that is out of alphabetical order, without reordering it."
related : [alphabetize-siblings]
layout  : doc
---

# unsorted-positionals

<RuleLayout rule="unsorted_positionals">

`unsorted-positionals` reports a run of positionally-bound names that sits out of alphabetical order and leaves the reorder to a hand that can check the callers. Alphabetical order gives a reader the same landmarks in a positional run that [[alphabetize-siblings]] gives everywhere else, yet *Prose* never reorders the run, because each name's slot is part of the call contract. Every positional call binds by slot, and a single-file formatter cannot see the callers in other modules, in frameworks, or behind dynamic dispatch, so moving a name would silently rebind them.

Two constructs carry such a run, and the first is a function's positional-or-keyword parameters, free function and method alike, because a method's callers bind by slot exactly as a free function's do. The second is the annotated field run of a class whose header generates a positional constructor, where a `NamedTuple` base or a `@dataclass` decorator turns the fields into that constructor's parameters and a call like `Window(1920, 1080)` binds them in source order.

A function whose decorator is a call carrying positional arguments (*`pytest.mark.parametrize(...)`, `click.argument(...)`, and the like*) draws no report, because the decorator may bind values to the parameters by slot. A name that binds no positional slot drops from the run rather than silencing it, covering the `self` and `cls` receivers, the positional-only parameters before the `/`, a `ClassVar` declaration, and the `dataclasses.KW_ONLY` sentinel. The lint never rewrites, so the diagnostic is reported and the source stays as written.

<template #configuration>

<RuleConfigTable />

The target order puts the required names in alphabetical order ahead of the defaulted names in alphabetical order, rather than plain alphabetical order throughout, because Python permits nothing else. A required field after a defaulted one raises `TypeError: non-default argument 'zebra' follows default argument 'alpha'` the moment the dataclass is created.

The keyword-only block past the `*` is a separate matter, along with the fields below a `KW_ONLY` sentinel and those of a `kw_only=True` generator. Each binds by name at every call site, so reordering it always preserves behavior, and [[alphabetize-siblings]] sorts it as an auto-fix rather than reporting it here.

</template>

<template #related-after>

For per-line opt-outs, the [**Suppression**](/usage/suppression#tagging-a-line) chapter covers the `# prose: ignore[unsorted-positionals]` directive.

</template>

</RuleLayout>
