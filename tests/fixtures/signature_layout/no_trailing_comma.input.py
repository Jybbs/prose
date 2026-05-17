"""
A multi-line signature whose last parameter lacks a trailing comma
gets the trailing comma added in the canonical expansion. Pins the
rewrite's trailing-comma invariant against source variants that omit
it.
"""


def render(
    layout: tuple[int, int],
    palette: str,
    spread: float,
    target: int,
    verbose: bool
):
    return (layout, palette, spread, target, verbose)
