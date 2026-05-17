"""
A trailing comment on the `def` line after `:` sits outside `(...)`
and so does not pin the signature. The rule still expands when count
or length trips, confirming the pin check covers only comments
strictly between the opening `(` and closing `)`.
"""


def render(target: int, palette: str, layout: tuple[int, int], spread: float, verbose: bool):  # entry point
    return (target, palette, layout, spread, verbose)
