---
caption : "Sheds the class and instance a `super(C, self)` call restates, leaving the bare `super()` the interpreter resolves on its own."
related : [shed-parentheses, strip-none-return, modernize-annotations]
layout  : doc
---

# shed-super-args

<RuleLayout rule="shed_super_args">

A `super(Button, self)` call restates the enclosing class and the bound instance that the interpreter already resolves from the method it sits in. The reader parses two arguments to learn nothing the surrounding `def` did not already say, and the restatement goes stale the moment the class is renamed. `shed-super-args` deletes the arguments, leaving the `super()` form whose behavior is unchanged and whose intent reads at a glance.

The rewrite fires only where the bare call resolves the same pair, so the first argument must name the one enclosing class *(or the `__class__` cell directly)* and the second must name the enclosing callable's first positional parameter, whether that reads `self`, `cls`, or a positional-only receiver. A call keeps its arguments where they name anything else, where a comprehension or a lambda taking no positional parameter stands between the call and its method, where an enclosing scope binds the class name to something other than the class, where the class is a `@dataclass(slots=True)` whose generated replacement the bare call's cell does not follow, where a comment sits inside the argument list, and where the module binds `super` or `__class__` itself.

Deleting the arguments pulls every token after them leftward, so a later line of the same statement that the author aligned against a column at or past those arguments would be left pointing at a position that moved. The rule declines rather than strand it, which is the same call [[align-imports]] makes when a multi-line import would break its continuation indent. A line hanging one indent step under the statement keeps its depth and the rewrite proceeds, since nothing it was measured against moved.

The pass runs after [[call-layout]], so most such lines never reach it. A call the author hand-wrapped has by then either rejoined onto one row or settled into a flush column whose body hangs at a fixed indent, and neither shape hangs off the parenthesis the deletion moves. What still takes the decline is a continuation the layout rules do not own, a grouping pair around a binary expression being the common one.

<template #configuration>

<RuleConfigTable />

</template>

<template #related-after>

For per-statement opt-outs, the [**Suppression**](/usage/suppression) chapter covers the `# prose: skip[shed-super-args]` directive, which holds every line a wrapped statement spans.

</template>

</RuleLayout>
