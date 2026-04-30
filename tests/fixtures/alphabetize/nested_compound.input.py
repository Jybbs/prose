"""
Nested compound statements recurse to arbitrary depth. The outer
arm's body and the inner arm's body each sort independently, and
the inner compound stays in its source slot within the outer
arm while its own body's contents reorder.
"""

if outer:
    if inner:
        from zeta import a
        from alpha import b
    from gamma import c
    from beta import d
