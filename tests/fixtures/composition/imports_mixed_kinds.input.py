"""
Module top with a mixed run of bare imports and from-imports, each
kind out of alphabetical order. alphabetize sorts each kind in place,
align_imports aligns the `as` keyword across the aliased lines, and
blank_lines inserts one blank line at the bare-to-from boundary.

Rules:
- alphabetize
- align_imports
- blank_lines
"""

import requests as req
import numpy as np
import collections as col
from typing import Any
from os import path
from collections import Counter
