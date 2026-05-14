"""
Module-level typed constants whose annotations use `Optional` and
`Union` from `typing`.

Rules:
- align_colons
- align_equals
"""

from typing import Optional, Union

ALPHA: Optional[str] = None
BRAVO: Union[int, str] = 0
CHARLIE_LONG: Optional[int] = 12
