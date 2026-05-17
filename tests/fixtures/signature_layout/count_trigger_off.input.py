"""
A five-parameter signature whose inline form fits the line budget
stays inline when `max-inline-params = false` disables the count
trigger entirely. Pins line-length as the sole expansion gate under
the disabled-count configuration.
"""


def render(layout: int, palette: str, spread: float, target: int, verbose: bool):
    return (layout, palette, spread, target, verbose)
