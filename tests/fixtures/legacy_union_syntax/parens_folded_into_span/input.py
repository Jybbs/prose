"""
Surrounding parentheses around a `Subscript` are folded into the
diagnostic range via `parenthesized_range`, so the surfaced span
matches the user's mental model of "this annotation."
"""

from typing import Optional


def annotate(x: (Optional[int])) -> None: ...
