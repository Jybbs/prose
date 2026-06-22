from typing import TYPE_CHECKING


def use_types():
    if TYPE_CHECKING:
        from zeta import a
        from alpha import b
        from beta import c
    return a, b, c
