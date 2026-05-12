"""
A `from __future__ import annotations as <name>` directive still
imports the `annotations` feature regardless of the local binding
name. The rule fires on the directive, deleting the line.
"""

from __future__ import annotations as legacy

x = 1
