"""
An operand whose left side is a call expression breaks the run,
splitting the surrounding qualifying operands into singletons.
"""

if (
    foo == 1
    and len(bar) == 2
    and qux == 3
):
    pass
