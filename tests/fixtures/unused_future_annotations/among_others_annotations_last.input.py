"""
The `annotations` alias sits last in a multi-name `__future__` import.
The surgical deletion strips the preceding comma alongside the alias,
leaving the earlier entries' formatting intact.
"""

from __future__ import division, annotations

x = 1 / 2
