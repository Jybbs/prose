"""
A typing import that names something other than `Optional` or `Union`
is not flagged. The walker resolves `List[int]` to `typing.List` and
falls through the suffix match's catch-all.
"""

from typing import List

values: List[int] = []
