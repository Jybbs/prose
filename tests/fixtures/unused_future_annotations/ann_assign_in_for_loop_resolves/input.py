"""
An `AnnAssign` inside a `for` loop body still flags the file as
carrying annotations and routes through full binding resolution. The
`Item` reference resolves to the class above the loop, so trigger 3
fires and the directive is removed.
"""

from __future__ import annotations


class Item:
    pass


for x in range(3):
    y: Item = Item()
