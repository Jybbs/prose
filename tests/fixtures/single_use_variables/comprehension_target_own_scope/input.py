"""
A comprehension target lives in the comprehension's own scope rather
than the enclosing function, so its single use never enters the
function-scope enumeration.
"""


def squares(xs):
    return [x * x for x in xs]
