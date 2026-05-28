"""
Module top with bare, external `from`, and local-package imports
scrambled together. alphabetize reorders into the three canonical
groups, blank_lines separates each with one blank line, and
align_imports aligns the `import` keyword inside each group.

Rules:
- alphabetize
- align_imports
- blank_lines
"""

from myapp import app
import sys
from collections import Counter
import os
from . import shared
from typing import Any
import myapp.core
from myapp.db import Session
