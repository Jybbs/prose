"""
A module-level annotated assignment in `SCREAMING_CASE` draws the
diagnostic regardless of whether a value follows the annotation.
Both `X: int` and `Y: int = 1` shapes pin the `Stmt::AnnAssign`
code path through the pipeline.
"""

X: int
Y: int = 1
