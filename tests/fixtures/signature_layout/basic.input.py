"""
A five-parameter signature whose inline form fits the line budget
pins the count trigger firing alone. The rule expands solely on the
parameter count exceeding `max-inline-params`.
"""


def render(target: int, palette: str, layout: tuple[int, int], spread: float, verbose: bool):
    return (target, palette, layout, spread, verbose)
