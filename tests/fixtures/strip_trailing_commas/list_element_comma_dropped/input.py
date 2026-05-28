"""
A multi-line list literal drops its trailing comma. The list's
range covers `[...]`, and the backward scan from before the closing
`]` lands on the comma after the last element.
"""

retired = [
    "INST_2014_0007",
    "INST_2018_0033",
    "INST_2021_0044",
]
