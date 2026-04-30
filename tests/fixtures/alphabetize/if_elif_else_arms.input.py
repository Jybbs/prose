"""
Each `if` / `elif` / `else` arm sorts its body independently. The
arms do not share a sort key space, so an import named
`zeta` in one arm and `alpha` in another do not migrate across
the arm boundary.
"""

if cond:
    from zeta import a
    from alpha import b
elif other:
    from gamma import c
    from beta import d
else:
    from omega import e
    from delta import f
