"""
An operand whose own range spans multiple source lines breaks the
run.
"""

if (
    foo == (
        1
        + 2
    )
    and bar_baz == 3
    and qux == 4
):
    pass
