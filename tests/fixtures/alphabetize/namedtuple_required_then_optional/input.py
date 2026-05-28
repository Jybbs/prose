"""
A `NamedTuple` class body has its annotated field declarations
alphabetized with required-then-optional. Default-bearing fields
land last per Python's positional-default invariant, which the
required-then-optional split already enforces.
"""

from typing import NamedTuple


class Point(NamedTuple):
    z: int = 0
    x: int
    color: str = "black"
    y: int
