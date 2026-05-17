"""
A trailing comment after the prev-operand source line breaks the
alignment run on the comment check alone, with line distance still
one.
"""

if (
    foo == 1  # trailing
    and bar_baz == 2
    and qux == 3
):
    pass
