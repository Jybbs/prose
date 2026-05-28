"""
A five-parameter signature with out-of-order typed parameters runs
through `alphabetize` → `signature-layout` → `align-colons` →
`align-equals` in pipeline order. The composition reorders the
parameters into required-then-optional alphabetical groups, expands
the now-long inline form to one parameter per line at the def's
indent, aligns the type-annotation `:` columns within the expanded
shape, and aligns the default `=` columns across the optional run.

Rules:
- alphabetize
- signature-layout
- align-colons
- align-equals
"""


def configure(zebra: str, alpha: int, beta: float, mango: bool = False, delta: float = 0.5):
    return (zebra, alpha, beta, mango, delta)
