def own_line_pin(
    layout: tuple[int, int],
    # comment between parameters
    palette: str,
    spread: float,
    target: int,
    verbose: bool,
):
    return (layout, palette, spread, target, verbose)


def trailing_line_pin(
    layout: tuple[int, int],
    palette: str,  # trailing on a parameter line
    spread: float,
    target: int,
    verbose: bool,
):
    return (layout, palette, spread, target, verbose)
