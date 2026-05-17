"""
An operand using `is`, `is not`, `in`, or `not in` breaks the run,
leaving the surrounding `==` operands as singletons.
"""

if (
    foo == 1
    and bar is None
    and qux == 3
):
    pass
