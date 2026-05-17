"""
Two signatures already at their canonical shape. An inline form with
two parameters and an expanded form with four.
"""


def already_inline(target, palette):
    return (target, palette)


def already_expanded(
    target,
    palette,
    layout,
    spread,
):
    return (target, palette, layout, spread)
