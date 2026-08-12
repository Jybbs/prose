---
caption : "Collapses a collection constructor wrapped around a form that already builds that collection, and a comprehension that copies its input unchanged."
related : [reflow-collections, shed-parentheses]
layout  : doc
---

# simplify-comprehensions

<RuleLayout rule="simplify_comprehensions">

`set([x for x in xs])` builds a list, throws it away, and builds a set from it, and the reader unwinds two constructions to reach one value. `simplify-comprehensions` removes the layer that does no work, so a constructor wrapped around a literal, a comprehension, or a generator gives way to the form that was always underneath. `dict()` reaches `{}`, `tuple([1])` reaches `(1,)`, and `dict(alpha=1)` reaches `{"alpha": 1}`.

The brace form is emitted only where it is unambiguous. An empty `set()` stays a call because `{}` names an empty dict rather than an empty set, so `set([])` reaches `set()` and never `{}`. A `dict(...)` call reaches the brace form only where its argument carries key-value pairs a literal or a dict comprehension can hold, leaving `dict(**defaults)` and `dict(defaults, extra=1)` as they stand.

A comprehension whose element repeats its target unchanged is spelling a copy, so `[row for row in rows]` reaches `list(rows)` and `{key: value for key, value in rows}` reaches `dict(rows)`. Adding a guard or a second generator makes the comprehension do work no constructor call does, and both shapes stay as written. Where a wrapper and a copy meet, the rewrite settles in a single step, so `list(row for row in rows)` reaches `list(rows)` directly rather than passing through an intermediate comprehension. It stops short of `list(list(rows))`, since a doubled constructor reads no better than the comprehension it would replace. An f-string or t-string replacement field goes unvisited, so a call written inside one keeps whatever shape its author gave it.

`set`, `dict`, `list`, and `tuple` are builtins a module is free to rebind, and a rebound name no longer reaches the constructor. A module that binds any of the four to something of its own therefore holds every call to that name exactly as written, while the other three collapse as usual.

<template #configuration>

<RuleConfigTable />

</template>

<template #related-after>

For per-statement opt-outs, the [**Suppression**](/usage/suppression) chapter covers the `# prose: skip[simplify-comprehensions]` directive, which holds every line a wrapped call spans.

</template>

</RuleLayout>
