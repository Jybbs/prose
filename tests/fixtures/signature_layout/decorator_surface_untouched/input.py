"""
A `@cache`-decorated function with a five-parameter signature trips
the count trigger and expands. The decorator surface stays untouched
because the rule's replacement range starts at `(`, leaving lines
above the `def` keyword out of scope.
"""

from functools import cache


@cache
def render(layout: tuple[int, int], palette: str, spread: float, target: int, verbose: bool):
    return (layout, palette, spread, target, verbose)
