"""
Two consecutive `from __future__ import` lines, one carrying
`annotations` and one carrying `division`. Each `ImportFrom` is
processed independently, so the `annotations` line is removed while
the `division` line remains.
"""

from __future__ import annotations
from __future__ import division

x = 1 / 2
