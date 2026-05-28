"""
A legacy `Optional[X]` nested inside another `Subscript` is flagged
on the inner expression. The parent passed to `parenthesized_range`
is the enclosing expression rather than the enclosing statement.
"""

from typing import Optional

table: dict[str, Optional[int]] = {}
