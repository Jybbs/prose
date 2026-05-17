"""
A comment on its own line between two operands pushes them onto
non-adjacent source lines, so the line-distance check breaks the
run regardless of comment content.
"""

if (
    foo == 1
    # divider
    and bar_baz == 2
    and qux == 3
):
    pass
