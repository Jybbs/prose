"""
Three single-comparator `==` operands in a multi-line `and`-chain
share an alignment column. The operator at each row sits one space
past the widest operand's left side.
"""

if (
    foo == 1
    and bar_baz == 2
    and quux == 3
):
    pass
