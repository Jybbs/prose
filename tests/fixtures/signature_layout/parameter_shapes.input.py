"""
The expansion preserves every parameter-list shape (*defaults, the
posonly-only `/` separator, the bare `*` separator, and a return-type
annotation*) verbatim from source. Pins the slice machinery rendering
each canonical separator at its source position rather than
synthesizing a different shape.
"""


def render(palette: str, target: int, /, *, layout: tuple[int, int], spread: float, verbose: bool = False) -> tuple[int, str]:
    return (palette, target, layout, spread, verbose)
