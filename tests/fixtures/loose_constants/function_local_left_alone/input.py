"""
A function-local SCREAMING_CASE assignment is not module-level, so
the rule leaves it alone. The walker stops at `def` and `class`
boundaries by design.
"""

def compute(radius: float) -> float:
    PI = 3.14
    return PI * radius * radius
