"""
Two `==` operands followed by two `<` operands form two independent
groups in the same chain, each aligning at its own widest left side.
"""

if (
    foo == 1
    and bar_baz == 2
    and qux < 100
    and quux_long < 200
):
    pass
