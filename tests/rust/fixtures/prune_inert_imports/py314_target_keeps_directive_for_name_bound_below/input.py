from __future__ import annotations

import inspect


def convert(value: Alias) -> None:
    return None


SIGNATURE = inspect.signature(convert)

Alias = int
