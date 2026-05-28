"""
An `async def` carries its annotation in the same `StmtFunctionDef`
shape as a sync `def`, with `is_async` flipped. Trigger 1 sees the
annotation, trigger 3 evaluates, and with `Result` defined before the
annotation the directive is removed.
"""

from __future__ import annotations


class Result:
    pass


async def fetch() -> Result:
    return Result()
