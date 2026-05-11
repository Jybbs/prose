"""
Non-list comprehension shapes (`{k: v for ...}`, `{x for ...}`,
`(x for ...)`) push their own comprehension scope alongside the
already-covered list-comprehension shape.
"""


pairs = [("a", 1), ("b", 2)]
mapping = {k: v for k, v in pairs}
keys = {k for k, _ in pairs}
total = sum(v for _, v in pairs)
