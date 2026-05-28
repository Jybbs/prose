"""
Nested function defs each carry their own docstring, and the
rewrite fires at every level independently.
"""


def outer():
    """Outer docstring."""

    def inner():
        """Inner docstring."""
        return 1

    return inner()
