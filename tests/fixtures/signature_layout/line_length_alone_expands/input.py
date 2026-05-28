"""
A three-parameter signature whose inline form overflows the default
`code-line-length` of 88 columns pins the length trigger firing
alone, with the parameter count comfortably under the cap.
"""


def render(left_descriptor: LayoutLayer, palette_descriptor: PaletteSpec, right_descriptor: LayoutLayer):
    return (left_descriptor, palette_descriptor, right_descriptor)
