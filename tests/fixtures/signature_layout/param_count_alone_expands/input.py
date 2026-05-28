"""
A five-parameter signature whose inline form fits the line budget
pins the count trigger firing alone. The rule expands solely on the
parameter count exceeding `max-inline-params`.
"""


def render(layout: tuple[int, int], palette: str, spread: float, target: int, verbose: bool):
    return (layout, palette, spread, target, verbose)
