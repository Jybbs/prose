"""
The rule emits no edits, so every pass leaves the source byte-for-byte
identical. A mix of legacy and modern forms stays as written.
"""

from typing import Optional, Union

legacy_optional: Optional[int] = None
legacy_union: Union[int, str]  = 0
modern: int | None = None
