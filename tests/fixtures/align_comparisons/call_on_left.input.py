"""
A call expression on the left side still qualifies. The call's
display width counts as the left-hand side just like any other
expression.
"""

if (
    foo == 1
    and len(b) == 2
    and qux == 3
):
    pass
