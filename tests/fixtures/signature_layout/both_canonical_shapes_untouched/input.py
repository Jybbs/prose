def already_inline(palette: str, target: int):
    return (palette, target)


def already_expanded(
    layout: tuple[int, int],
    palette: str,
    spread: float,
    target: int,
    verbose: bool
):
    return (layout, palette, spread, target, verbose)
