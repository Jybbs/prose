from __future__ import annotations


def collect(errors: ExceptionGroup) -> int:
    return len(errors.exceptions)
