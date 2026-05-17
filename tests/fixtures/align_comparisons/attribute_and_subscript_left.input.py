"""
Operands whose left sides are attribute access or subscript
expressions still qualify for the alignment group, alongside plain
name references.
"""

if (
    foo.attr == 1
    and bar[0] == 2
    and qux == 3
):
    pass
