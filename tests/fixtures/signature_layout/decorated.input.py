"""
A `@cache`-decorated function with a five-parameter signature trips
the count trigger and expands. The decorator surface stays untouched
because the rule's replacement range starts at `(`, leaving lines
above the `def` keyword out of scope.
"""

from functools import cache


@cache
def render(target: int, palette: str, layout: tuple[int, int], spread: float, verbose: bool):
    return (target, palette, layout, spread, verbose)
