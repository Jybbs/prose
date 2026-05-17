"""
A `@cache` decorator above the `def` and a four-parameter signature.
"""

from functools import cache


@cache
def render(target, palette, layout, spread):
    return (target, palette, layout, spread)
