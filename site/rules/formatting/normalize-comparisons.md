---
caption : "Rewrites a comparison to state its check directly, settling identity against `None`, operand order, and a leading `not`."
related : [align-comparisons, shed-parentheses]
layout  : doc
---

# normalize-comparisons

<RuleLayout rule="normalize_comparisons">

`x == None`, `42 == n`, and `not a in b` each state a check in a shape that reads around it. The first tests a singleton by value where identity is what it means, the second leads with the constant nobody is looking for, and the third negates a whole comparison where the operator already carries its negated form. One traversal over comparison nodes reaches all of them, so `normalize-comparisons` carries the rewrites with a facet apiece.

A condition carrying every shape at once:

```python
if 0 == n and x == None and not y in ys:
```

comes back stating each check directly:

```python
if n == 0 and x is None and y not in ys:
```

The rule pairs with [[align-comparisons]], because a vertically aligned operator column reads cleanly only when every row leads with its subject and states the check outright.

## What Each Facet Settles

`rewrite-identity` turns `== None` into `is None` and `!= None` into `is not None`, which holds behavior exactly because `None` is a singleton, meaning every `None` in a program is the same object, so identity is what a test against it means. An equality against a constant that is **not** a singleton keeps its `==`, so `None == 0` stays as written rather than becoming a comparison the interpreter never promises to answer the same way.

The same facet flags a test against `True` or `False` instead of rewriting it. The rewrite a reader expects there is a bare `x`, and that changes the test for any operand that is not itself a boolean, wherein `2 == True` is false while `if 2` fires. The diagnostic names both directions, leaving the truth check and the identity check to whoever wrote the line.

`rewrite-operand-order` flips a comparison whose constant side leads, so `42 == n` reads `n == 42` and an ordered operator reverses as it crosses, turning `0 < n` into `n > 0`. Which side counts as the constant runs on a ranking rather than a literal test, wherein a literal outranks a `SCREAMING_CASE` name and both outrank an ordinary one, so `LIMIT == size` flips while `FLOOR == LIMIT` holds the order its author gave it. A collection or an arithmetic expression takes the weakest rank among its parts, leaving `[a, 1] == xs` alone where `[0, 1] == xs` flips.

`rewrite-negation` folds a leading `not` into the operator it negates, so `not a in b` reads `a not in b` and `not a is b` reads `a is not b`. Both folds are exact, because the language defines `not in` and `is not` as the negations of `in` and `is` rather than as separate operations a class can redefine. The equality operators carry no such guarantee, since `__eq__` and `__ne__` are independent, so `not a == b` keeps its `not`.

## What the Rule Leaves Alone

A chained comparison is left as written, whatever its operators. A range check such as `0 < n < 10` already reads in the order its values fall, and rewriting one link of `a == b == None` would leave a chain mixing `==` with `is`, which reads worse than the uniform chain it replaced.

A comparison carrying a comment anywhere inside it holds its operand order too, because a swap would leave a trailing comment attached to whichever operand landed on its line rather than the one it was written about.

A comparison inside an f-string or t-string replacement field is opaque the same way the layout rules treat one, so `f"{not k in table}"` keeps the shape its author gave it while the same expression outside the string folds.

The rewrites compose in one pass, so `not x == None` settles as `x is not None` rather than needing a second run to fold the `not` the identity rewrite exposed. Grouping parentheses travel with the operand they wrap, leaving `42 == (a + b)` parsing exactly as it did once flipped, and [[shed-parentheses]] clears any pair the fold leaves redundant.

<template #configuration>

<RuleConfigTable />

Each facet defaults on, so the rewrites arrive together. Clearing one freezes that move and leaves the rest running, and clearing `rewrite-identity` withdraws the boolean-literal diagnostic alongside the `None` rewrite.

</template>

<template #related-after>

For per-statement opt-outs, the [**Suppression**](/usage/suppression) chapter covers the `# prose: skip[normalize-comparisons]` directive, which holds every line a wrapped condition spans.

</template>

</RuleLayout>
