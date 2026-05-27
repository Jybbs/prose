"""
Module top with a mixed run of bare imports and from-imports, each
kind out of alphabetical order. alphabetize sorts each kind in
its own sub-run, blank_lines inserts one blank line at the
bare-to-from boundary, and align_imports keys on the unified
block so the post-keyword names land at one shared column across
the whole run.

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
