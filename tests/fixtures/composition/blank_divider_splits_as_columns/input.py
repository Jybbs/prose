"""
Two aliased-import runs separated by extra blank lines. Each run has
its own `as` alignment column because the blank-line divider
partitions alignment groups.

Rules:
- blank_lines
- align_imports
"""

import numpy as np
import collections as col


import functools as ft
import itertools as it
