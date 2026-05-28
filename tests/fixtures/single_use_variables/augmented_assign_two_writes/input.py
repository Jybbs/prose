"""
An augmented assignment is both a read and a write of its target, so
the binding's write count climbs above one and the single-use rule
leaves it alone.
"""


def accumulate(items):
    total = 0
    for value in items:
        total += value
    return total
