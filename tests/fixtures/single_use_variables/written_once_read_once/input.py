"""
A function-local binding written once and read once draws the
canonical diagnostic, with the diagnostic range covering the
assignment target.
"""


def basic(arg):
    x = expensive(arg)
    return x + 1
