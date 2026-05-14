"""
Cross-rule composition on a small class: alphabetize reorders methods
into the canonical dunder / property / private / public groups,
alphabetizes annotated fields, and sorts both bare and from-import
runs at the module top. align_colons and align_equals then settle
the field-declaration columns. The full pipeline running twice on
this input produces no further change.
"""

from typing import Any
from collections import Counter

import sys
import os


class Posting:
    title: str = "Untitled"
    description: str = ""
    company: str = "TBD"
    salary: float = 0.0

    def __repr__(self):
        return f"Posting({self.title})"

    def update(self, *, atomic=True, retries=3):
        return self

    def __init__(self):
        pass

    @property
    def name(self):
        return self.title

    def _internal(self):
        pass
