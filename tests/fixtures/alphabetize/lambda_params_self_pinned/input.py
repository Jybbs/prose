ranked = sorted(items, key=lambda b, a: a + b)


paired = sorted(
    pairs,
    key=lambda y, x, *, weight=1.0, scale=2.0: weight * (x + y) * scale,
)
