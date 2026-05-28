"""
Annotated assignment with an initializer (`x: int = 1`) participates
in the group. The effective left-hand-side width extends through the
annotation, so the `=` of `beta: int` aligns with the `=` of plain
assignments alongside.
"""

alpha = 1
beta: int = 2
gamma_long = 3
