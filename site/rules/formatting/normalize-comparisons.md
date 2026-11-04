---
caption : "Rewrites a comparison to state its check directly, using `is` against `None`, putting the variable side first, and folding a leading `not` into its operator."
related : [align-comparisons, reflow-parentheses]
layout  : doc
---

# normalize-comparisons

<RuleLayout rule="normalize_comparisons">

`normalize-comparisons` rewrites a comparison so it states its check directly, turning a `==` or `!=` test against `None` into an identity test, moving a leading constant to the right of its subject, and folding a leading `not` into the operator it negates.

```python
if 0 == n and x == None and not y in ys:
```

```python
if n == 0 and x is None and y not in ys:
```

## Comparisons Left as Written

A chained comparison stays as written whatever its operators, because `0 < n < 10` already reads in the order its values fall and rewriting one link of `a == b == None` would leave a chain mixing `==` with `is`.

A comparison with a comment anywhere inside it keeps its operand order, since a swap would leave the comment attached to whichever operand ended up on its line.

A comparison inside an f-string or t-string replacement field is left as written.

<template #facets>

Each facet defaults on, so the rewrites arrive together. Setting one to `false` stops that rewrite and leaves the rest running.

The rewrites compose in one pass, so `not x == None` settles as `x is not None` without a second run. Grouping parentheses move with the operand they wrap, and [[reflow-parentheses]] removes any pair the fold leaves redundant.

### `rewrite-identity`

`rewrite-identity` turns `== None` into `is None` and `!= None` into `is not None`, because `None` is a singleton and a test against it is an identity test. An equality against a non-singleton constant keeps its `==`, so `None == 0` stays as written.

The same facet reports a test against `True` or `False` rather than rewriting it, since dropping the literal to test the bare operand changes the result for any non-boolean operand, in that `2 == True` is false whereas `if 2:` runs its body. Setting `rewrite-identity` to `false` stops that report along with the `None` rewrite.

### `rewrite-negation`

`rewrite-negation` folds a leading `not` into the operator it negates, so `not a in b` reads `a not in b` and `not a is b` reads `a is not b`. Both folds are exact, because the language defines `not in` and `is not` as the negations of `in` and `is`, whereas `__eq__` and `__ne__` are independent methods, so `not a == b` keeps its `not`.

### `rewrite-operand-order`

`rewrite-operand-order` flips a comparison whose constant side comes first, so `42 == n` reads `n == 42`, and an ordered operator reverses as it crosses, which turns `0 < n` into `n > 0`. A literal ranks as more constant than a `SCREAMING_CASE` name and both rank above an ordinary name, which is why `LIMIT == size` flips whereas `FLOOR == LIMIT` stays. A collection or arithmetic expression takes the lowest rank among its parts, so `[a, 1] == xs` stays where `[0, 1] == xs` flips.

</template>

</RuleLayout>
