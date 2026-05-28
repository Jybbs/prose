"""
A chained compare anchors on its first operator. The remaining
operators stay in place and don't participate in the column math.
"""

if (
    foo == 1
    and 0 < bar < 100
    and qux == 3
):
    pass
