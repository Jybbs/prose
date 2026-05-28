"""
A similarly-shaped import from a different module is untouched. The
rule fires only on `from __future__ import annotations`.
"""

from typing import annotations  # pretend module

x = annotations
