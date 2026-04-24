"""
An already-canonically-expanded collection produces no edit on a
second pass. Multi-line input always takes the expansion path, but
when the canonical output matches the input verbatim the rule emits
no `Edit::range_replacement` at all.
"""

config = {
    "alpha": 1,
    "beta": 2,
    "gamma": 3
}
