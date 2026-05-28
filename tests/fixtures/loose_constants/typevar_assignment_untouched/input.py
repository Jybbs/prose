"""
A `TypeVar("T")` assignment is SCREAMING_CASE by Python convention
and structural by definition, so it passes through the rule
untouched.
"""

from typing import TypeVar

T = TypeVar("T")
