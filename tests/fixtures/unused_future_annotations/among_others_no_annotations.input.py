"""
The `annotations` alias sits alongside `division` in a multi-name
`__future__` import. Only the `annotations` entry is removed, leaving
the surrounding `division` entry's formatting intact.
"""

from __future__ import annotations, division

x = 1 / 2
