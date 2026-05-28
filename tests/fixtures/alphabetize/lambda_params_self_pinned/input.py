"""
Lambda parameters alphabetize the same way function parameters
do, with `self` / `cls` pinned (vanishingly rare as lambda
parameter names but consistent with the function-parameter rule).
Lambdas cannot carry decorators, so the positional-binding skip
never engages here.
"""

ranked = sorted(items, key=lambda b, a: a + b)


paired = sorted(
    pairs,
    key=lambda y, x, *, weight=1.0, scale=2.0: weight * (x + y) * scale,
)
