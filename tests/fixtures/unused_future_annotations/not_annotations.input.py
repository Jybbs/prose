"""
Other `__future__` directives like `division` may still carry semantic
weight on legacy code paths and are out of scope. The rule fires only
on the `annotations` alias.
"""

from __future__ import division

x = 1 / 2
