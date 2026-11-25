from __future__ import annotations

BASE = 1
DERIVED = BASE + 1


def consume():
    return DERIVED


HANDLER = consume
