---
caption : "Rewrites a parameterized `super(C, self)` call to the bare `super()` the interpreter resolves on its own."
related : [shed-parentheses, strip-none-return, modernize-annotations]
layout  : doc
---

# bare-super

<RuleLayout rule="bare_super">

A `super(Button, self)` call restates the enclosing class and the bound instance that the interpreter already resolves from the method it sits in. The reader parses two arguments to learn nothing the surrounding `def` did not already say, and the restatement goes stale the moment the class is renamed. `bare-super` deletes the arguments, leaving the `super()` form whose behavior is unchanged and whose intent reads at a glance.

The rewrite fires only where the bare call resolves the same pair, so the first argument must name the one enclosing class *(or the `__class__` cell directly)* and the second must name the enclosing callable's first positional parameter, whether that reads `self`, `cls`, or a positional-only receiver. A call keeps its arguments where they name anything else, where a comprehension or a lambda taking no positional parameter stands between the call and its method, where an enclosing scope binds the class name to something other than the class, where the class is a `@dataclass(slots=True)` whose generated replacement the bare call's cell does not follow, where a comment sits inside the argument list, and where the module binds `super` or `__class__` itself.

<template #configuration>

<RuleConfigTable />

</template>

<template #related-after>

For per-statement opt-outs, the [**Suppression**](/usage/suppression) chapter covers the `# prose: skip[bare-super]` directive, which holds every line a wrapped statement spans.

</template>

</RuleLayout>
