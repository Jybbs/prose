"""
A three-parameter signature whose inline form overflows the default
`code-line-length` of 88 columns pins the length trigger firing
alone, with the parameter count comfortably under the cap.
"""


def render(left_descriptor: LayoutLayer, right_descriptor: LayoutLayer, palette_descriptor: PaletteSpec):
    return (left_descriptor, right_descriptor, palette_descriptor)
