"""
Nested function defs inside another function body do not pair-count.
The Function scope has no canonical blank-line rule, so the inner
defs round-trip unchanged regardless of their source spacing.
"""


def outer():
    def inner_a():
        return 1
    def inner_b():
        return 2
    return inner_a, inner_b
