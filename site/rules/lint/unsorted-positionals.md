---
caption : "Surfaces a run of positionally-bound names that sits out of alphabetical order."
related : [alphabetize]
layout  : doc
---

# unsorted-positionals

<RuleLayout rule="unsorted_positionals">

Alphabetical order gives a reader the same landmarks in a positional run that [[alphabetize]] gives them everywhere else. *Prose* will not reorder the run for you, though, because each name's slot is part of the call contract. Every positional call binds by slot, and a single-file formatter cannot see the callers in other modules, in frameworks, or behind dynamic dispatch, so moving a name would silently rebind them. `unsorted-positionals` reports the out-of-order run instead, leaving the reorder to a hand that can weigh the callers.

Two constructs carry such a run, the first being a function's positional-or-keyword parameters, free function and method alike, since a method's callers bind by slot exactly as a free function's do. The second is the annotated field run of a class whose header generates a positional constructor, where a `NamedTuple` base or a `@dataclass` decorator turns the fields into that constructor's parameters and a call like `Window(1920, 1080)` binds them in source order.

A function under a positional-binding decorator (*`pytest.mark.parametrize`, `click.argument`, and the like*) draws nothing, since the decorator may hand values to the parameters by slot. A name that binds no positional slot drops from the run rather than silencing it, covering the `self` and `cls` receivers, the positional-only parameters before the `/`, a `ClassVar` declaration, and the `dataclasses.KW_ONLY` sentinel. The lint is non-rewriting, so the diagnostic surfaces without touching the source.

<template #configuration>

<RuleConfigTable />

The target order puts the required names alphabetized ahead of the defaulted names alphabetized, rather than plain alphabetical throughout. Python permits nothing else, in that a required field following a defaulted one raises `TypeError: non-default argument 'zebra' follows default argument 'alpha'` the moment the class is created.

The keyword-only block past the `*` is a separate matter, along with the fields below a `KW_ONLY` sentinel and those of a `kw_only=True` generator. Each binds by name at every call site, so reordering it is always behavior-preserving, and [[alphabetize]] sorts it as an auto-fix rather than reporting it here.

</template>

<template #related-after>

For per-line opt-outs, the [**Suppression**](/usage/suppression#lint-directives) chapter covers the `# prose: ignore[unsorted-positionals]` directive.

</template>

</RuleLayout>
