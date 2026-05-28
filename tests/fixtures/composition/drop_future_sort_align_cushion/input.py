"""
Module top with `from __future__ import annotations`, a run of
aliased imports out of order, and irregular blank-line spacing
before the first function. The full rule subset removes the unused
future, sorts the aliased imports, aligns the `as` column, and
normalizes the blank-line cushion.

Rules:
- alphabetize
- unused_future_annotations
- blank_lines
- align_imports
"""

from __future__ import annotations

import requests as req
import numpy as np
import collections as col



def add(a, b):
    return a + b
