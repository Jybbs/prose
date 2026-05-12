"""
The sole `__future__` import wraps onto multiple lines with parentheses.
The deletion spans every line of the statement, so no parenthesized tail
remains even though `annotations` is the only alias named.
"""

from __future__ import (
    annotations,
)

x = 1
