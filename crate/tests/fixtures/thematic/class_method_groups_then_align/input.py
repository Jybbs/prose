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
