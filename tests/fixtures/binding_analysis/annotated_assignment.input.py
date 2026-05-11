"""
Annotated assignment records a write at the target offset. The
annotation expression is visited as a read, attributing a read to
each name it references.
"""


Pair = tuple[int, int]
x: int = 0
y: Pair = (1, 2)
