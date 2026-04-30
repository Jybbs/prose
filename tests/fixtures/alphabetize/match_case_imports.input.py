"""
Each `match` case arm sorts its body independently. The arms do
not share a sort key space, so identical import names in two arms
do not migrate across the case boundary.
"""

match value:
    case 1:
        from zeta import a
        from alpha import b
    case 2:
        from gamma import c
        from beta import d
    case _:
        pass
