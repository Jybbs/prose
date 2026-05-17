"""
A multi-line signature whose last parameter lacks a trailing comma
gets the trailing comma added in the canonical expansion. Pins the
rewrite's trailing-comma invariant against source variants that omit
it.
"""


def render(
    target: int,
    palette: str,
    layout: tuple[int, int],
    spread: float,
    verbose: bool
):
    return (target, palette, layout, spread, verbose)
