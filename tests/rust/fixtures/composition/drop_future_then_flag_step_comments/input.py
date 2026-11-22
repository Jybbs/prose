from __future__ import annotations


def process(items):
    # 1. Filter empties.
    items = [i for i in items if i]
    # 2. Sort ascending.
    items.sort()
    return items
