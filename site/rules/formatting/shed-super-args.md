---
caption : "Removes the class and instance arguments from a `super(C, self)` call, leaving the bare `super()` the interpreter resolves on its own."
related : [reflow-parentheses, strip-none-return, modernize-annotations]
layout  : doc
---

# shed-super-args

<RuleLayout rule="shed_super_args">

`shed-super-args` removes the arguments from a `super(Button, self)` call, leaving `super()`, which behaves the same and reads at a glance. The two arguments restate the enclosing class and the bound instance the interpreter already resolves from the method the call sits in, so the reader parses them to learn nothing the surrounding `def` did not already say, and the restatement goes stale the moment the class is renamed.

The rewrite runs only where the bare call resolves the same pair, so the first argument has to name the one enclosing class *(or the `__class__` cell directly)* and the second has to name the enclosing callable's first positional parameter, whether that reads `self`, `cls`, or a positional-only receiver. A call keeps its arguments wherever one of these applies:

1. The arguments name anything else.
2. A comprehension or a lambda taking no positional parameter sits between the call and its method.
3. An enclosing scope binds the class name to something other than the class.
4. The class is a `@dataclass(slots=True)`, whose generated replacement the bare call's cell does not follow.
5. A comment sits inside the argument list.
6. The module binds `super` or `__class__` itself.

Removing the arguments pulls every token after them leftward. Where the author aligned a later line of the same statement to a column at or past those arguments, that line moves left by the width of the removed span and keeps pointing at the column it was measured against. A line hanging one indent step under the statement keeps its depth, since nothing it was measured against moved. A row inside a multi-line string is the one continuation no move can shift without changing the string's value, so a call whose rewrite would have to move such a row keeps its arguments.

The rule runs ahead of [[reflow-calls]], so the bare `super()` is the text every later width measurement reads, and a call the rewrite brings within its budget settles in the same pass.

<template #configuration>

<RuleConfigTable />

</template>

<template #related-after>

For per-statement opt-outs, the [**Suppression**](/usage/suppression) chapter covers the `# prose: skip[shed-super-args]` directive, which covers every line a wrapped statement spans.

</template>

</RuleLayout>
