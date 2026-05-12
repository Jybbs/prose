"""
The configured `target-version = "3.14"` defers annotations by default
per PEP 749, so the directive is a no-op even with annotations present.
The sole import is removed.
"""

from __future__ import annotations


def add(x: int, y: int) -> int:
    return x + y
