"""
The pipeline-level filter applies to every rule, not only
alignment. A function call inside a fmt: off block keeps its
trailing comma while a similar call outside the block has the
comma stripped.
"""

# fmt: off
inside(1, 2, 3,)
# fmt: on

outside(4, 5, 6,)
