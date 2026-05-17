"""
A three-parameter signature whose inline form fits the default
`code-line-length` of 88 but overflows a custom budget of 50 expands
under the custom budget. Pins the line-length trigger consuming the
configured threshold rather than a hard-coded one.
"""


def render(left: LayoutLayer, right: LayoutLayer, palette: PaletteSpec):
    return (left, right, palette)
