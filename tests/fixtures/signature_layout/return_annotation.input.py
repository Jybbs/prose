"""
A single typed parameter in expanded shape collapses to an inline
signature when the inline form fits under both thresholds. Pins the
return annotation following the closing `)` on the same line as the
inline form, rather than landing on its own line.
"""


def render(
    target: int,
) -> int:
    return target
