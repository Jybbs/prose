from __future__ import annotations

try:
    from foo import Bar
except ImportError:
    pass


def use(value: Bar) -> None:
    pass
