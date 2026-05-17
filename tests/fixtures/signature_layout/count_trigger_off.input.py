"""
A five-parameter signature whose inline form fits the line budget
stays inline when `max-inline-params = false` disables the count
trigger entirely. Pins line-length as the sole expansion gate under
the disabled-count configuration.
"""


def render(target: int, palette: str, layout: int, spread: float, verbose: bool):
    return (target, palette, layout, spread, verbose)
