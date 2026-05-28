"""
A module opening with `from __future__ import annotations` followed
by extra blank lines before the first non-import statement.

Rules:
- unused_future_annotations
- blank_lines
"""

from __future__ import annotations



def add(a, b):
    return a + b
