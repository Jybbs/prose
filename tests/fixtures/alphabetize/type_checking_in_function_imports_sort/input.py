"""
`if TYPE_CHECKING` inside a function body sorts its imports
through the same compound-statement recursion that fires at
module level. The function body itself skips class, method,
and field reorders because Function scope leaves sequential
statements untouched, but import runs sort in every scope.
"""

from typing import TYPE_CHECKING


def use_types():
    if TYPE_CHECKING:
        from zeta import a
        from alpha import b
        from beta import c
    return a, b, c
