"""
`==` operands and `<` operands share one aligned group. The
variable-width operators right-align so the second `=` of `==` and
the `<` land in the same column.
"""

if (
    foo == 1
    and bar_baz == 2
    and qux < 100
    and quux < 200
):
    pass
