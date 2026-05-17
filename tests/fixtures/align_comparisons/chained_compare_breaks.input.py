"""
A chained compare like `a < b < c` breaks the group, leaving the
surrounding qualifying operands as singletons.
"""

if (
    foo == 1
    and 0 < bar < 100
    and qux == 3
):
    pass
