"""
Module top with a mixed run of bare imports and from-imports, each
kind out of alphabetical order. alphabetize sorts each kind in place
and align_imports aligns the `as` keyword across the aliased lines.

Rules:
- alphabetize
- align_imports
"""

import requests as req
import numpy as np
import collections as col
from typing import Any
from os import path
from collections import Counter
