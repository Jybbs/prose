"""
A block with no bare imports. alphabetize keeps external `from`
imports ahead of the local-package group, blank_lines separates the
two with one blank line and adds no leading bare group, and
align_imports aligns within each group.

Rules:
- alphabetize
- align_imports
- blank_lines
"""

from . import shared
from collections import Counter
from myapp import app
from typing import Any
