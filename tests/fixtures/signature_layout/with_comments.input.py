"""
A comment anywhere between `(` and `)` pins the existing shape,
overriding both count and length triggers. Each pinned signature
keeps its comment in place rather than getting rewritten around.
Both own-line and trailing-line comment placements anchor.
"""


def own_line_pin(
    target: int,
    # comment between parameters
    palette: str,
    layout: tuple[int, int],
    spread: float,
    verbose: bool,
):
    return (target, palette, layout, spread, verbose)


def trailing_line_pin(
    target: int,
    palette: str,  # trailing on a parameter line
    layout: tuple[int, int],
    spread: float,
    verbose: bool,
):
    return (target, palette, layout, spread, verbose)
