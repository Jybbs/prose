"""
A module-level def with a leading decorator has its leading edge
counted from the decorator line. The rule normalizes the blank-line
count above the decorator.
"""


def first():
    return 1

@decorator
def second():
    return 2
