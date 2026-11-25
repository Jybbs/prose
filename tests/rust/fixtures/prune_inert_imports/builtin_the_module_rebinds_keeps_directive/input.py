from __future__ import annotations


def scale(value: int) -> int:
    return value


try:
    int = long
except NameError:
    pass
