"""
A module opening with `from __future__ import annotations` followed
by a run of aliased imports, where no annotation in the module needs
the future import.

Rules:
- unused_future_annotations
- align_imports
"""

from __future__ import annotations

import numpy as np
import collections as col
import functools as ft


def add(a, b):
    return a + b
