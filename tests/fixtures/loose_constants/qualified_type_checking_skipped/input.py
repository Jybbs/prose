"""
The attribute-qualified `typing.TYPE_CHECKING` form skips the
block exactly as the bare `TYPE_CHECKING` name does, mirroring
how `ruff_python_semantic` handles the same two shapes.
"""

import typing

if typing.TYPE_CHECKING:
    DEFAULT_BACKEND = "memory"
