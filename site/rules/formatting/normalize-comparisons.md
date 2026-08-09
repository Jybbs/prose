---
caption : "Rewrites a comparison to state its check directly, settling identity against `None`, operand order, and a leading `not`."
lints   : true
related : [align-comparisons, shed-parentheses]
layout  : doc
---

# normalize-comparisons

<RuleLayout rule="normalize_comparisons">

`normalize-comparisons` states a check in the shape that reads directly, turning a singleton tested by value into an identity test, a constant-leading comparison onto its subject, and a negated comparison into the operator carrying that negation. One traversal reaches all three, each behind its own facet.

```python
if 0 == n and x == None and not y in ys:
```

```python
if n == 0 and x is None and y not in ys:
```

## What Each Facet Settles

`rewrite-identity` turns `== None` into `is None` and `!= None` into `is not None`, `None` being a singleton so identity is what a test against it means. An equality against a non-singleton constant keeps its `==`, leaving `None == 0` as written.

The same facet flags a test against `True` or `False` rather than rewriting it, since the bare `x` a reader expects changes the test for any non-boolean operand, wherein `2 == True` is false while `if 2` fires.

`rewrite-operand-order` flips a comparison whose constant side leads, so `42 == n` reads `n == 42` and an ordered operator reverses as it crosses, turning `0 < n` into `n > 0`. A literal outranks a `SCREAMING_CASE` name and both outrank an ordinary one, so `LIMIT == size` flips while `FLOOR == LIMIT` holds. A collection or arithmetic expression takes the weakest rank among its parts, leaving `[a, 1] == xs` alone where `[0, 1] == xs` flips.

`rewrite-negation` folds a leading `not` into the operator it negates, so `not a in b` reads `a not in b` and `not a is b` reads `a is not b`. Both folds are exact, the language defining `not in` and `is not` as the negations of `in` and `is`, whereas `__eq__` and `__ne__` are independent so `not a == b` keeps its `not`.

## What the Rule Leaves Alone

A chained comparison holds whatever its operators, `0 < n < 10` already reading in the order its values fall and one rewritten link of `a == b == None` leaving a chain mixing `==` with `is`.

A comparison carrying a comment anywhere inside it holds its operand order, a swap leaving the comment attached to whichever operand landed on its line.

A comparison inside an f-string or t-string replacement field is opaque the way the layout rules treat one.

The rewrites compose in one pass, so `not x == None` settles as `x is not None` rather than needing a second run. Grouping parentheses travel with the operand they wrap, and [[shed-parentheses]] clears any pair the fold leaves redundant.

<template #configuration>

<RuleConfigTable />

Each facet defaults on, so the rewrites arrive together. Clearing one freezes that move and leaves the rest running, and clearing `rewrite-identity` withdraws the boolean-literal diagnostic alongside the `None` rewrite.

</template>

<template #related-after>

For per-statement opt-outs, the [**Suppression**](/usage/suppression) chapter covers the `# prose: skip[normalize-comparisons]` directive, which holds every line a wrapped condition spans.

</template>

</RuleLayout>
