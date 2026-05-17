"""
Defaults, `/` and `*` separators, and a return-type annotation
round-trip through the expansion.
"""


def render(target, palette, /, *, layout, spread=None) -> tuple[int, str]:
    return (target, palette, layout, spread)
