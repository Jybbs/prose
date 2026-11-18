---
caption : "Removes a collection constructor wrapped around a literal, comprehension, or generator that already builds that collection, and replaces a comprehension that copies its input unchanged with the constructor call."
related : [reflow-collections, reflow-parentheses]
layout  : doc
---

# simplify-comprehensions

<RuleLayout rule="simplify_comprehensions">

`simplify-comprehensions` removes the layer that does no work, so a constructor wrapped around a literal, a comprehension, or a generator is replaced by the form underneath it, and `set([row.width for row in rows])` reads `{row.width for row in rows}`. `set([x for x in xs])` builds a list, throws it away, and builds a set from it, and the reader unwinds two constructions to reach one value, so that call reads `set(xs)`. `dict()` becomes `{}`, `tuple([1])` becomes `(1,)`, and `dict(alpha=1)` becomes `{"alpha": 1}`.

The brace form is written only where it is unambiguous. An empty `set()` stays a call because `{}` names an empty dict rather than an empty set, so `set([])` becomes `set()` and never `{}`. A `dict(...)` call becomes the brace form only where its argument carries key-value pairs a literal or a dict comprehension can express, so `dict(**defaults)` and `dict(defaults, extra=1)` stay as written.

Each keyword of a `dict(...)` call sets where its key falls in the mapping's iteration order, so the call keeps its form wherever [[alphabetize-siblings]] would sort the literal it becomes into another order. It becomes the brace form where its keys already sit in sorted order, where the module reads the order of the mapping so that the literal keeps it, where [[alphabetize-siblings]] or its `sort-dict-keys` facet is off, and where a `# prose: keep`, or a `# prose: skip` covering [[alphabetize-siblings]], holds the call.

<FixtureCards rule="simplify_comprehensions" cases="keyword_call_keeps_its_order" />

A comprehension whose element repeats its target unchanged spells a copy, so `[row for row in rows]` becomes `list(rows)` and `{key: value for key, value in rows}` becomes `dict(rows)`. A guard or a second generator makes the comprehension do work no constructor call does, and both forms stay as written. Where a wrapper and a copy meet, the rewrite settles in one step, so `list(row for row in rows)` becomes `list(rows)` directly rather than passing through an intermediate comprehension. It stops short of `list(list(rows))`, since a doubled constructor reads no better than the comprehension it would replace. An f-string or t-string replacement field is not visited, so a call written inside one keeps whatever form its author gave it.

`set`, `dict`, `list`, and `tuple` are builtins a module is free to rebind, and a rebound name no longer names the constructor. A module that binds any of the four to something of its own therefore keeps every call to that name exactly as written, while the other three still collapse.

</RuleLayout>
