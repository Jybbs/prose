"""
A walrus inside a function-body conditional binds its target in the
enclosing function scope, distinct from the iter-source name.
"""


def first_one(items):
    if (n := len(items)) == 1:
        return n
    return None
