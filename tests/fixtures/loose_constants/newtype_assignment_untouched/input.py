"""
A `NewType("UserId", int)` assignment is SCREAMING_CASE by Python
convention and structural by definition, so it passes through the
rule untouched.
"""

from typing import NewType

UserId = NewType("UserId", int)
