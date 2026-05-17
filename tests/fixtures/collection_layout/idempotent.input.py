"""
A canonical-inline collection and a canonical-multi-line collection
both produce no edit. Inline canonicality holds when the literal
already fits at its column. Multi-line canonicality holds when the
assembled inline form overflows the budget so the rule keeps the
expanded shape.
"""

canonical_inline = {"alpha": 1, "beta": 2, "gamma": 3}
canonical_multi_line = {
    "alpha": 1,
    "beta": 2,
    "gamma": 3,
    "delta": 4,
    "epsilon": 5,
    "zeta": 6,
    "eta": 7
}
