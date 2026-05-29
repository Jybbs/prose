def dispatch(palette: str, target: int, *args: int, layout: tuple[int, int], spread: float = 0.0, **kwargs: object):
    return (palette, target, layout, spread, args, kwargs)
