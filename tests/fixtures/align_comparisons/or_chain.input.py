"""
The rule aligns operands across `or` chains with the same column
math it applies to `and` chains.
"""

if (
    foo == 1
    or bar_baz == 2
    or quux == 3
):
    pass
