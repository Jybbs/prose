"""
The `__future__` import spans multiple lines with a parenthesized
names list. Only the `annotations` entry is removed, and the
surrounding whitespace plus the `division` entry remain intact.
"""

from __future__ import (
    annotations,
    division,
)

x = 1 / 2
