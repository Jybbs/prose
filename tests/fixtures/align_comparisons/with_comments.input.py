"""
A comment between two operands breaks the alignment run, splitting
them into singletons.
"""

if (
    foo == 1
    # divider
    and bar_baz == 2
    and qux == 3
):
    pass
